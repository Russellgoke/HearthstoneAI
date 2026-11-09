use hsengine::serializer::{from_json, from_json_file, to_json, to_json_compact};
use hsengine::state::*;
use hsengine::GameState;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

#[test]
fn roundtrip_random_midgame_states() {
    let mut rng = SmallRng::seed_from_u64(99);
    let mut list = hsengine::ActionList::new();
    for _ in 0..200 {
        let deck_size = rng.random_range(0..=60);
        let mut s = GameState::new_random_game(&mut rng, deck_size);
        let stop = rng.random_range(0..200);
        for _ in 0..stop {
            if s.is_terminal() {
                break;
            }
            s.legal_actions_into(&mut list);
            s.apply_action(list[rng.random_range(0..list.len())]);
        }
        let pretty = to_json(&s);
        assert_eq!(from_json(&pretty).unwrap(), s);
        let compact = to_json_compact(&s);
        assert_eq!(from_json(&compact).unwrap(), s);
    }
}

#[test]
fn defaults_and_name_or_id_card_refs() {
    let s = from_json(
        r#"{ "players": [
            { "max_mana": 5, "hand": ["chillwind yeti", 8],
              "board": [ { "card": "War Golem", "health": 3 } ],
              "deck": ["Core Hound", "Murloc Raider"] },
            {} ] }"#,
    )
    .unwrap();
    let p = &s.players[0];
    assert_eq!(p.hero_health, 30);
    assert_eq!(p.mana, 5);
    assert_eq!(p.hand_cards(), &[5, 8]);
    assert_eq!(p.minions[0], Minion { card: 9, attack: 7, health: 3, max_health: 7, can_attack: true });
    // deck lists draw order; internally top is at the end.
    assert_eq!(p.deck_cards(), &[0, 10]);
    assert_eq!(s.turn, 1);
    assert_eq!(s.status, GameStatus::Ongoing);
}

#[test]
fn rejects_invalid_states() {
    let bad = [
        (r#"{ "players": [ { "hand": ["Leeroy Jenkins"] }, {} ] }"#, "unknown card"),
        (r#"{ "players": [ { "hand": [200] }, {} ] }"#, "unknown card id"),
        (r#"{ "players": [ { "board": [{"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0}] }, {} ] }"#, "board"),
        (r#"{ "players": [ { "hand": [0,0,0,0,0,0,0,0,0,0,0] }, {} ] }"#, "hand"),
        (r#"{ "players": [ { "max_mana": 11 }, {} ] }"#, "max_mana"),
        (r#"{ "players": [ { "board": [{"card":0,"health":0}] }, {} ] }"#, "health"),
        (r#"{ "current_player": 2, "players": [ {}, {} ] }"#, "current_player"),
        (r#"{ "status": "player0_won", "players": [ {}, {} ] }"#, "inconsistent"),
        (r#"{ "players": [ { "manna": 3 }, {} ] }"#, "unknown field"),
        (r#"{ "players": [ {} ] }"#, ""),
    ];
    for (json, needle) in bad {
        let err = from_json(json).expect_err(json).to_string();
        assert!(err.contains(needle), "error {err:?} should mention {needle:?}");
    }
}

#[test]
fn loads_sandbox_file() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sandbox/lethal_puzzle.json");
    let s = from_json_file(path).unwrap();
    assert_eq!(s.current, 0);
    // The puzzle has exactly-lethal: Yeti (4) + Raptor (3) + hero power (1) = 8.
    assert_eq!(s.players[1].hero_health, 8);
}
