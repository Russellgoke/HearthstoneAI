# hsengine

High-performance Hearthstone-style simulator (Rust core, PyO3 bindings) for
Open-Loop ISMCTS and RL.

```
src/
  state.rs       GameState / PlayerState / Minion — flat, Copy, no heap (240 bytes)
  action.rs      Action enum <-> fixed index 0..91, ActionList (stack-allocated)
  engine.rs      turn logic, legal-action generation / masking, apply_action
  cards.rs       static vanilla card table
  hash.rs        observable-state hash for transposition detection
  rollout.rs     uniform-random rollout policy
  serializer.rs  sandbox JSON format (load/save arbitrary states)
  lib.rs         module root + PyO3 bindings (feature "python")
  bin/bench.rs   rollout throughput benchmark
tests/           engine + serializer tests
sandbox/         example JSON states
examples/        Python usage example
```

## Commands

```sh
cargo test
cargo run --release --bin bench -- 10000      # games/sec
maturin develop --release                     # install Python module into active venv
python examples/random_play.py
```

## Action space (`MAX_ACTIONS = 91`)

| Index  | Action |
|--------|--------|
| 0–9    | `PlayMinion { hand_idx }` — always summons at the far right |
| 10–73  | `Attack { attacker, target }` = `10 + attacker*8 + target`; slot 7 = hero |
| 74–89  | `HeroPower { target }`: 0–6 enemy minions, 7 enemy hero, 8–14 friendly minions, 15 own hero |
| 90     | `EndTurn` |

## Rules implemented (Phase 1)

Mana crystals +1/turn (cap 10) and refill, draw per turn, burn on full hand
(10), cumulative fatigue, summoning sickness, 7-minion board limit, minion and
hero combat, 2-mana "deal 1 damage" hero power. Player 0 starts with 3 cards,
player 1 with 4 (no Coin yet).
