//! The seat of a caller: a policy whose decisions something outside takes.
//!
//! A seat of `ygo-policies` asks its [`Policy`] at every step of every
//! selection.  The policy here is the caller of a duel of the trainer: what
//! a network is not asked there is answered the same way (the deck's pilot
//! places the cards, a single answer is taken), and what it is asked is
//! shown as a [`View`], with the seat's own trail through the duel.
//!
//! Two things a duel's runner knows are read from the messages the seat is
//! fed: that a selection begins, and that the other seat was asked one,
//! which a table that speaks EDOPro's protocol tells a client with
//! `MSG_WAITING`.  Whoever feeds the seat tells its [`Progress`] of every
//! message, and of every answer the seat gives.

use std::sync::{Arc, Mutex, MutexGuard};

use ygo_policies::agent::Policy;
use ygo_policies::model::{Decision, Observation};
use ygo_policies_ocgcore::message::is_selection;
use ygo_policies_ocgcore::wire::msg;

use crate::trail::Trail;
use crate::view::{self, View};
use crate::Options;

/// What the seat's policy and whoever feeds the seat share of a duel.
#[derive(Default)]
struct Shared {
    trail: Trail,
    /// A selection has arrived and none of its steps is answered yet.
    fresh: bool,
    /// The steps answered of the selection being answered.
    step: usize,
    /// Selections the seat has answered.
    answers: usize,
    /// Decisions the caller took.
    decisions: usize,
}

/// Where a caller's seat stands in its duel.  A clone is the same standing:
/// one is given to the seat's [`Caller`], the other kept by whoever feeds
/// the seat its messages.
#[derive(Clone, Default)]
pub struct Progress(Arc<Mutex<Shared>>);

impl Progress {
    fn shared(&self) -> MutexGuard<'_, Shared> {
        self.0.lock().expect("the seat's trail is not poisoned")
    }

    /// A message of the duel, as the seat is fed it.
    pub fn received(&self, message: &[u8]) {
        let mut shared = self.shared();
        match message.first() {
            // The other seat is asked a selection, which it answers before
            // this one is asked again.
            Some(&msg::WAITING) => shared.trail.interrupt(),
            // A client is sent its own prompts only.
            Some(&id) if is_selection(id) => shared.fresh = true,
            _ => {}
        }
    }

    /// The seat answered a selection.
    pub fn answered(&self) {
        self.shared().answers += 1;
    }

    /// Decisions the caller took.
    pub fn decisions(&self) -> usize {
        self.shared().decisions
    }

    /// A new duel: nothing is kept of the one before.
    pub fn restart(&self) {
        *self.shared() = Shared::default();
    }
}

/// The policy of a caller's seat, with the deck's pilot asked alongside.
/// `ask` is shown each decision a network takes and answers with one of the
/// view's choices; without an answer the pilot's stands in.
pub struct Caller<A> {
    pilot: Box<dyn Policy>,
    options: Options,
    progress: Progress,
    ask: A,
}

impl<A: FnMut(View) -> Option<usize> + Send> Caller<A> {
    pub fn new(pilot: Box<dyn Policy>, options: Options, progress: Progress, ask: A) -> Self {
        Caller { pilot, options, progress, ask }
    }
}

impl<A: FnMut(View) -> Option<usize> + Send> Policy for Caller<A> {
    fn choose(&mut self, observation: &Observation, decision: &Decision) -> usize {
        let standing = self.progress.shared().trail.offered(observation, decision, &self.options);
        let label = self.pilot.choose(observation, decision).min(decision.choices.len() - 1);
        let mut choice = if standing.placement { label } else { standing.open.first().copied().unwrap_or(0) };
        // Nothing is held while the caller is asked: it may look at the
        // seat's progress, and it may take its time.
        let shown = {
            let mut shared = self.progress.shared();
            if std::mem::take(&mut shared.fresh) {
                shared.trail.asked(decision);
                shared.step = 0;
            }
            standing.real.then(|| {
                let mut shown = view::view(observation, decision, &standing.open);
                shared.trail.stamp(&mut shown, observation, shared.step, shared.answers);
                shown.label = standing.open.iter().position(|index| *index == label);
                shown.pilot = shown.label;
                shown
            })
        };
        let taken = shown.map(|shown| (self.ask)(shown).and_then(|index| standing.open.get(index).copied()));
        let mut shared = self.progress.shared();
        match taken {
            Some(Some(taken)) => {
                choice = taken;
                shared.trail.remember(&decision.choices[choice], self.options.history);
                shared.decisions += 1;
            }
            // The caller gave no answer: the pilot's stands in for it.
            Some(None) => choice = label,
            None => {}
        }
        shared.trail.note(decision, choice);
        shared.step += 1;
        choice
    }
}
