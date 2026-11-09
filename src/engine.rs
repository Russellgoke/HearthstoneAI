//! Rules: turn structure, legal-action generation, and atomic action transitions.
//!
//! The engine itself is fully deterministic; the only randomness (deck order)
//! is supplied by the caller when constructing a game.

use rand::Rng;

use crate::action::*;
use crate::cards;
use crate::state::*;

/// Vanilla "Fireblast"-style hero power: 2 mana, deal 1 damage to any target.
pub const HERO_POWER_COST: u8 = 2;
pub const HERO_POWER_DAMAGE: i8 = 1;
pub const STARTING_HAND_FIRST: usize = 3;
pub const STARTING_HAND_SECOND: usize = 4;
pub const DEFAULT_DECK_SIZE: usize = 30;

impl GameState {
    /// Starts a game from two decks given in draw order (shuffle beforehand).
    /// Player 0 draws 3, player 1 draws 4, then player 0's first turn begins
    /// (1 mana crystal, 1 card drawn).
    pub fn new_game(deck0: &[CardId], deck1: &[CardId]) -> Self {
        let mut s = GameState {
            players: [PlayerState::with_deck(deck0), PlayerState::with_deck(deck1)],
            current: 0,
            turn: 0,
            status: GameStatus::Ongoing,
        };
        for _ in 0..STARTING_HAND_FIRST {
            s.draw_card(0);
        }
        for _ in 0..STARTING_HAND_SECOND {
            s.draw_card(1);
        }
        s.start_turn();
        s
    }

    /// Game with two uniformly random decks of `deck_size` cards.
    pub fn new_random_game<R: Rng + ?Sized>(rng: &mut R, deck_size: usize) -> Self {
        assert!(deck_size <= MAX_DECK);
        let mut d0 = [0 as CardId; MAX_DECK];
        let mut d1 = [0 as CardId; MAX_DECK];
        cards::fill_random_deck(rng, &mut d0[..deck_size]);
        cards::fill_random_deck(rng, &mut d1[..deck_size]);
        Self::new_game(&d0[..deck_size], &d1[..deck_size])
    }

    /// Draws one card for player `p`: burn if hand full, fatigue if deck empty.
    #[inline]
    pub fn draw_card(&mut self, p: usize) {
        let pl = &mut self.players[p];
        match pl.pop_deck() {
            Some(c) => {
                pl.add_to_hand(c); // false => burned
            }
            None => {
                pl.fatigue = pl.fatigue.saturating_add(1);
                let dmg = pl.fatigue as i16;
                pl.damage_hero(dmg);
            }
        }
    }

    /// Start-of-turn bookkeeping for `self.current`.
    fn start_turn(&mut self) {
        self.turn = self.turn.saturating_add(1);
        let p = self.current as usize;
        {
            let pl = &mut self.players[p];
            if pl.max_mana < MAX_MANA {
                pl.max_mana += 1;
            }
            pl.mana = pl.max_mana;
            pl.hero_power_used = false;
            pl.hero_attacked = false;
            for m in pl.board_mut() {
                m.can_attack = true; // clears summoning sickness
            }
        }
        self.draw_card(p);
        self.update_status();
    }

    /// Executes one atomic micro-action. The active player only changes on
    /// `EndTurn`. Illegal actions are a logic error (checked in debug builds);
    /// use [`GameState::try_apply_action`] for untrusted input.
    #[inline]
    pub fn apply_action(&mut self, action: Action) {
        debug_assert!(self.is_legal(action), "illegal action {action:?}");
        if self.status.is_terminal() {
            return;
        }
        let me = self.current as usize;
        let op = me ^ 1;
        match action {
            Action::PlayMinion { hand_idx } => {
                let pl = &mut self.players[me];
                let card = pl.take_from_hand(hand_idx as usize);
                pl.mana -= cards::card(card).cost;
                pl.summon(Minion::summon(card));
            }
            Action::Attack { attacker_idx, target_idx } => {
                let atk = if attacker_idx == HERO_SLOT {
                    self.players[me].hero_attack
                } else {
                    self.players[me].minions[attacker_idx as usize].attack
                };
                // Heroes deal no counter-damage when defending.
                let counter = if target_idx == HERO_SLOT {
                    self.players[op].damage_hero(atk as i16);
                    0
                } else {
                    let t = &mut self.players[op].minions[target_idx as usize];
                    t.take_damage(atk);
                    t.attack
                };
                let mine = &mut self.players[me];
                if attacker_idx == HERO_SLOT {
                    mine.hero_attacked = true;
                    mine.damage_hero(counter as i16);
                } else {
                    let a = &mut mine.minions[attacker_idx as usize];
                    a.can_attack = false;
                    a.take_damage(counter);
                }
                self.players[me].remove_dead();
                self.players[op].remove_dead();
                self.update_status();
            }
            Action::HeroPower { target_idx } => {
                {
                    let pl = &mut self.players[me];
                    pl.mana -= HERO_POWER_COST;
                    pl.hero_power_used = true;
                }
                match target_idx {
                    HP_ENEMY_HERO => self.players[op].damage_hero(HERO_POWER_DAMAGE as i16),
                    HP_FRIENDLY_HERO => self.players[me].damage_hero(HERO_POWER_DAMAGE as i16),
                    t if t < HP_ENEMY_HERO => {
                        self.players[op].minions[t as usize].take_damage(HERO_POWER_DAMAGE);
                        self.players[op].remove_dead();
                    }
                    t => {
                        let i = (t - HP_FRIENDLY_BASE) as usize;
                        self.players[me].minions[i].take_damage(HERO_POWER_DAMAGE);
                        self.players[me].remove_dead();
                    }
                }
                self.update_status();
            }
            Action::EndTurn => {
                self.current ^= 1;
                self.start_turn();
            }
        }
    }

