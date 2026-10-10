//! A decision as the arrays the policy reads, and batches of them.
//!
//! This is the trainer's own packing of a decision, written in Python
//! (`pack_decision`, `deck_arrays` and `batch_many_packed` there), in Rust
//! and number for number: the same arrays under the same names, with the
//! same types and shapes, filled by the same arithmetic in the same order.
//! The Python stays the reference: the trainer's tests hold the two
//! together on real duels, and `tests/vectors.rs` holds this crate to what
//! the reference gave for the decisions of `vectors/`.  Like the reference,
//! this adds printed card data and scales numbers; it decides nothing about
//! what a seat may see.

use std::sync::Arc;

use crate::features::{combat_value, Features, CATEGORIES, NUMERIC};
use crate::view::{self, View};
use crate::Result;

/// Cards a seat can be shown at once.
pub const MAX_CARDS: usize = 160;
/// A Main Deck and an Extra Deck.
pub const MAX_DECK_CARDS: usize = 80;
pub const MAX_HISTORY: usize = 32;
pub const MAX_ACTION_MEMBERS: usize = 64;
/// What is known of a card where it lies, beside its printed data.
pub const STATUS: usize = 13;
/// The cards a decision is about, each in a slot of its own.
pub const CONTEXT: usize = 7;
pub const SCALARS: usize = 24;
pub const ACTION_SCALARS: usize = 8;
pub const HISTORY_SCALARS: usize = 2;

/// A description names a card's effect as the card's code shifted past its
/// index among that card's effects.
const EFFECT_INDEX_BITS: u32 = 20;

fn effect_index(description: u64) -> f32 {
    ((description & ((1 << EFFECT_INDEX_BITS) - 1)).min(15) as f64 / 15.0) as f32
}

fn names_an_effect(description: u64) -> f32 {
    if description >> EFFECT_INDEX_BITS != 0 {
        1.0
    } else {
        0.0
    }
}

fn flag(on: bool) -> f32 {
    if on {
        1.0
    } else {
        0.0
    }
}

/// `min(value, most) / most`, as Python computes it.
fn capped(value: u64, most: u64) -> f32 {
    (value.min(most) as f64 / most as f64) as f32
}

/// The numbers of an array.
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    I64(Vec<i64>),
    F32(Vec<f32>),
    /// 0 or 1 a value, as NumPy keeps a `bool`.
    Bool(Vec<u8>),
}

