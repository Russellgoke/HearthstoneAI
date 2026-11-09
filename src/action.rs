//! Discrete, fixed action space. Every atomic micro-action maps to a stable
//! integer index in `0..MAX_ACTIONS` for RL action masking.
//!
//! | Range   | Count | Action                                                        |
//! |---------|-------|---------------------------------------------------------------|
//! | 0..10   | 10    | `PlayMinion { hand_idx }`                                     |
//! | 10..74  | 64    | `Attack { attacker_idx, target_idx }` = 10 + attacker*8 + target |
//! | 74..90  | 16    | `HeroPower { target_idx }`                                    |
//! | 90      | 1     | `EndTurn`                                                     |
//!
//! Attack indices: attacker 0..7 = friendly minion slot, 7 = own hero;
//! target 0..7 = enemy minion slot, 7 = enemy hero.
//!
//! Hero power targets: 0..7 enemy minions, 7 enemy hero, 8..15 friendly minions
//! (8 + slot), 15 own hero.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::state::{MAX_BOARD, MAX_HAND};

/// Attacker / target index meaning "the hero" in `Attack`.
pub const HERO_SLOT: u8 = MAX_BOARD as u8; // 7

pub const HP_ENEMY_HERO: u8 = 7;
pub const HP_FRIENDLY_BASE: u8 = 8;
pub const HP_FRIENDLY_HERO: u8 = 15;
pub const NUM_HP_TARGETS: usize = 16;

const ATTACK_SIDE: usize = MAX_BOARD + 1; // 8

pub const PLAY_BASE: usize = 0;
pub const ATTACK_BASE: usize = PLAY_BASE + MAX_HAND; // 10
pub const HERO_POWER_BASE: usize = ATTACK_BASE + ATTACK_SIDE * ATTACK_SIDE; // 74
pub const END_TURN_INDEX: usize = HERO_POWER_BASE + NUM_HP_TARGETS; // 90
pub const MAX_ACTIONS: usize = END_TURN_INDEX + 1; // 91

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Minion always spawns at the far right of the board.
    PlayMinion { hand_idx: u8 },
    Attack { attacker_idx: u8, target_idx: u8 },
    HeroPower { target_idx: u8 },
    EndTurn,
}

impl Action {
    #[inline(always)]
    pub const fn to_index(self) -> usize {
        match self {
            Action::PlayMinion { hand_idx } => PLAY_BASE + hand_idx as usize,
            Action::Attack { attacker_idx, target_idx } => {
                ATTACK_BASE + attacker_idx as usize * ATTACK_SIDE + target_idx as usize
            }
            Action::HeroPower { target_idx } => HERO_POWER_BASE + target_idx as usize,
            Action::EndTurn => END_TURN_INDEX,
        }
    }

    #[inline]
    pub const fn from_index(i: usize) -> Option<Action> {
        if i < ATTACK_BASE {
            Some(Action::PlayMinion { hand_idx: (i - PLAY_BASE) as u8 })
        } else if i < HERO_POWER_BASE {
            let k = i - ATTACK_BASE;
            Some(Action::Attack {
                attacker_idx: (k / ATTACK_SIDE) as u8,
                target_idx: (k % ATTACK_SIDE) as u8,
            })
        } else if i < END_TURN_INDEX {
            Some(Action::HeroPower { target_idx: (i - HERO_POWER_BASE) as u8 })
        } else if i == END_TURN_INDEX {
            Some(Action::EndTurn)
        } else {
            None
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn side(i: u8, who: &str) -> String {
            if i == HERO_SLOT { format!("{who} hero") } else { format!("{who} minion {i}") }
        }
        match *self {
            Action::PlayMinion { hand_idx } => write!(f, "Play hand card {hand_idx}"),
            Action::Attack { attacker_idx, target_idx } => write!(
                f,
                "Attack: {} -> {}",
                side(attacker_idx, "friendly"),
                side(target_idx, "enemy")
            ),
            Action::HeroPower { target_idx } => {
                let t = match target_idx {
                    0..=6 => format!("enemy minion {target_idx}"),
                    HP_ENEMY_HERO => "enemy hero".to_string(),
                    HP_FRIENDLY_HERO => "own hero".to_string(),
                    _ => format!("friendly minion {}", target_idx - HP_FRIENDLY_BASE),
                };
                write!(f, "Hero power -> {t}")
            }
            Action::EndTurn => write!(f, "End turn"),
        }
    }
}

/// Stack-allocated list of actions (no heap allocation in rollouts).
/// Capacity `MAX_ACTIONS` is a hard upper bound on simultaneously legal actions.
#[derive(Clone, Copy)]
pub struct ActionList {
    len: usize,
    actions: [Action; MAX_ACTIONS],
}

impl ActionList {
    #[inline]
    pub const fn new() -> Self {
        ActionList { len: 0, actions: [Action::EndTurn; MAX_ACTIONS] }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    pub fn push(&mut self, a: Action) {
        debug_assert!(self.len < MAX_ACTIONS);
        self.actions[self.len] = a;
        self.len += 1;
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[Action] {
        &self.actions[..self.len]
    }
}

impl Default for ActionList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for ActionList {
    type Target = [Action];
    fn deref(&self) -> &[Action] {
        self.as_slice()
    }
}
