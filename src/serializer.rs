//! Sandbox JSON format for loading arbitrary board/hand/mana/health states.
//!
//! `GameState`, `PlayerState` and `Minion` derive `Serialize`/`Deserialize`
//! through the JSON-friendly mirror types below (`#[serde(into, try_from)]`),
//! so the hot structs stay flat/fixed-size while the JSON stays readable and
//! hand-editable. Almost every field is optional:
//!
//! ```json
//! {
//!   "current_player": 0,
//!   "players": [
//!     { "hero_health": 12, "max_mana": 5,
//!       "board": [ { "card": "Chillwind Yeti", "health": 3 } ],
//!       "hand":  [ "Boulderfist Ogre", 1 ],
//!       "deck":  [ "War Golem" ] },
//!     { "board": [ { "card": "Bloodfen Raptor", "can_attack": false } ] }
//!   ]
//! }
//! ```
//!
//! * Cards may be referenced by name (case-insensitive) or numeric id.
//! * `deck` is listed in draw order: the first element is drawn next.
//! * Minion `attack`/`health`/`max_health` default to the card's printed stats;
//!   `can_attack` defaults to `true`.
//! * `mana` defaults to `max_mana`; `hero_health` to 30.
//! * `status` is derived from hero health; if supplied it must agree.

use std::error::Error;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cards;
use crate::state::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CardRef {
    Id(u8),
    Name(String),
}

impl CardRef {
    pub fn resolve(&self) -> Result<CardId, String> {
        match self {
            CardRef::Id(id) => cards::card_checked(*id)
                .map(|c| c.id)
                .ok_or_else(|| format!("unknown card id {id}")),
            CardRef::Name(n) => cards::find_by_name(n)
                .map(|c| c.id)
                .ok_or_else(|| format!("unknown card name {n:?}")),
        }
    }

    fn named(id: CardId) -> CardRef {
        CardRef::Name(cards::card(id).name.to_string())
    }
}

fn default_true() -> bool {
    true
}
fn default_health() -> i16 {
    STARTING_HEALTH
}
fn default_turn() -> u16 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinionJson {
    pub card: CardRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_health: Option<i8>,
    #[serde(default = "default_true")]
    pub can_attack: bool,
}

impl From<Minion> for MinionJson {
    fn from(m: Minion) -> Self {
        MinionJson {
            card: CardRef::named(m.card),
            attack: Some(m.attack),
            health: Some(m.health),
            max_health: Some(m.max_health),
            can_attack: m.can_attack,
        }
    }
}

