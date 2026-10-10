//! What a seat keeps of its own play, and what a decision is to a network.
//!
//! A decision is shown with the seat's earlier answers and with the card of
//! the command it follows from.  A duel of the trainer keeps them for each
//! of its seats, and so does a seat at a table that speaks EDOPro's
//! protocol: the same code, so that a network is shown there what it was
//! shown when it was trained.

use std::collections::VecDeque;

use ygo_policies::model::{Choice, Decision, DecisionKind, Member, Observation};

use crate::Options;
use crate::view::{self, Past, View};

/// What a step is to a network.
pub struct Standing {
    /// The choices a network may take.
    pub open: Vec<usize>,
    /// The zone a card goes to: the pilot's choice.
    pub placement: bool,
    /// A decision to take, and to learn from.
    pub real: bool,
}

pub fn standing(decision: &Decision, options: &Options) -> Standing {
    let open = view::open_choices(decision);
    let placement = options.delegate_placement && decision.kind == DecisionKind::Place;
    let enough = if options.skip_single { open.len() > 1 } else { !open.is_empty() };
    Standing { real: !placement && enough, open, placement }
}

/// A seat's own trail through a duel.
pub struct Trail {
    /// The seat's own earlier answers, oldest first.
    history: VecDeque<Past>,
    /// The card of the command the current transaction began with.
    acting: Option<Member>,
    transaction: u32,
    /// The other seat answered since this one last decided.
    interrupted: bool,
}

impl Default for Trail {
    fn default() -> Self {
        Trail { history: VecDeque::new(), acting: None, transaction: 0, interrupted: true }
    }
}

impl Trail {
    /// The seat is asked a selection, whose first step is `decision`.  A
    /// command opens a transaction; what it leads to stays in it until the
    /// other seat has had something to answer.
    pub fn asked(&mut self, decision: &Decision) {
        let command = matches!(decision.kind, DecisionKind::Idle | DecisionKind::Battle | DecisionKind::Chain { .. });
        if command || self.interrupted {
            self.transaction += 1;
            self.acting = None;
        }
        self.interrupted = false;
    }

    /// The other seat answered a selection.
    pub fn interrupt(&mut self) {
        self.interrupted = true;
    }

    /// Mark a view with where it stands in the duel: `decisions` answers
    /// were given before it, and it is step `step` of its own.
    pub fn stamp(&self, shown: &mut View, observation: &Observation, step: usize, decisions: usize) {
        shown.answer = decisions;
        shown.step = step;
        shown.transaction = self.transaction;
        shown.history = self.history.iter().cloned().collect();
        shown.acting = self.acting.as_ref().map(|member| view::named_now(observation, member));
    }

    /// An answered step: a command's card is what the rest of its
    /// transaction is about.
    pub fn note(&mut self, decision: &Decision, choice: usize) {
        if matches!(decision.kind, DecisionKind::Idle | DecisionKind::Battle | DecisionKind::Chain { .. }) {
            self.acting = decision.choices[choice].card;
        }
    }

    /// Keep a real decision's answer in the seat's history, of which the
    /// last `kept` are shown.
    pub fn remember(&mut self, choice: &Choice, kept: usize) {
        self.history.push_back(view::past(choice));
        while self.history.len() > kept {
            self.history.pop_front();
        }
    }
}
