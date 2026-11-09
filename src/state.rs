//! Flat, `Copy` game state. No heap allocations anywhere inside `GameState`, so
//! cloning for tree search is a plain memcpy (~260 bytes).
//!
//! INVARIANT: every array slot at or beyond its `*_len` counter is zeroed
//! (`Minion::EMPTY` / card id 0). All mutators below preserve this, which makes
//! the derived `PartialEq`/`Eq`/`Hash` reflect only the meaningful contents.

use serde::{Deserialize, Serialize};

use crate::cards;
use crate::serializer::{GameJson, MinionJson, PlayerJson};

pub type CardId = u8;

pub const MAX_BOARD: usize = 7;
pub const MAX_HAND: usize = 10;
pub const MAX_DECK: usize = 60;
pub const MAX_MANA: u8 = 10;
pub const STARTING_HEALTH: i16 = 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "MinionJson", try_from = "MinionJson")]
pub struct Minion {
    pub card: CardId,
    pub attack: i8,
    pub health: i8,
    pub max_health: i8,
    /// False while summoning sick or after attacking this turn.
    pub can_attack: bool,
}

impl Minion {
    pub const EMPTY: Minion =
        Minion { card: 0, attack: 0, health: 0, max_health: 0, can_attack: false };

    /// A freshly summoned (summoning-sick) minion with its printed stats.
    #[inline]
    pub fn summon(card: CardId) -> Minion {
        let d = cards::card(card);
        Minion { card, attack: d.attack, health: d.health, max_health: d.health, can_attack: false }
    }

    #[inline]
    pub fn take_damage(&mut self, amount: i8) {
        self.health = self.health.saturating_sub(amount);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "PlayerJson", try_from = "PlayerJson")]
pub struct PlayerState {
    pub hero_health: i16,
    pub hero_max_health: i16,
    /// Hero attack (from weapons in later phases; 0 in vanilla).
    pub hero_attack: i8,
    pub hero_attacked: bool,
    pub mana: u8,
    pub max_mana: u8,
    pub hero_power_used: bool,
    /// Next fatigue hit deals `fatigue + 1`.
    pub fatigue: u8,
    pub board_len: u8,
    pub hand_len: u8,
    pub deck_len: u8,
    /// Left-to-right board order.
    pub minions: [Minion; MAX_BOARD],
    pub hand: [CardId; MAX_HAND],
    /// `deck[deck_len - 1]` is the top of the deck (next draw).
    pub deck: [CardId; MAX_DECK],
}

impl PlayerState {
    pub const EMPTY: PlayerState = PlayerState {
        hero_health: STARTING_HEALTH,
        hero_max_health: STARTING_HEALTH,
        hero_attack: 0,
        hero_attacked: false,
        mana: 0,
        max_mana: 0,
        hero_power_used: false,
        fatigue: 0,
        board_len: 0,
        hand_len: 0,
        deck_len: 0,
        minions: [Minion::EMPTY; MAX_BOARD],
        hand: [0; MAX_HAND],
        deck: [0; MAX_DECK],
    };

    /// New player whose deck is given in draw order (`draw_order[0]` is drawn first).
    pub fn with_deck(draw_order: &[CardId]) -> Self {
        assert!(draw_order.len() <= MAX_DECK, "deck larger than {MAX_DECK}");
        let mut p = Self::EMPTY;
        for (slot, &c) in p.deck.iter_mut().zip(draw_order.iter().rev()) {
            *slot = c;
        }
        p.deck_len = draw_order.len() as u8;
        p
    }

    #[inline(always)]
    pub fn board(&self) -> &[Minion] {
        &self.minions[..self.board_len as usize]
    }

    #[inline(always)]
    pub fn board_mut(&mut self) -> &mut [Minion] {
        &mut self.minions[..self.board_len as usize]
    }

    #[inline(always)]
    pub fn hand_cards(&self) -> &[CardId] {
        &self.hand[..self.hand_len as usize]
    }

    /// Deck contents, bottom first (last element is the next draw).
    #[inline(always)]
    pub fn deck_cards(&self) -> &[CardId] {
        &self.deck[..self.deck_len as usize]
    }

    #[inline(always)]
    pub fn board_full(&self) -> bool {
        self.board_len as usize >= MAX_BOARD
    }

