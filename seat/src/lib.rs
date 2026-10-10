//! A seat of `ygo-policies` that a network decides for.
//!
//! * [`view`]: what the seat is asked, as the plain data `ygo-model-input`
//!   makes arrays of;
//! * [`trail`]: what a seat keeps of its own play, and which decisions are
//!   a network's to take;
//! * [`caller`]: the seat's policy, which hands those decisions to whatever
//!   answers them.
//!
//! This crate names the types of `ygo-policies` and so builds inside a
//! project that holds them: it takes `ygo-policies` and
//! `ygo-policies-ocgcore` from the `[workspace.dependencies]` of the
//! workspace it is a member of.

use serde::Deserialize;

pub mod caller;
pub mod trail;
pub mod view;

pub use crate::caller::{Caller, Progress};
pub use crate::trail::{standing, Standing, Trail};
pub use crate::view::View;

/// What is kept from a network.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Options {
    /// The zone a card goes to is the pilot's choice.
    pub delegate_placement: bool,
    /// A decision with a single answer takes it.
    pub skip_single: bool,
    /// How many of its own earlier answers a seat is shown.
    pub history: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { delegate_placement: true, skip_single: true, history: 32 }
    }
}
