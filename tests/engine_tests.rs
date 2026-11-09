use hsengine::action::*;
use hsengine::serializer::from_json;
use hsengine::state::*;
use hsengine::{random_rollout, GameState};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

fn load(json: &str) -> GameState {
    from_json(json).unwrap_or_else(|e| panic!("bad test state: {e}"))
}

fn attack(a: u8, t: u8) -> Action {
    Action::Attack { attacker_idx: a, target_idx: t }
}

fn assert_invariants(s: &GameState) {
    for p in &s.players {
        assert!(p.board_len as usize <= MAX_BOARD);
        assert!(p.hand_len as usize <= MAX_HAND);
        assert!(p.deck_len as usize <= MAX_DECK);
        assert!(p.minions[p.board_len as usize..].iter().all(|m| *m == Minion::EMPTY));
        assert!(p.hand[p.hand_len as usize..].iter().all(|&c| c == 0));
        assert!(p.deck[p.deck_len as usize..].iter().all(|&c| c == 0));
        assert!(p.board().iter().all(|m| m.health > 0));
        assert!(p.max_mana <= MAX_MANA);
    }
}

// ---------------------------------------------------------------- action space

#[test]
fn action_index_roundtrip() {
    assert_eq!(MAX_ACTIONS, 91);
    for i in 0..MAX_ACTIONS {
        let a = Action::from_index(i).unwrap();
        assert_eq!(a.to_index(), i, "{a:?}");
    }
    assert_eq!(Action::from_index(MAX_ACTIONS), None);
    assert_eq!(Action::EndTurn.to_index(), 90);
    assert_eq!(attack(HERO_SLOT, HERO_SLOT).to_index(), HERO_POWER_BASE - 1);
}

#[test]
fn card_ids_match_indices() {
    for (i, c) in hsengine::cards::CARDS.iter().enumerate() {
        assert_eq!(c.id as usize, i);
        assert!((1..=7).contains(&c.cost));
    }
}

// ---------------------------------------------------------------- combat

#[test]
fn combat_trade_damages_and_removes_preserving_order() {
    let mut s = load(
        r#"{ "players": [
            { "max_mana": 4, "board": [ { "card": "Chillwind Yeti" } ] },
            { "board": [ { "card": "River Crocolisk" }, { "card": "Bloodfen Raptor" }, { "card": "Boulderfist Ogre" } ] }
        ]}"#,
    );
    s.apply_action(attack(0, 1)); // Yeti 4/5 hits Raptor 3/2
    let me = &s.players[0];
    let op = &s.players[1];
    assert_eq!(me.board_len, 1);
    assert_eq!((me.minions[0].attack, me.minions[0].health), (4, 2));
    assert!(!me.minions[0].can_attack);
    assert_eq!(op.board_len, 2);
    let names: Vec<_> = op.board().iter().map(|m| hsengine::cards::card(m.card).name).collect();
    assert_eq!(names, ["River Crocolisk", "Boulderfist Ogre"]);
    assert_invariants(&s);
}

#[test]
fn combat_mutual_destruction() {
    let mut s = load(
        r#"{ "players": [
            { "board": [ { "card": "Bloodfen Raptor" } ] },
            { "board": [ { "card": "Bloodfen Raptor" } ] }
        ]}"#,
    );
    s.apply_action(attack(0, 0));
    assert_eq!(s.players[0].board_len, 0);
    assert_eq!(s.players[1].board_len, 0);
    assert_invariants(&s);
}

#[test]
fn attacking_hero_has_no_counter_damage() {
    let mut s = load(
        r#"{ "players": [ { "board": [ { "card": "Magma Rager" } ] }, {} ] }"#,
    );
    s.apply_action(attack(0, HERO_SLOT));
    assert_eq!(s.players[1].hero_health, 25);
    assert_eq!(s.players[0].minions[0].health, 1);
    // Already attacked: no more attack actions.
    assert!(!s.get_legal_actions().iter().any(|a| matches!(a, Action::Attack { .. })));
}

