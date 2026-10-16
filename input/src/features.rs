//! Printed card data as the policy reads it.
//!
//! A card's row is made from the columns of the card database (EDOPro's
//! `cards.cdb`), in a fixed order.

use std::collections::HashMap;

/// kind, attribute, race, level, and four set codes.
pub const CATEGORIES: usize = 8;
/// ATK, DEF, then the monster, Spell and Trap subtype flags.
pub const NUMERIC: usize = 17;

const MONSTER: i64 = 0x1;
const SPELL: i64 = 0x2;
const TRAP: i64 = 0x4;
const NORMAL: i64 = 0x10;
const EFFECT: i64 = 0x20;
const FUSION: i64 = 0x40;
const RITUAL: i64 = 0x80;
const TUNER: i64 = 0x1000;
const SYNCHRO: i64 = 0x2000;
const QUICK_PLAY: i64 = 0x10000;
const CONTINUOUS: i64 = 0x20000;
const EQUIP: i64 = 0x40000;
const FIELD: i64 = 0x80000;
const COUNTER: i64 = 0x100000;

/// What is printed on a card.
#[derive(Clone, Debug, PartialEq)]
pub struct Printed {
    pub categories: [i64; CATEGORIES],
    pub numeric: [f32; NUMERIC],
}

/// A card the database does not hold.
pub const UNKNOWN: Printed = Printed { categories: [0; CATEGORIES], numeric: [0.0; NUMERIC] };

/// ATK or DEF on the policy's scale: -1 for the database's negative "?",
/// and no more than 10,000.
pub fn combat_value(value: i64) -> f32 {
    if value < 0 {
        -1.0
    } else {
        ((value as f64).min(10_000.0) / 4_000.0) as f32
    }
}

/// A one-hot value of the database as a small index; 0 is unknown.
fn bit_index(value: i64, maximum: i64) -> i64 {
    if value <= 0 || value & (value - 1) != 0 {
        return 0;
    }
    let bit_length = 64 - value.leading_zeros() as i64;
    bit_length.min(maximum - 1)
}

fn row(kind: i64, attack: i64, defense: i64, level: i64, race: i64, attribute: i64, setcode: i64) -> Printed {
    let (monster, spell, trap) = (kind & MONSTER != 0, kind & SPELL != 0, kind & TRAP != 0);
    let packed = setcode as u64;
    let set = |slot: u32| ((packed >> (16 * slot)) & 0x0fff) as i64;
    let categories = [
        if monster {
            1
        } else if spell {
            2
        } else if trap {
            3
        } else {
            0
        },
        bit_index(attribute, 16),
        bit_index(race, 64),
        (level.abs() & 0xff).min(15),
        set(0),
        set(1),
        set(2),
        set(3),
    ];
    let flag = |on: bool, bit: i64| if on && kind & bit != 0 { 1.0 } else { 0.0 };
    let numeric = [
        if monster { combat_value(attack) } else { 0.0 },
        if monster { combat_value(defense) } else { 0.0 },
        flag(monster, NORMAL),
        flag(monster, EFFECT),
        flag(monster, RITUAL),
        flag(monster, FUSION),
        flag(monster, SYNCHRO),
        flag(monster, TUNER),
        flag(spell, NORMAL),
        flag(spell, CONTINUOUS),
        flag(spell, EQUIP),
        flag(spell, FIELD),
        flag(spell, QUICK_PLAY),
        flag(spell, RITUAL),
        flag(trap, NORMAL),
        flag(trap, CONTINUOUS),
        flag(trap, COUNTER),
    ];
    Printed { categories, numeric }
}

/// Every card of the database.
pub struct Features {
    cards: HashMap<i64, Printed>,
}

impl Features {
    /// The cards of a database, each from its row of the `datas` table:
    /// `id`, then `type`, `atk`, `def`, `level`, `race`, `attribute` and
    /// `setcode`, as the database holds them.
    pub fn from_rows(rows: impl IntoIterator<Item = [i64; 8]>) -> Self {
        let cards = rows
            .into_iter()
            .map(|[code, kind, attack, defense, level, race, attribute, setcode]| {
                (code, row(kind, attack, defense, level, race, attribute, setcode))
            })
            .collect();
        Features { cards }
    }

    /// What is printed on the card `code`.
    pub fn printed(&self, code: u32) -> &Printed {
        self.cards.get(&i64::from(code)).unwrap_or(&UNKNOWN)
    }
}