    /// Validated variant of `apply_action`.
    pub fn try_apply_action(&mut self, action: Action) -> Result<(), String> {
        if !self.is_legal(action) {
            return Err(format!("illegal action: {action} ({action:?})"));
        }
        self.apply_action(action);
        Ok(())
    }

    /// Single-action legality check (mirrors `for_each_legal`).
    pub fn is_legal(&self, action: Action) -> bool {
        if self.status.is_terminal() {
            return false;
        }
        let me = self.me();
        let opp = self.opp();
        match action {
            Action::PlayMinion { hand_idx } => {
                let h = hand_idx as usize;
                h < me.hand_len as usize
                    && !me.board_full()
                    && cards::card(me.hand[h]).cost <= me.mana
            }
            Action::Attack { attacker_idx, target_idx } => {
                let target_ok =
                    target_idx == HERO_SLOT || target_idx < opp.board_len;
                let attacker_ok = if attacker_idx == HERO_SLOT {
                    me.hero_attack > 0 && !me.hero_attacked
                } else if attacker_idx < me.board_len {
                    let m = &me.minions[attacker_idx as usize];
                    m.can_attack && m.attack > 0
                } else {
                    false
                };
                target_ok && attacker_ok
            }
            Action::HeroPower { target_idx } => {
                !me.hero_power_used
                    && me.mana >= HERO_POWER_COST
                    && match target_idx {
                        t if t < HP_ENEMY_HERO => t < opp.board_len,
                        HP_ENEMY_HERO | HP_FRIENDLY_HERO => true,
                        t if t < HP_FRIENDLY_HERO => t - HP_FRIENDLY_BASE < me.board_len,
                        _ => false,
                    }
            }
            Action::EndTurn => true,
        }
    }

    /// Calls `f` for every legal action, in ascending index order.
    #[inline]
    pub fn for_each_legal<F: FnMut(Action)>(&self, mut f: F) {
        if self.status.is_terminal() {
            return;
        }
        let me = self.me();
        let opp = self.opp();

        if !me.board_full() {
            for (i, &c) in me.hand_cards().iter().enumerate() {
                if cards::card(c).cost <= me.mana {
                    f(Action::PlayMinion { hand_idx: i as u8 });
                }
            }
        }

        let opp_len = opp.board_len;
        for (i, m) in me.board().iter().enumerate() {
            if m.can_attack && m.attack > 0 {
                for t in 0..opp_len {
                    f(Action::Attack { attacker_idx: i as u8, target_idx: t });
                }
                f(Action::Attack { attacker_idx: i as u8, target_idx: HERO_SLOT });
            }
        }
        if me.hero_attack > 0 && !me.hero_attacked {
            for t in 0..opp_len {
                f(Action::Attack { attacker_idx: HERO_SLOT, target_idx: t });
            }
            f(Action::Attack { attacker_idx: HERO_SLOT, target_idx: HERO_SLOT });
        }

        if !me.hero_power_used && me.mana >= HERO_POWER_COST {
            for t in 0..opp_len {
                f(Action::HeroPower { target_idx: t });
            }
            f(Action::HeroPower { target_idx: HP_ENEMY_HERO });
            for t in 0..me.board_len {
                f(Action::HeroPower { target_idx: HP_FRIENDLY_BASE + t });
            }
            f(Action::HeroPower { target_idx: HP_FRIENDLY_HERO });
        }

        f(Action::EndTurn);
    }

    /// Allocation-free legal action generation (use this in rollouts).
    #[inline]
    pub fn legal_actions_into(&self, out: &mut ActionList) {
        out.clear();
        self.for_each_legal(|a| out.push(a));
    }

    pub fn get_legal_actions(&self) -> Vec<Action> {
        let mut v = Vec::with_capacity(32);
        self.for_each_legal(|a| v.push(a));
        v
    }

    pub fn get_action_mask(&self) -> [bool; MAX_ACTIONS] {
        let mut mask = [false; MAX_ACTIONS];
        self.for_each_legal(|a| mask[a.to_index()] = true);
        mask
    }
}
