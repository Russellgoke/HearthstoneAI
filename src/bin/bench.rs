//! Rollout throughput benchmark.
//!
//! cargo run --release --bin bench -- [games=10000] [seed=42]

use std::hint::black_box;
use std::time::Instant;

use hsengine::engine::DEFAULT_DECK_SIZE;
use hsengine::{random_rollout, GameState, GameStatus};
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(42);

    println!("GameState size: {} bytes (Copy, no heap)", std::mem::size_of::<GameState>());

    // Warm-up.
    let mut rng = SmallRng::seed_from_u64(seed ^ 0xDEAD_BEEF);
    for _ in 0..1_000 {
        let mut s = GameState::new_random_game(&mut rng, DEFAULT_DECK_SIZE);
        black_box(random_rollout(&mut s, &mut rng));
    }

    // 1) Full games from scratch (deck generation + deal + random play).
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut wins = [0u64; 3];
    let mut total_actions = 0u64;
    let mut total_turns = 0u64;
    let start = Instant::now();
    for _ in 0..games {
        let mut s = GameState::new_random_game(&mut rng, DEFAULT_DECK_SIZE);
        let (status, steps) = random_rollout(&mut s, &mut rng);
        total_actions += steps as u64;
        total_turns += s.turn as u64;
        match status {
            GameStatus::Player0Won => wins[0] += 1,
            GameStatus::Player1Won => wins[1] += 1,
            _ => wins[2] += 1,
        }
    }
    let secs = start.elapsed().as_secs_f64();

    println!("--- {games} random legal-action games (single thread) ---");
    println!("elapsed:          {:.3} s", secs);
    println!("games/sec:        {:.0}", games as f64 / secs);
    println!("actions/sec:      {:.0}", total_actions as f64 / secs);
    println!("avg actions/game: {:.1}", total_actions as f64 / games as f64);
    println!("avg turns/game:   {:.1}", total_turns as f64 / games as f64);
    println!("P0 / P1 / draw:   {} / {} / {}", wins[0], wins[1], wins[2]);

    // 2) ISMCTS-style: clone a mid-game root and roll out from the copy.
    let mut rng = SmallRng::seed_from_u64(seed + 1);
    let mut root = GameState::new_random_game(&mut rng, DEFAULT_DECK_SIZE);
    for _ in 0..6 {
        root.apply_action(hsengine::Action::EndTurn);
    }
    let start = Instant::now();
    let mut acc = 0u64;
    for _ in 0..games {
        let mut s = black_box(root); // Copy = memcpy clone
        acc += random_rollout(&mut s, &mut rng).1 as u64;
    }
    let secs = start.elapsed().as_secs_f64();
    black_box(acc);
    println!("--- {games} rollouts from cloned mid-game root ---");
    println!("rollouts/sec:     {:.0}", games as f64 / secs);
    println!("avg actions:      {:.1}", acc as f64 / games as f64);
    println!("target:           50000 games/sec");
}
