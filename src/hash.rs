//! Deterministic 64-bit hashing of the *observable* state for transposition
//! detection and branch pruning.
//!
//! The full `GameState` also derives `Hash`/`Eq` (exact-state dedup). This
//! module hashes only what `perspective` can see: both boards, heroes, mana,
//! flags, hand/deck sizes, and the perspective player's own hand contents. Deck
//! order and the opponent's hand are excluded, so states that differ only in
//! hidden information collide on purpose (what Open-Loop ISMCTS wants).
//!
//! Computed on demand (~40 multiply-xor rounds) rather than maintained
//! incrementally: with a ~260-byte state this is cheaper and far less
//! error-prone than threading Zobrist XOR updates through every mutation.

use crate::state::{GameState, PlayerState};

const K: u64 = 0x9E37_79B9_7F4A_7C15;

struct Mixer(u64);

impl Mixer {
    #[inline(always)]
    fn add(&mut self, v: u64) {
        self.0 = (self.0 ^ v).wrapping_mul(K).rotate_left(29);
    }

    #[inline(always)]
    fn finish(self) -> u64 {
        // splitmix64 finalizer
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[inline]
fn add_player(h: &mut Mixer, p: &PlayerState, reveal_hand: bool) {
    h.add(
        (p.hero_health as u16 as u64)
            | (p.hero_max_health as u16 as u64) << 16
            | (p.hero_attack as u8 as u64) << 32
            | (p.hero_attacked as u64) << 40
            | (p.hero_power_used as u64) << 41
            | (p.fatigue as u64) << 48,
    );
    h.add(
        p.mana as u64
            | (p.max_mana as u64) << 8
            | (p.board_len as u64) << 16
            | (p.hand_len as u64) << 24
            | (p.deck_len as u64) << 32,
    );
    for m in p.board() {
        h.add(
            m.card as u64
                | (m.attack as u8 as u64) << 8
                | (m.health as u8 as u64) << 16
                | (m.max_health as u8 as u64) << 24
                | (m.can_attack as u64) << 32,
        );
    }
    if reveal_hand {
        for chunk in p.hand_cards().chunks(8) {
            let mut w = 0u64;
            for (i, &c) in chunk.iter().enumerate() {
                w |= (c as u64 + 1) << (i * 8);
            }
            h.add(w);
        }
    }
}

/// Hash of the state as observed by `perspective` (0 or 1).
pub fn observable_hash(s: &GameState, perspective: u8) -> u64 {
    let mut h = Mixer(0x243F_6A88_85A3_08D3);
    h.add(s.current as u64 | (perspective as u64) << 8 | (s.status as u64) << 16);
    let p = (perspective & 1) as usize;
    add_player(&mut h, &s.players[p], true);
    add_player(&mut h, &s.players[p ^ 1], false);
    h.finish()
}

impl GameState {
    #[inline]
    pub fn observable_hash(&self, perspective: u8) -> u64 {
        observable_hash(self, perspective)
    }
}