impl TryFrom<MinionJson> for Minion {
    type Error = String;
    fn try_from(j: MinionJson) -> Result<Self, String> {
        let card = j.card.resolve()?;
        let def = cards::card(card);
        let attack = j.attack.unwrap_or(def.attack);
        let health = j.health.unwrap_or(def.health);
        let max_health = j.max_health.unwrap_or(health.max(def.health));
        if attack < 0 {
            return Err(format!("{}: attack must be >= 0", def.name));
        }
        if health <= 0 {
            return Err(format!("{}: health must be > 0 for a minion on the board", def.name));
        }
        if max_health < health {
            return Err(format!("{}: max_health < health", def.name));
        }
        Ok(Minion { card, attack, health, max_health, can_attack: j.can_attack })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerJson {
    #[serde(default = "default_health")]
    pub hero_health: i16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero_max_health: Option<i16>,
    #[serde(default)]
    pub hero_attack: i8,
    #[serde(default)]
    pub hero_attacked: bool,
    #[serde(default)]
    pub max_mana: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana: Option<u8>,
    #[serde(default)]
    pub hero_power_used: bool,
    #[serde(default)]
    pub fatigue: u8,
    #[serde(default)]
    pub board: Vec<Minion>,
    #[serde(default)]
    pub hand: Vec<CardRef>,
    /// Draw order: `deck[0]` is the next card drawn.
    #[serde(default)]
    pub deck: Vec<CardRef>,
}

impl From<PlayerState> for PlayerJson {
    fn from(p: PlayerState) -> Self {
        PlayerJson {
            hero_health: p.hero_health,
            hero_max_health: Some(p.hero_max_health),
            hero_attack: p.hero_attack,
            hero_attacked: p.hero_attacked,
            max_mana: p.max_mana,
            mana: Some(p.mana),
            hero_power_used: p.hero_power_used,
            fatigue: p.fatigue,
            board: p.board().to_vec(),
            hand: p.hand_cards().iter().map(|&c| CardRef::named(c)).collect(),
            deck: p.deck_cards().iter().rev().map(|&c| CardRef::named(c)).collect(),
        }
    }
}

impl TryFrom<PlayerJson> for PlayerState {
    type Error = String;
    fn try_from(j: PlayerJson) -> Result<Self, String> {
        if j.board.len() > MAX_BOARD {
            return Err(format!("board has {} minions (max {MAX_BOARD})", j.board.len()));
        }
        if j.hand.len() > MAX_HAND {
            return Err(format!("hand has {} cards (max {MAX_HAND})", j.hand.len()));
        }
        if j.deck.len() > MAX_DECK {
            return Err(format!("deck has {} cards (max {MAX_DECK})", j.deck.len()));
        }
        if j.max_mana > MAX_MANA {
            return Err(format!("max_mana {} exceeds {MAX_MANA}", j.max_mana));
        }
        if j.hero_attack < 0 {
            return Err("hero_attack must be >= 0".into());
        }
        let hero_max_health = j.hero_max_health.unwrap_or(j.hero_health.max(STARTING_HEALTH));
        if hero_max_health <= 0 {
            return Err("hero_max_health must be > 0".into());
        }

        let mut p = PlayerState::EMPTY;
        p.hero_health = j.hero_health;
        p.hero_max_health = hero_max_health;
        p.hero_attack = j.hero_attack;
        p.hero_attacked = j.hero_attacked;
        p.max_mana = j.max_mana;
        p.mana = j.mana.unwrap_or(j.max_mana);
        p.hero_power_used = j.hero_power_used;
        p.fatigue = j.fatigue;

        for (slot, m) in p.minions.iter_mut().zip(&j.board) {
            *slot = *m;
        }
        p.board_len = j.board.len() as u8;
        for (slot, c) in p.hand.iter_mut().zip(&j.hand) {
            *slot = c.resolve()?;
        }
        p.hand_len = j.hand.len() as u8;
        // Internal deck stores the top (next draw) at the end.
        for (slot, c) in p.deck.iter_mut().zip(j.deck.iter().rev()) {
            *slot = c.resolve()?;
        }
        p.deck_len = j.deck.len() as u8;
        Ok(p)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameJson {
    #[serde(default)]
    pub current_player: u8,
    #[serde(default = "default_turn")]
    pub turn: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<GameStatus>,
    pub players: [PlayerState; 2],
}

impl From<GameState> for GameJson {
    fn from(s: GameState) -> Self {
        GameJson {
            current_player: s.current,
            turn: s.turn,
            status: Some(s.status),
            players: s.players,
        }
    }
}

impl TryFrom<GameJson> for GameState {
    type Error = String;
    fn try_from(j: GameJson) -> Result<Self, String> {
        if j.current_player > 1 {
            return Err(format!("current_player must be 0 or 1, got {}", j.current_player));
        }
        let mut s = GameState {
            players: j.players,
            current: j.current_player,
            turn: j.turn,
            status: GameStatus::Ongoing,
        };
        s.update_status();
        if let Some(given) = j.status {
            if given != s.status {
                return Err(format!(
                    "status {given:?} inconsistent with hero health (implies {:?})",
                    s.status
                ));
            }
        }
        Ok(s)
    }
}

pub fn to_json(state: &GameState) -> String {
    serde_json::to_string_pretty(state).expect("GameState serialization cannot fail")
}

pub fn to_json_compact(state: &GameState) -> String {
    serde_json::to_string(state).expect("GameState serialization cannot fail")
}

pub fn from_json(s: &str) -> Result<GameState, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<GameState, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(from_json(&text)?)
}

pub fn to_json_file<P: AsRef<Path>>(state: &GameState, path: P) -> std::io::Result<()> {
    std::fs::write(path, to_json(state))
}