    /// Summons at the far right. Returns false (no-op) if the board is full.
    #[inline]
    pub fn summon(&mut self, m: Minion) -> bool {
        self.summon_at(self.board_len as usize, m)
    }

    /// Inserts at `pos`, shifting minions right. Used for tokens spawned to the
    /// right of their source (`pos = source_idx + 1`).
    #[inline]
    pub fn summon_at(&mut self, pos: usize, m: Minion) -> bool {
        let len = self.board_len as usize;
        if len >= MAX_BOARD || pos > len {
            return false;
        }
        self.minions.copy_within(pos..len, pos + 1);
        self.minions[pos] = m;
        self.board_len += 1;
        true
    }

    /// Removes all minions with health <= 0, preserving order.
    #[inline]
    pub fn remove_dead(&mut self) {
        let len = self.board_len as usize;
        let mut w = 0;
        for r in 0..len {
            let m = self.minions[r];
            if m.health > 0 {
                self.minions[w] = m;
                w += 1;
            }
        }
        for slot in &mut self.minions[w..len] {
            *slot = Minion::EMPTY;
        }
        self.board_len = w as u8;
    }

    /// Order-preserving removal from hand (keeps action indices of the remaining
    /// cards stable relative to each other, which is what makes play-order
    /// transpositions collapse to identical states).
    #[inline]
    pub fn take_from_hand(&mut self, idx: usize) -> CardId {
        let len = self.hand_len as usize;
        debug_assert!(idx < len);
        let c = self.hand[idx];
        self.hand.copy_within(idx + 1..len, idx);
        self.hand[len - 1] = 0;
        self.hand_len -= 1;
        c
    }

    /// Returns false if the hand is full (card is burned).
    #[inline]
    pub fn add_to_hand(&mut self, c: CardId) -> bool {
        if self.hand_len as usize >= MAX_HAND {
            return false;
        }
        self.hand[self.hand_len as usize] = c;
        self.hand_len += 1;
        true
    }

    #[inline]
    pub fn pop_deck(&mut self) -> Option<CardId> {
        if self.deck_len == 0 {
            return None;
        }
        self.deck_len -= 1;
        let i = self.deck_len as usize;
        let c = self.deck[i];
        self.deck[i] = 0;
        Some(c)
    }

    #[inline]
    pub fn damage_hero(&mut self, amount: i16) {
        self.hero_health = self.hero_health.saturating_sub(amount);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    #[default]
    Ongoing,
    Player0Won,
    Player1Won,
    Draw,
}

impl GameStatus {
    #[inline(always)]
    pub fn is_terminal(self) -> bool {
        self != GameStatus::Ongoing
    }

    pub fn winner(self) -> Option<u8> {
        match self {
            GameStatus::Player0Won => Some(0),
            GameStatus::Player1Won => Some(1),
            _ => None,
        }
    }

    /// Status implied by hero health totals.
    #[inline]
    pub fn from_health(h0: i16, h1: i16) -> GameStatus {
        match (h0 <= 0, h1 <= 0) {
            (false, false) => GameStatus::Ongoing,
            (true, true) => GameStatus::Draw,
            (true, false) => GameStatus::Player1Won,
            (false, true) => GameStatus::Player0Won,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "GameJson", try_from = "GameJson")]
pub struct GameState {
    pub players: [PlayerState; 2],
    /// Active player (0 or 1).
    pub current: u8,
    /// Incremented at the start of every player-turn (turn 1 = player 0's first turn).
    pub turn: u16,
    pub status: GameStatus,
}

impl GameState {
    #[inline(always)]
    pub fn me(&self) -> &PlayerState {
        &self.players[self.current as usize]
    }

    #[inline(always)]
    pub fn opp(&self) -> &PlayerState {
        &self.players[(self.current ^ 1) as usize]
    }

    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    #[inline]
    pub fn update_status(&mut self) {
        self.status =
            GameStatus::from_health(self.players[0].hero_health, self.players[1].hero_health);
    }
}

// Compile-time guards on the zero-allocation, cache-friendly layout.
const _: () = assert!(std::mem::size_of::<GameState>() <= 320);
const fn assert_copy<T: Copy>() {}
const _: () = assert_copy::<GameState>();
