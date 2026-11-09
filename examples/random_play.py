"""Smoke test / usage example for the hsengine Python bindings.

    maturin develop --release   # or: pip install target/wheels/hsengine-*.whl
    python examples/random_play.py
"""
import json
import random
import time
from pathlib import Path

import hsengine

SANDBOX = Path(__file__).resolve().parent.parent / "sandbox" / "lethal_puzzle.json"


def play_random_game(seed: int) -> hsengine.GameState:
    rng = random.Random(seed)
    s = hsengine.GameState(seed=seed)
    while not s.is_terminal:
        legal = s.get_legal_actions()
        mask = s.get_action_mask()
        assert len(mask) == hsengine.MAX_ACTIONS
        assert [i for i, ok in enumerate(mask) if ok] == legal
        s.step(rng.choice(legal))
    return s


def main() -> None:
    s = play_random_game(0)
    print("finished:", s, "winner =", s.winner)

    # JSON round trip
    restored = hsengine.GameState.from_json(s.to_json())
    assert restored == s and restored.to_json() == s.to_json()

    # Sandbox puzzle: find lethal by brute-force over action sequences.
    puzzle = hsengine.GameState.from_json(SANDBOX.read_text())
    print("puzzle:", puzzle)
    for a in puzzle.get_legal_actions():
        print(f"  {a:3d}  {hsengine.action_to_string(a)}")

    def find_lethal(state, depth=0):
        if state.winner == state.current_player:
            return []
        if depth >= 6:
            return None
        for a in state.get_legal_actions():
            if a == 90:  # EndTurn
                continue
            child = state.copy()
            child.step(a)
            if child.is_terminal and child.winner != puzzle.current_player:
                continue
            line = find_lethal(child, depth + 1)
            if line is not None:
                return [a] + line
        return None

    line = find_lethal(puzzle)
    print("lethal line:", [hsengine.action_to_string(a) for a in line])

    # Illegal actions raise.
    try:
        puzzle.step(0)  # Boulderfist Ogre costs 6, we have 5
    except ValueError as e:
        print("illegal step rejected:", e)

    # Rust-side rollouts (no Python per-step overhead).
    t = time.perf_counter()
    n = 20_000
    wins = sum(1 for i in range(n) if puzzle.random_rollout(seed=i) == 0)
    dt = time.perf_counter() - t
    print(f"{n} rollouts from puzzle in {dt:.3f}s ({n / dt:,.0f}/s), P0 win rate {wins / n:.2%}")

    print("observable hash (p0):", hex(puzzle.observable_hash(0)))
    print(json.loads(puzzle.to_json(pretty=False))["players"][0]["board"])


if __name__ == "__main__":
    main()