#[test]
fn zero_attack_minion_cannot_attack() {
    let s = load(
        r#"{ "players": [ { "board": [ { "card": "Oasis Snapjaw", "attack": 0 } ] }, {} ] }"#,
    );
    assert_eq!(s.get_legal_actions(), vec![Action::EndTurn]);
}

// ---------------------------------------------------------------- summoning sickness & mana

#[test]
fn summoning_sickness() {
    let mut s = load(
        r#"{ "players": [ { "max_mana": 2, "hand": ["Bloodfen Raptor"], "deck": ["Murloc Raider", "Murloc Raider"] },
                          { "deck": ["Murloc Raider"] } ] }"#,
    );
    s.apply_action(Action::PlayMinion { hand_idx: 0 });
    assert_eq!(s.players[0].mana, 0);
    assert_eq!(s.players[0].board_len, 1);
    assert!(!s.players[0].minions[0].can_attack);
    assert!(!s.is_legal(attack(0, HERO_SLOT)));
    assert_eq!(s.get_legal_actions(), vec![Action::EndTurn]);

    s.apply_action(Action::EndTurn);
    assert_eq!(s.current, 1);
    assert!(!s.players[0].minions[0].can_attack, "sickness clears on OWNER's turn");
    s.apply_action(Action::EndTurn);
    assert_eq!(s.current, 0);
    assert!(s.players[0].minions[0].can_attack);
    assert!(s.is_legal(attack(0, HERO_SLOT)));
}

#[test]
fn mana_cost_and_crystal_growth() {
    let s = load(
        r#"{ "players": [ { "max_mana": 3, "hand": ["Chillwind Yeti", "Spider Tank"] }, {} ] }"#,
    );
    assert!(!s.is_legal(Action::PlayMinion { hand_idx: 0 }));
    assert!(s.is_legal(Action::PlayMinion { hand_idx: 1 }));

    // Mana caps at 10 and refills each turn (empty decks -> fatigue, so give lots of HP).
    let mut s = load(
        r#"{ "players": [ { "hero_health": 1000, "hero_max_health": 1000 },
                          { "hero_health": 1000, "hero_max_health": 1000 } ] }"#,
    );
    for _ in 0..30 {
        s.apply_action(Action::EndTurn);
        let me = s.me();
        assert_eq!(me.mana, me.max_mana);
    }
    assert_eq!(s.players[0].max_mana, 10);
    assert_eq!(s.players[1].max_mana, 10);
}

