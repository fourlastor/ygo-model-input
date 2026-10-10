//! A decision of one seat, as plain data.
//!
//! A view holds what the seat may see when it is asked and nothing else:
//! `ygo-model-seat` derives it from what a pilot of `ygo-policies` is given
//! at the same point.  Players are relative to the seat (0: the seat
//! itself, 1: the other player) and cards are named by their place in
//! [`View::cards`].  A card's zone number is left out on purpose: it
//! identifies the engine's response and is not something to play by.
//!
//! Locations, positions, phases, hints and the kinds of a decision and of a
//! choice are small numbers here.  Which number stands for what is said
//! where a view is made (`ygo_model_seat::view`); nothing in this crate
//! reads them, they are what the policy's embeddings are indexed by.

use serde::{Deserialize, Serialize};

/// A card the seat sees, or one of the other player's it knows only the place of.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Card {
    /// 0: the seat's own, 1: the other player's.
    pub controller: u8,
    /// Where it lies, as a view numbers locations.
    pub location: u8,
    /// 0 when the seat cannot see which card it is.
    pub code: u32,
    /// Its battle position, as a view numbers them; 0 away from the field.
    pub position: u8,
    pub attack: i32,
    pub defense: i32,
    pub level: u32,
    pub can_attack: bool,
    /// Damage calculations it has been through this turn.
    pub battles: u32,
    pub counters: u32,
    /// A registered coin effect: 0 none, 1 tails, 2 heads.
    pub coin: u8,
    /// Picked so far in the selection being answered.
    pub selected: bool,
    pub attacking: bool,
    pub attacked: bool,
    /// The card of the event being responded to, such as a monster being Summoned.
    pub event: bool,
    /// The source of a link of the current chain.
    pub chained: bool,
    /// A target of a link of the current chain.
    pub targeted: bool,
}

/// A link of the current chain, oldest first.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Link {
    pub code: u32,
    pub controller: u8,
    /// Its source in [`View::cards`], or -1.
    pub source: i32,
    pub targets: Vec<i32>,
}

/// A card named by a choice.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Named {
    /// Its place in [`View::cards`], or -1 (a card in a Deck, for instance).
    pub card: i32,
    pub code: u32,
    pub controller: u8,
    pub location: u8,
    pub position: u8,
    /// Prompt-specific: a Tribute or sum value, or for an attacker 1 when it
    /// may attack directly.
    pub value: i64,
    pub required: bool,
}

/// One answer the seat may give.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Offered {
    /// What kind of answer, as a view numbers them.
    pub kind: u8,
    /// The acting or subject card.
    pub card: Option<Named>,
    /// The cards of a selection answer.
    pub members: Vec<Named>,
    /// The engine's description of the effect or option; 0 when there is none.
    pub description: u64,
}

/// A card of the event being responded to, such as a monster being Summoned.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Happening {
    /// Its place in [`View::cards`], or -1.
    pub card: i32,
    pub code: u32,
    pub controller: u8,
    pub location: u8,
}

/// One of the seat's earlier answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Past {
    pub kind: u8,
    pub code: u32,
    pub location: u8,
    pub description: u64,
}

/// What is asked.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Question {
    /// What kind of decision, as a view numbers them.
    pub kind: u8,
    /// A chain window where passing is not allowed.
    pub forced: bool,
    /// A chain window that offers only triggered effects.
    pub triggers: bool,
    /// What the selection is for, as a view numbers hints.
    pub hint: u8,
    pub minimum: u32,
    pub maximum: u32,
    /// The card the question is about, when it names one.
    pub subject: u32,
}

/// One decision of a seat.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct View {
    /// The seat that decides: 0 goes first.
    pub player: u8,
    /// The place, among all the answers of the duel, of the engine answer
    /// this decision belongs to.  It is the `decision` of a `policy-bench`
    /// row's deviations.
    pub answer: usize,
    /// The step within that answer: card selections take several.
    pub step: usize,
    /// Decisions of one seat that belong together: a command and the
    /// selections it leads to.
    pub transaction: u32,
    pub turn: u32,
    /// 0: the seat's turn, 1: the other player's, -1: not known yet.
    pub turn_player: i8,
    /// The phase, as a view numbers them; 0 before the first.
    pub phase: u8,
    /// The seat's Life Points, then the other player's.
    pub life_points: [i32; 2],
    /// Cards in the Deck, hand, Graveyard, banished and Extra Deck piles,
    /// each for the seat then the other player.
    pub piles: [[u32; 2]; 5],
    /// The turn player has made this turn's Normal Summon or Set.
    pub summon_used: bool,
    pub cards: Vec<Card>,
    pub chain: Vec<Link>,
    pub events: Vec<Happening>,
    /// The card of the command this decision follows from: the attacker
    /// whose target is asked, the card whose activation asks for a cost.
    pub acting: Option<Named>,
    /// Heads and tails of the toss the resolving effect just made.
    pub coin_toss: Option<[u8; 2]>,
    pub question: Question,
    /// The answers the seat may give.  Hand shuffling and the cancelling of a
    /// selection are left out.
    pub choices: Vec<Offered>,
    /// The answer to imitate, among `choices`: the accompanying pilot's for
    /// a caller's seat, the recorded one for a recorded seat.
    pub label: Option<usize>,
    /// The accompanying pilot's own answer, among `choices`.  It differs
    /// from `label` where a recorded duel left the pilot's answer.
    pub pilot: Option<usize>,
    /// The answer that was played, among `choices`, when the seat was not
    /// the caller's to decide.
    pub chosen: Option<usize>,
    /// The seat's own earlier answers, oldest first.
    pub history: Vec<Past>,
}
