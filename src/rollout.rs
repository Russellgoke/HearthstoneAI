//! Uniform-random rollout policy (baseline for MCTS playouts and benchmarking).

use rand::Rng;

use crate::action::ActionList;
use crate::state::{GameState, GameStatus};

/// Plays uniformly random legal actions until the game ends. Returns the final
/// status and the number of actions applied. Allocation-free.
///
/// Terminates: every turn draws a card, so once decks run out fatigue damage
/// grows each turn until a hero dies.
#[inline]
pub fn random_rollout<R: Rng + ?Sized>(state: &mut GameState, rng: &mut R) -> (GameStatus, u32) {
    let mut actions = ActionList::new();
    let mut steps = 0u32;
    while !state.is_terminal() {
        state.legal_actions_into(&mut actions);
        let a = actions.as_slice()[rng.random_range(0..actions.len())];
        state.apply_action(a);
        steps += 1;
    }
    (state.status, steps)
}
