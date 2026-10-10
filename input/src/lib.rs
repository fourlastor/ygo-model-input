//! A Yu-Gi-Oh! duel decision as the arrays a neural policy reads.
//!
//! * [`view`]: a decision of one seat as plain data, with what the seat may
//!   see and nothing else;
//! * [`features`]: what is printed on a card, as the policy reads it;
//! * [`pack`]: a view as named arrays, and decisions padded into a batch.
//!
//! Nothing here knows the engine or the pilots, and nothing reads a file:
//! `ygo-model-seat` makes a view from what a seat of `ygo-policies` is
//! asked, and whoever holds a card database gives its rows.  What a
//! network makes of the arrays is not here either.

pub mod features;
pub mod pack;
pub mod view;

/// Everything here fails with a message.
pub type Result<T> = std::result::Result<T, String>;