#[test]
fn board_limit_blocks_play() {
    let mut s = load(
        r#"{ "players": [ { "max_mana": 10, "hand": ["Murloc Raider"],
              "board": [ {"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0} ] }, {} ] }"#,
    );
    assert!(s.is_legal(Action::PlayMinion { hand_idx: 0 }));
    s.apply_action(Action::PlayMinion { hand_idx: 0 });
    assert_eq!(s.players[0].board_len, 7);
    // New minion spawns far right and is sick.
    assert!(!s.players[0].minions[6].can_attack);

    let s = load(
        r#"{ "players": [ { "max_mana": 10, "hand": ["Murloc Raider"],
              "board": [ {"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0},{"card":0} ] }, {} ] }"#,
    );
    assert!(!s.is_legal(Action::PlayMinion { hand_idx: 0 }));
    assert!(!s.get_action_mask()[PLAY_BASE]);
}

#[test]
fn hand_removal_preserves_order() {
    let mut s = load(
        r#"{ "players": [ { "max_mana": 10, "hand": [0, 1, 2, 3] }, {} ] }"#,
    );
    s.apply_action(Action::PlayMinion { hand_idx: 1 });
    assert_eq!(s.players[0].hand_cards(), &[0, 2, 3]);
    assert_eq!(s.players[0].mana, 8);
    assert_invariants(&s);
}

// ---------------------------------------------------------------- hero power

#[test]
fn hero_power_once_per_turn() {
    let mut s = load(
        r#"{ "players": [ { "max_mana": 4, "board": [ {"card": "Murloc Raider"} ] },
                          { "board": [ {"card": "Murloc Raider"}, {"card": "Chillwind Yeti"} ] } ] }"#,
    );
    s.apply_action(Action::HeroPower { target_idx: 0 }); // kill enemy Murloc
    assert_eq!(s.players[1].board_len, 1);
    assert_eq!(hsengine::cards::card(s.players[1].minions[0].card).name, "Chillwind Yeti");
    assert_eq!(s.players[0].mana, 2);
    assert!(!s.is_legal(Action::HeroPower { target_idx: HP_ENEMY_HERO }));

    let mut s2 = load(
        r#"{ "players": [ { "max_mana": 2, "board": [ {"card": "Murloc Raider"} ] }, {} ] }"#,
    );
    assert!(!s2.is_legal(Action::HeroPower { target_idx: 0 }), "no enemy minion 0");
    assert!(s2.is_legal(Action::HeroPower { target_idx: HP_FRIENDLY_BASE }));
    assert!(!s2.is_legal(Action::HeroPower { target_idx: HP_FRIENDLY_BASE + 1 }));
    s2.apply_action(Action::HeroPower { target_idx: HP_FRIENDLY_BASE });
    assert_eq!(s2.players[0].board_len, 0);
}

// ---------------------------------------------------------------- lethal & fatigue

#[test]
fn lethal_detection_ends_game() {
    let mut s = load(
        r#"{ "players": [ { "board": [ {"card": "Chillwind Yeti"} ] }, { "hero_health": 4 } ] }"#,
    );
    s.apply_action(attack(0, HERO_SLOT));
    assert_eq!(s.status, GameStatus::Player0Won);
    assert!(s.is_terminal());
    assert!(s.get_legal_actions().is_empty());
    assert!(s.get_action_mask().iter().all(|&b| !b));
    assert!(!s.is_legal(Action::EndTurn));
}

#[test]
fn hero_power_lethal_and_self_kill() {
    let mut s = load(r#"{ "players": [ { "max_mana": 2 }, { "hero_health": 1 } ] }"#);
    s.apply_action(Action::HeroPower { target_idx: HP_ENEMY_HERO });
    assert_eq!(s.status, GameStatus::Player0Won);

    let mut s = load(r#"{ "players": [ { "max_mana": 2, "hero_health": 1 }, {} ] }"#);
    s.apply_action(Action::HeroPower { target_idx: HP_FRIENDLY_HERO });
    assert_eq!(s.status, GameStatus::Player1Won);
}

#[test]
fn fatigue_is_cumulative_and_can_kill() {
    let mut s = load(r#"{ "players": [ {}, {} ] }"#);
    s.apply_action(Action::EndTurn); // P1 draws from empty deck: 1
    assert_eq!(s.players[1].hero_health, 29);
    s.apply_action(Action::EndTurn); // P0: 1
    assert_eq!(s.players[0].hero_health, 29);
    s.apply_action(Action::EndTurn); // P1: 2
    assert_eq!(s.players[1].hero_health, 27);
    s.apply_action(Action::EndTurn); // P1: ... P0: 2
    s.apply_action(Action::EndTurn); // P1: 3
    assert_eq!(s.players[1].hero_health, 24);
    assert_eq!(s.players[1].fatigue, 3);

    let mut s = load(r#"{ "players": [ {}, { "hero_health": 3, "fatigue": 2 } ] }"#);
    s.apply_action(Action::EndTurn);
    assert_eq!(s.players[1].hero_health, 0);
    assert_eq!(s.status, GameStatus::Player0Won);
}

#[test]
fn full_hand_burns_draw() {
    let mut s = load(
        r#"{ "current_player": 1, "players": [
            { "hand": [0,0,0,0,0,0,0,0,0,0], "deck": ["War Golem", "Core Hound"] }, {} ] }"#,
    );
    s.apply_action(Action::EndTurn);
    let p = &s.players[0];
    assert_eq!(p.hand_len, 10);
    assert!(p.hand_cards().iter().all(|&c| c == 0), "War Golem was burned");
    assert_eq!(p.deck_len, 1);
    assert_eq!(p.fatigue, 0);
}

#[test]
fn new_game_deal() {
    let deck: Vec<CardId> = (0..30).map(|i| (i % 11) as CardId).collect();
    let s = GameState::new_game(&deck, &deck);
    assert_eq!(s.current, 0);
    assert_eq!(s.turn, 1);
    assert_eq!(s.players[0].hand_len, 4);
    assert_eq!(s.players[1].hand_len, 4);
    assert_eq!(s.players[0].deck_len, 26);
    assert_eq!(s.players[0].max_mana, 1);
    assert_eq!(s.players[1].max_mana, 0);
    // Draw order: first element of the deck list is drawn first.
    assert_eq!(s.players[0].hand_cards(), &[0, 1, 2, 3]);
}

// ---------------------------------------------------------------- transpositions

#[test]
fn transposed_action_orders_reach_identical_state() {
    let base = load(
        r#"{ "players": [
            { "max_mana": 6, "hand": ["Murloc Raider", "Spider Tank"],
              "board": [ {"card": "Chillwind Yeti"}, {"card": "Bloodfen Raptor"} ] },
            { "board": [ {"card": "Oasis Snapjaw"} ] } ] }"#,
    );
    // Order 1: Yeti face, Raptor face, play Murloc.
    let mut a = base;
    a.apply_action(attack(0, HERO_SLOT));
    a.apply_action(attack(1, HERO_SLOT));
    a.apply_action(Action::PlayMinion { hand_idx: 0 });
    // Order 2: play Murloc, Raptor face, Yeti face.
    let mut b = base;
    b.apply_action(Action::PlayMinion { hand_idx: 0 });
    b.apply_action(attack(1, HERO_SLOT));
    b.apply_action(attack(0, HERO_SLOT));

    assert_eq!(a, b);
    assert_eq!(a.observable_hash(0), b.observable_hash(0));
    assert_eq!(a.observable_hash(1), b.observable_hash(1));

    let mut c = base;
    c.apply_action(attack(0, 0)); // a genuinely different line
    assert_ne!(a.observable_hash(0), c.observable_hash(0));
}

#[test]
fn observable_hash_ignores_hidden_info() {
    let a = load(r#"{ "players": [ { "hand": [1, 2], "deck": [3, 4] }, { "hand": [5], "deck": [6, 7] } ] }"#);
    let b = load(r#"{ "players": [ { "hand": [1, 2], "deck": [4, 3] }, { "hand": [9], "deck": [8, 8] } ] }"#);
    assert_ne!(a, b);
    assert_eq!(a.observable_hash(0), b.observable_hash(0));
    assert_ne!(a.observable_hash(1), b.observable_hash(1), "P1 sees its own hand changed");
}

// ---------------------------------------------------------------- random play fuzz

#[test]
fn random_games_respect_invariants_and_mask() {
    let mut rng = SmallRng::seed_from_u64(7);
    let mut list = ActionList::new();
    for _ in 0..300 {
        let mut s = GameState::new_random_game(&mut rng, 30);
        let mut steps = 0;
        while !s.is_terminal() {
            assert_invariants(&s);
            s.legal_actions_into(&mut list);
            let mask = s.get_action_mask();
            assert_eq!(mask.iter().filter(|&&b| b).count(), list.len());
            for i in 0..MAX_ACTIONS {
                let a = Action::from_index(i).unwrap();
                assert_eq!(mask[i], s.is_legal(a), "mask/is_legal mismatch for {a:?}");
            }
            assert!(list.contains(&Action::EndTurn));
            let a = list[rng.random_range(0..list.len())];
            let player = s.current;
            s.apply_action(a);
            if a != Action::EndTurn && !s.is_terminal() {
                assert_eq!(s.current, player, "micro-actions keep the active player");
            }
            steps += 1;
            assert!(steps < 10_000, "game did not terminate");
        }
        assert_invariants(&s);
    }
}

#[test]
fn rollout_terminates_from_fresh_game() {
    let mut rng = SmallRng::seed_from_u64(1);
    for _ in 0..1000 {
        let mut s = GameState::new_random_game(&mut rng, 30);
        let (status, _) = random_rollout(&mut s, &mut rng);
        assert!(status.is_terminal());
    }
}