impl Data {
    pub fn kind(&self) -> &'static str {
        match self {
            Data::I64(_) => "int64",
            Data::F32(_) => "float32",
            Data::Bool(_) => "bool",
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Data::I64(values) => values.len(),
            Data::F32(values) => values.len(),
            Data::Bool(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_ptr(&self) -> *const u8 {
        match self {
            Data::I64(values) => values.as_ptr().cast(),
            Data::F32(values) => values.as_ptr().cast(),
            Data::Bool(values) => values.as_ptr(),
        }
    }

    /// As many zeros, of the same type.
    fn zeros(&self, count: usize) -> Data {
        match self {
            Data::I64(_) => Data::I64(vec![0; count]),
            Data::F32(_) => Data::F32(vec![0.0; count]),
            Data::Bool(_) => Data::Bool(vec![0; count]),
        }
    }

    /// Copy `count` values of `from`, starting at `source`, to `target` here.
    fn copy(&mut self, target: usize, from: &Data, source: usize, count: usize) {
        match (self, from) {
            (Data::I64(to), Data::I64(from)) => {
                to[target..target + count].copy_from_slice(&from[source..source + count])
            }
            (Data::F32(to), Data::F32(from)) => {
                to[target..target + count].copy_from_slice(&from[source..source + count])
            }
            (Data::Bool(to), Data::Bool(from)) => {
                to[target..target + count].copy_from_slice(&from[source..source + count])
            }
            _ => unreachable!("an array keeps its type from one decision to the next"),
        }
    }
}

/// Numbers with a shape, in row-major order.
#[derive(Clone, Debug, PartialEq)]
pub struct Array {
    pub shape: Vec<usize>,
    pub data: Data,
}

impl Array {
    fn new(shape: &[usize], data: Data) -> Arc<Array> {
        debug_assert_eq!(shape.iter().product::<usize>(), data.len());
        Arc::new(Array { shape: shape.to_vec(), data })
    }

    fn scalar(value: i64) -> Arc<Array> {
        Array::new(&[1], Data::I64(vec![value]))
    }

    /// The values of one entry along the first axes `axes`.
    fn stride(&self, axes: usize) -> usize {
        self.shape[axes..].iter().product()
    }
}

/// Rows of card arrays, filled one card at a time.
struct Cards {
    count: usize,
    card: Vec<i64>,
    location: Vec<i64>,
    position: Vec<i64>,
    controller: Vec<i64>,
    categories: Vec<i64>,
    numeric: Vec<f32>,
    status: Vec<f32>,
}

/// A card a choice, the question or the state of play names.
struct Reference {
    /// Its place among the cards of the view, or -1.
    card: i64,
    code: u32,
    controller: u8,
    location: u8,
    position: u8,
}

impl From<&view::Named> for Reference {
    fn from(named: &view::Named) -> Self {
        Reference {
            card: i64::from(named.card),
            code: named.code,
            controller: named.controller,
            location: named.location,
            position: named.position,
        }
    }
}

impl Cards {
    fn new(count: usize) -> Cards {
        Cards {
            count,
            card: vec![0; count],
            location: vec![0; count],
            position: vec![0; count],
            controller: vec![0; count],
            categories: vec![0; count * CATEGORIES],
            numeric: vec![0.0; count * NUMERIC],
            status: vec![0.0; count * STATUS],
        }
    }

    fn printed(&mut self, row: usize, code: u32, features: &Features) {
        let printed = features.printed(code);
        self.categories[row * CATEGORIES..(row + 1) * CATEGORIES].copy_from_slice(&printed.categories);
        self.numeric[row * NUMERIC..(row + 1) * NUMERIC].copy_from_slice(&printed.numeric);
    }

    /// A card of the view, where it lies and as it is now.
    fn put(&mut self, row: usize, card: &view::Card, features: &Features) {
        self.card[row] = i64::from(card.code);
        self.location[row] = i64::from(card.location);
        self.position[row] = i64::from(card.position);
        self.controller[row] = i64::from(card.controller);
        // What happens to a card is seen even when the card is not.
        self.status[row * STATUS..(row + 1) * STATUS].copy_from_slice(&[
            flag(card.code != 0),
            capped(u64::from(card.level), 12),
            flag(card.can_attack),
            capped(u64::from(card.battles), 3),
            capped(u64::from(card.counters), 10),
            flag(card.coin == 1),
            flag(card.coin == 2),
            flag(card.selected),
            flag(card.attacking),
            flag(card.attacked),
            flag(card.event),
            flag(card.chained),
            flag(card.targeted),
        ]);
        if card.code == 0 {
            return;
        }
        self.printed(row, card.code, features);
        // Current ATK and DEF replace the printed ones.
        self.numeric[row * NUMERIC] = combat_value(i64::from(card.attack));
        self.numeric[row * NUMERIC + 1] = combat_value(i64::from(card.defense));
    }

    /// A named card: the card of the view when it is one of those, else what
    /// its code alone says.
    fn put_named(&mut self, row: usize, named: &Reference, cards: &[view::Card], features: &Features) {
        if named.card >= 0 {
            if let Some(card) = cards.get(named.card as usize) {
                if card.code == named.code {
                    self.put(row, card, features);
                    return;
                }
            }
        }
        self.card[row] = i64::from(named.code);
        self.location[row] = i64::from(named.location);
        self.position[row] = i64::from(named.position);
        self.controller[row] = i64::from(named.controller);
        if named.code != 0 {
            self.printed(row, named.code, features);
            self.status[row * STATUS] = 1.0;
        }
    }

    /// The seven arrays, each with the shape `shape` before its own columns.
    fn named(self, prefix: &'static str, shape: &[usize], into: &mut Vec<(String, Arc<Array>)>) {
        debug_assert_eq!(shape.iter().product::<usize>(), self.count);
        let with = |columns: usize| [shape, &[columns][..]].concat();
        into.push((format!("{prefix}card"), Array::new(shape, Data::I64(self.card))));
        into.push((format!("{prefix}location"), Array::new(shape, Data::I64(self.location))));
        into.push((format!("{prefix}position"), Array::new(shape, Data::I64(self.position))));
        into.push((format!("{prefix}controller"), Array::new(shape, Data::I64(self.controller))));
        into.push((format!("{prefix}categories"), Array::new(&with(CATEGORIES), Data::I64(self.categories))));
        into.push((format!("{prefix}numeric"), Array::new(&with(NUMERIC), Data::F32(self.numeric))));
        into.push((format!("{prefix}status"), Array::new(&with(STATUS), Data::F32(self.status))));
    }
}

/// A deck list as arrays: built once for a deck and shared by its decisions.
#[derive(Clone, Debug)]
pub struct DeckArrays {
    card: Arc<Array>,
    categories: Arc<Array>,
    numeric: Arc<Array>,
}

impl DeckArrays {
    /// `codes` are the Main Deck then the Extra Deck.
    pub fn new(codes: &[u32], features: &Features) -> Result<DeckArrays> {
        let codes = &codes[..codes.len().min(MAX_DECK_CARDS)];
        if codes.is_empty() {
            return Err("a deck list has at least one card".into());
        }
        let mut categories = Vec::with_capacity(codes.len() * CATEGORIES);
        let mut numeric = Vec::with_capacity(codes.len() * NUMERIC);
        for code in codes {
            let printed = features.printed(*code);
            categories.extend_from_slice(&printed.categories);
            numeric.extend_from_slice(&printed.numeric);
        }
        Ok(DeckArrays {
            card: Array::new(&[codes.len()], Data::I64(codes.iter().map(|code| i64::from(*code)).collect())),
            categories: Array::new(&[codes.len(), CATEGORIES], Data::I64(categories)),
            numeric: Array::new(&[codes.len(), NUMERIC], Data::F32(numeric)),
        })
    }
}

/// One decision: its arrays by name, in the order Python gives them.
#[derive(Clone, Debug, Default)]
pub struct Packed {
    pub arrays: Vec<(String, Arc<Array>)>,
}

impl Packed {
    /// Where the array `name` lies: the same place in every decision.
    fn place(&self, name: &str) -> usize {
        self.arrays.iter().position(|(key, _)| key == name).expect("every decision has every array")
    }

    fn at(&self, place: usize) -> &Array {
        &self.arrays[place].1
    }
}

/// The cards the decision is about, each in its slot.
fn context(view: &View, cards: &[view::Card], features: &Features, into: &mut Vec<(String, Arc<Array>)>) {
    let mut rows = Cards::new(CONTEXT);
    let mut mask = vec![0u8; CONTEXT];
    let mut put = |slot: usize, named: Option<Reference>| {
        let Some(named) = named else { return };
        if named.card < 0 && named.code == 0 {
            return;
        }
        rows.put_named(slot, &named, cards, features);
        mask[slot] = 1;
    };
    let flagged = |flag: fn(&view::Card) -> bool| {
        cards.iter().position(flag).map(|index| Reference {
            card: index as i64,
            code: cards[index].code,
            controller: 0,
            location: 0,
            position: 0,
        })
    };
    let link = |position: usize| {
        let entry = view.chain.len().checked_sub(position).map(|index| &view.chain[index])?;
        Some(Reference {
            card: i64::from(entry.source),
            code: entry.code,
            controller: entry.controller,
            location: 0,
            position: 0,
        })
    };
    put(0, link(1));
    put(1, link(2));
    put(2, flagged(|card| card.attacking));
    put(3, flagged(|card| card.attacked));
    put(
        4,
        view.events.first().map(|event| Reference {
            card: i64::from(event.card),
            code: event.code,
            controller: event.controller,
            location: event.location,
            position: 0,
        }),
    );
    let subject = view.question.subject;
    put(5, (subject != 0).then_some(Reference { card: -1, code: subject, controller: 0, location: 0, position: 0 }));
    put(6, view.acting.as_ref().map(Reference::from));
    rows.named("context_", &[CONTEXT], into);
    into.push(("context_mask".to_owned(), Array::new(&[CONTEXT], Data::Bool(mask))));
}

/// The arrays of one decision of a duel between the decks `own` (the
/// deciding seat's) and `other`.
pub fn pack(view: &View, own: &DeckArrays, other: &DeckArrays, features: &Features) -> Result<Packed> {
    let cards = &view.cards[..view.cards.len().min(MAX_CARDS)];
    let mut state = Cards::new(cards.len().max(1));
    for (row, card) in cards.iter().enumerate() {
        state.put(row, card, features);
    }

    let question = &view.question;
    let piles = &view.piles;
    let toss = view.coin_toss.unwrap_or([0, 0]);
    let pile = |count: u32, most: f64| (f64::from(count) / most) as f32;
    let scalars = vec![
        (f64::from(view.life_points[0]) / 8000.0) as f32,
        (f64::from(view.life_points[1]) / 8000.0) as f32,
        (f64::from(view.turn) / 100.0) as f32,
        (view.chain.len() as f64 / 16.0) as f32,
        pile(piles[1][1], 10.0),
        pile(piles[0][0], 60.0),
        pile(piles[0][1], 60.0),
        1.0,
        match view.turn_player {
            player if player < 0 => 0.0,
            0 => 1.0,
            _ => -1.0,
        },
        pile(piles[4][0], 15.0),
        pile(piles[4][1], 15.0),
        pile(piles[1][0], 10.0),
        pile(piles[2][0], 60.0),
        pile(piles[2][1], 60.0),
        pile(piles[3][0], 60.0),
        pile(piles[3][1], 60.0),
        flag(view.summon_used),
        capped(u64::from(question.minimum), 10),
        capped(u64::from(question.maximum), 10),
        flag(question.forced),
        flag(question.triggers),
        capped(u64::from(toss[0]), 3),
        capped(u64::from(toss[1]), 3),
        capped(view.step as u64, 10),
    ];
    debug_assert_eq!(scalars.len(), SCALARS);

    let choices = &view.choices;
    if choices.is_empty() {
        return Err("a decision offers at least one answer".into());
    }
    let count = choices.len();
    let width = choices.iter().map(|choice| choice.members.len()).max().unwrap_or(0).clamp(1, MAX_ACTION_MEMBERS);
    let mut actions = Cards::new(count);
    let mut action_kind = vec![0i64; count];
    let mut action_scalars = vec![0f32; count * ACTION_SCALARS];
    let mut members = Cards::new(count * width);
    let mut member_value = vec![0i64; count * width];
    let mut member_order = vec![0f32; count * width];
    let mut member_mask = vec![0u8; count * width];
    for (row, choice) in choices.iter().enumerate() {
        action_kind[row] = i64::from(choice.kind);
        let mut value = 0i64;
        if let Some(named) = &choice.card {
            actions.put_named(row, &Reference::from(named), cards, features);
            value = named.value;
        }
        action_scalars[row * ACTION_SCALARS..(row + 1) * ACTION_SCALARS].copy_from_slice(&[
            flag(choice.card.is_some()),
            flag(choice.card.as_ref().is_some_and(|named| named.required)),
            ((value & 0xffff).min(16) as f64 / 16.0) as f32,
            (((value >> 16) & 0xffff).min(16) as f64 / 16.0) as f32,
            capped(choice.members.len() as u64, 8),
            effect_index(choice.description),
            names_an_effect(choice.description),
            1.0,
        ]);
        for (column, member) in choice.members.iter().take(MAX_ACTION_MEMBERS).enumerate() {
            let place = row * width + column;
            members.put_named(place, &Reference::from(member), cards, features);
            member_value[place] = member.value;
            member_order[place] = (column as f64 / MAX_ACTION_MEMBERS as f64) as f32;
            member_mask[place] = 1;
        }
    }

    let history = &view.history[view.history.len().saturating_sub(MAX_HISTORY)..];
    let past = history.len().max(1);
    let mut history_card = vec![0i64; past];
    let mut history_location = vec![0i64; past];
    let mut history_kind = vec![0i64; past];
    let mut history_scalars = vec![0f32; past * HISTORY_SCALARS];
    for (row, answer) in history.iter().enumerate() {
        history_card[row] = i64::from(answer.code);
        history_location[row] = i64::from(answer.location);
        history_kind[row] = i64::from(answer.kind);
        history_scalars[row * HISTORY_SCALARS] = effect_index(answer.description);
        history_scalars[row * HISTORY_SCALARS + 1] = 1.0;
    }

    let mut arrays = Vec::with_capacity(48);
    let mut put = |name: &str, array: Arc<Array>| arrays.push((name.to_owned(), array));
    put("state_length", Array::scalar(cards.len() as i64));
    let rows = state.count;
    state.named("state_", &[rows], &mut arrays);
    context(view, cards, features, &mut arrays);
    let mut put = |name: &str, array: Arc<Array>| arrays.push((name.to_owned(), array));
    put("self_deck", own.card.clone());
    put("self_deck_categories", own.categories.clone());
    put("self_deck_numeric", own.numeric.clone());
    put("opponent_deck", other.card.clone());
    put("opponent_deck_categories", other.categories.clone());
    put("opponent_deck_numeric", other.numeric.clone());
    put("phase", Array::scalar(i64::from(view.phase)));
    put("decision_kind", Array::scalar(i64::from(question.kind)));
    put("decision_hint", Array::scalar(i64::from(question.hint)));
    put("scalars", Array::new(&[SCALARS], Data::F32(scalars)));
    put("action_kind", Array::new(&[count], Data::I64(action_kind)));
    actions.named("action_", &[count], &mut arrays);
    arrays.push(("action_scalars".to_owned(), Array::new(&[count, ACTION_SCALARS], Data::F32(action_scalars))));
    members.named("action_member_", &[count, width], &mut arrays);
    let mut put = |name: &str, array: Arc<Array>| arrays.push((name.to_owned(), array));
    put("action_member_value", Array::new(&[count, width], Data::I64(member_value)));
    put("action_member_order", Array::new(&[count, width], Data::F32(member_order)));
    put("action_member_mask", Array::new(&[count, width], Data::Bool(member_mask)));
    put("history_card", Array::new(&[past], Data::I64(history_card)));
    put("history_location", Array::new(&[past], Data::I64(history_location)));
    put("history_kind", Array::new(&[past], Data::I64(history_kind)));
    put("history_scalars", Array::new(&[past, HISTORY_SCALARS], Data::F32(history_scalars)));
    put("history_length", Array::scalar(history.len() as i64));
    Ok(Packed { arrays })
}

/// Packed decisions padded into one batch: the tensors the model is given.
#[derive(Debug)]
pub struct Batch {
    pub arrays: Vec<(String, Array)>,
}

/// The lists whose length differs from one decision to the next, by the
/// start of the names of their arrays.
const LISTS: [&str; 5] = ["state", "self_deck", "opponent_deck", "action", "history"];

/// Pad already packed decisions into one batch.
pub fn batch(items: &[&Packed]) -> Result<Batch> {
    let Some(first) = items.first() else {
        return Err("cannot batch an empty list of decisions".into());
    };
    // An array is looked for once and then taken by its place, which is the
    // same in every decision.
    let alike = |item: &&Packed| {
        item.arrays.len() == first.arrays.len() && item.arrays.iter().zip(&first.arrays).all(|(a, b)| a.0 == b.0)
    };
    if !items.iter().all(alike) {
        return Err("the decisions of a batch have the same arrays, in the same order".into());
    }
    let count = items.len();
    let longest = |name: &str| {
        let place = first.place(name);
        items.iter().map(|item| item.at(place).shape[0]).max().unwrap_or(0)
    };
    let widths = [
        longest("state_card"),
        longest("self_deck"),
        longest("opponent_deck"),
        longest("action_kind"),
        longest("history_card"),
    ];
    let member = first.place("action_member_card");
    let members = items.iter().map(|item| item.at(member).shape[1]).max().unwrap_or(0);

    // Pad the first axis of each decision's array to `width`.
    let padded = |place: usize, width: usize| {
        let like = first.at(place);
        let stride = like.stride(1);
        let mut data = like.data.zeros(count * width * stride);
        for (index, item) in items.iter().enumerate() {
            let value = item.at(place);
            data.copy(index * width * stride, &value.data, 0, value.data.len());
        }
        Array { shape: [&[count, width][..], &like.shape[1..]].concat(), data }
    };
    // Pad the action and member axes of each decision's array.
    let padded_members = |place: usize| {
        let like = first.at(place);
        let stride = like.stride(2);
        let actions = widths[3];
        let mut data = like.data.zeros(count * actions * members * stride);
        for (index, item) in items.iter().enumerate() {
            let value = item.at(place);
            let (rows, columns) = (value.shape[0], value.shape[1]);
            for row in 0..rows {
                data.copy(
                    ((index * actions + row) * members) * stride,
                    &value.data,
                    row * columns * stride,
                    columns * stride,
                );
            }
        }
        Array { shape: [&[count, actions, members][..], &like.shape[2..]].concat(), data }
    };
    let stacked = |place: usize| {
        let like = first.at(place);
        let stride = like.data.len();
        let mut data = like.data.zeros(count * stride);
        for (index, item) in items.iter().enumerate() {
            data.copy(index * stride, &item.at(place).data, 0, stride);
        }
        Array { shape: [&[count][..], &like.shape[..]].concat(), data }
    };
    // One number a decision.
    let one = |values: Vec<i64>| Array { shape: vec![count], data: Data::I64(values) };
    let single = |name: &str| {
        let place = first.place(name);
        one(items
            .iter()
            .map(|item| match &item.at(place).data {
                Data::I64(values) => values[0],
                _ => unreachable!("{name} is a whole number"),
            })
            .collect())
    };
    let lengths = |name: &str| {
        let place = first.place(name);
        one(items.iter().map(|item| item.at(place).shape[0] as i64).collect())
    };

    let actions = widths[3];
    let kinds = first.place("action_kind");
    let mut mask = vec![0u8; count * actions];
    for (index, item) in items.iter().enumerate() {
        let offered = item.at(kinds).shape[0];
        mask[index * actions..index * actions + offered].fill(1);
    }
    let mut arrays = vec![
        ("state_lengths".to_owned(), single("state_length")),
        ("self_deck_lengths".to_owned(), lengths("self_deck")),
        ("opponent_deck_lengths".to_owned(), lengths("opponent_deck")),
        ("history_lengths".to_owned(), single("history_length")),
        ("action_mask".to_owned(), Array { shape: vec![count, actions], data: Data::Bool(mask) }),
        ("phase".to_owned(), single("phase")),
        ("decision_kind".to_owned(), single("decision_kind")),
        ("decision_hint".to_owned(), single("decision_hint")),
        ("scalars".to_owned(), stacked(first.place("scalars"))),
    ];
    for (place, (name, _)) in first.arrays.iter().enumerate() {
        if arrays.iter().any(|(key, _)| key == name) || name == "state_length" || name == "history_length" {
            continue;
        }
        let array = if name.starts_with("context_") {
            stacked(place)
        } else if name.starts_with("action_member_") {
            padded_members(place)
        } else {
            let list = LISTS
                .iter()
                .position(|list| name == list || name.strip_prefix(list).is_some_and(|rest| rest.starts_with('_')))
                .ok_or_else(|| format!("{name} belongs to no list of the input"))?;
            padded(place, widths[list])
        };
        arrays.push((name.clone(), array));
    }
    Ok(Batch { arrays })
}
