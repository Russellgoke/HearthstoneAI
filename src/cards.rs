//! Static card database. A card's `id` is its index into [`CARDS`], so lookups
//! are a single bounds-checked array index on the hot path.

use rand::Rng;

use crate::state::CardId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDef {
    pub id: CardId,
    pub name: &'static str,
    pub cost: u8,
    pub attack: i8,
    pub health: i8,
}

const fn c(id: CardId, name: &'static str, cost: u8, attack: i8, health: i8) -> CardDef {
    CardDef { id, name, cost, attack, health }
}

/// Vanilla minions (no keywords / text), mana costs 1-7.
pub static CARDS: [CardDef; 11] = [
    c(0, "Murloc Raider", 1, 2, 1),
    c(1, "Bloodfen Raptor", 2, 3, 2),
    c(2, "River Crocolisk", 2, 2, 3),
    c(3, "Magma Rager", 3, 5, 1),
    c(4, "Spider Tank", 3, 3, 4),
    c(5, "Chillwind Yeti", 4, 4, 5),
    c(6, "Oasis Snapjaw", 4, 2, 7),
    c(7, "Pit Fighter", 5, 5, 6),
    c(8, "Boulderfist Ogre", 6, 6, 7),
    c(9, "War Golem", 7, 7, 7),
    c(10, "Core Hound", 7, 9, 5),
];

pub const NUM_CARDS: usize = CARDS.len();

/// Unchecked-by-contract lookup: `id` must be a valid card id (all ids stored in a
/// `GameState` are validated on construction / deserialization).
#[inline(always)]
pub fn card(id: CardId) -> &'static CardDef {
    &CARDS[id as usize]
}

#[inline]
pub fn card_checked(id: CardId) -> Option<&'static CardDef> {
    CARDS.get(id as usize)
}

/// Case-insensitive lookup by card name.
pub fn find_by_name(name: &str) -> Option<&'static CardDef> {
    let name = name.trim();
    CARDS.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Fills `out` with uniformly random card ids (already in random order, so no
/// separate shuffle is needed). Allocation-free.
pub fn fill_random_deck<R: Rng + ?Sized>(rng: &mut R, out: &mut [CardId]) {
    for slot in out.iter_mut() {
        *slot = rng.random_range(0..NUM_CARDS) as CardId;
    }
}
