//! What a seat keeps of its own play, and what a decision is to a network.
//!
//! A decision is shown with the seat's earlier answers and with the card of
//! the command it follows from.  A duel of the trainer keeps them for each
//! of its seats, and so does a seat at a table that speaks EDOPro's
//! protocol: the same code, so that a network is shown there what it was
//! shown when it was trained, and is offered the same answers.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

use ygo_policies::model::{Choice, ChoiceKind, Decision, DecisionKind, Member, Observation};

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
    /// The positions of the turn the seat was offered commands at, with the
    /// commands it gave there (see [`Trail::offered`]).
    given: HashMap<u64, Vec<usize>>,
    /// The turn those positions are of.
    turn: u32,
    /// The position of the decision being answered, until it is answered.
    position: Option<u64>,
}

impl Default for Trail {
    fn default() -> Self {
        Trail {
            history: VecDeque::new(),
            acting: None,
            transaction: 0,
            interrupted: true,
            given: HashMap::new(),
            turn: 0,
            position: None,
        }
    }
}

/// Where a seat stands: what it is shown of the duel and the question it is
/// asked, but not its own earlier answers nor what happened since it last
/// answered.
fn position(observation: &Observation, decision: &Decision) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{observation:?}").hash(&mut hasher);
    format!("{decision:?}").hash(&mut hasher);
    hasher.finish()
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

    /// What a network that decides for the seat is offered of `decision`:
    /// the choices of [`standing`], but for a command or an activation the
    /// seat gave at the same position earlier in the turn.  The duel came
    /// back to where it was given, so it changed nothing; offered again, a
    /// network that takes its first choice gives it again, and the duel goes
    /// round until its cap of decisions ends it.  Lyla, Lightsworn Sorceress
    /// did so under Skill Drain, which negates her effect but lets her
    /// activate it again: she turns to Defense Position only if it resolves.
    /// She did it from a command of the Main Phase, and from a chain window
    /// of it, where the seat activated her and was asked the same window
    /// again.  Passing a chain window is never what brings the seat back,
    /// and it is always offered; where every command or activation was given
    /// there already, all of them stay.
    pub fn offered(&mut self, observation: &Observation, decision: &Decision, options: &Options) -> Standing {
        let mut standing = standing(decision, options);
        if observation.turn != self.turn {
            self.given.clear();
            self.turn = observation.turn;
        }
        let window = matches!(decision.kind, DecisionKind::Chain { .. });
        let command = matches!(decision.kind, DecisionKind::Idle | DecisionKind::Battle);
        // A window with one answer has nothing to take away.
        if !(command || (window && standing.open.len() > 1)) {
            self.position = None;
            return standing;
        }
        let here = position(observation, decision);
        self.position = Some(here);
        if let Some(given) = self.given.get(&here) {
            let kept = |index: &usize| !given.contains(index) || matches!(decision.choices[*index].kind, ChoiceKind::Pass);
            let left: Vec<usize> = standing.open.iter().copied().filter(kept).collect();
            if !left.is_empty() && left.len() < standing.open.len() {
                standing.real = if options.skip_single { left.len() > 1 } else { !left.is_empty() };
                standing.open = left;
            }
        }
        standing
    }

    /// An answered step: a command's card is what the rest of its
    /// transaction is about, and a command given where [`Trail::offered`]
    /// was asked is kept for its position.
    pub fn note(&mut self, decision: &Decision, choice: usize) {
        if matches!(decision.kind, DecisionKind::Idle | DecisionKind::Battle | DecisionKind::Chain { .. }) {
            self.acting = decision.choices[choice].card;
        }
        if let Some(here) = self.position.take() {
            self.given.entry(here).or_default().push(choice);
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

#[cfg(test)]
mod tests {
    use ygo_policies::model::{Choice, ChoiceKind, Decision, DecisionKind, Hint, Observation};

    use super::*;

    fn observation(turn: u32, life: i32) -> Observation {
        Observation {
            me: 0,
            turn,
            turn_player: Some(0),
            phase: None,
            life_points: [life, 8000],
            cards: vec![],
            pile_sizes: vec![],
            chain: vec![],
            battle_attacker: None,
            battle_target: None,
            event_cards: vec![],
            summon_used: false,
            chain_known: true,
            can_attack_known: true,
            coin_toss: None,
        }
    }

    /// A Main Phase: activate, set or end the turn.
    fn main_phase() -> Decision {
        let choice = |kind| Choice { kind, card: None, members: vec![], description: 0, place: None };
        Decision {
            kind: DecisionKind::Idle,
            hint: Hint::None,
            minimum: 0,
            maximum: 0,
            selected: vec![],
            subject: None,
            choices: vec![choice(ChoiceKind::Activate), choice(ChoiceKind::SetSpellTrap), choice(ChoiceKind::EndTurn)],
        }
    }

    #[test]
    fn an_answer_that_changed_nothing_is_not_offered_again_this_turn() {
        let options = Options::default();
        let mut trail = Trail::default();
        let decision = main_phase();
        let here = observation(3, 8000);
        assert_eq!(trail.offered(&here, &decision, &options).open, vec![0, 1, 2]);
        trail.note(&decision, 0);
        // Back where it activated: the activation is not offered there.
        let again = trail.offered(&here, &decision, &options);
        assert_eq!(again.open, vec![1, 2]);
        assert!(again.real);
        trail.note(&decision, 1);
        // Two answers given there: the third alone, taken as a single answer is.
        let last = trail.offered(&here, &decision, &options);
        assert_eq!(last.open, vec![2]);
        assert!(!last.real);
        trail.note(&decision, 2);
        // Every answer given there already: all of them stay.
        assert_eq!(trail.offered(&here, &decision, &options).open, vec![0, 1, 2]);
    }

    #[test]
    fn elsewhere_or_in_another_turn_everything_is_offered() {
        let options = Options::default();
        let mut trail = Trail::default();
        let decision = main_phase();
        trail.offered(&observation(3, 8000), &decision, &options);
        trail.note(&decision, 0);
        // The activation changed the life points: another position.
        assert_eq!(trail.offered(&observation(3, 7000), &decision, &options).open, vec![0, 1, 2]);
        trail.note(&decision, 0);
        // The next turn begins with nothing given.
        assert_eq!(trail.offered(&observation(4, 7000), &decision, &options).open, vec![0, 1, 2]);
        trail.note(&decision, 0);
        assert_eq!(trail.offered(&observation(4, 7000), &decision, &options).open, vec![1, 2]);
    }

    #[test]
    fn what_the_seat_was_told_since_does_not_make_another_position() {
        let options = Options::default();
        let mut trail = Trail::default();
        let decision = main_phase();
        let here = observation(3, 8000);
        trail.offered(&here, &decision, &options);
        trail.remember(&decision.choices[0], options.history);
        trail.note(&decision, 0);
        // Its own earlier answers are longer by one: the same position.
        assert_eq!(trail.offered(&here, &decision, &options).open, vec![1, 2]);
    }

    /// A chain window: activate one of two cards, or pass.
    fn chain_window(forced: bool) -> Decision {
        let choice = |kind| Choice { kind, card: None, members: vec![], description: 0, place: None };
        let mut choices = vec![choice(ChoiceKind::Activate), choice(ChoiceKind::Activate)];
        if !forced {
            choices.push(choice(ChoiceKind::Pass));
        }
        Decision { kind: DecisionKind::Chain { forced, triggers: false }, choices, ..main_phase() }
    }

    #[test]
    fn an_activation_at_a_chain_window_that_changed_nothing_is_not_offered_there_again() {
        let options = Options::default();
        let mut trail = Trail::default();
        let window = chain_window(false);
        let here = observation(3, 8000);
        assert_eq!(trail.offered(&here, &window, &options).open, vec![0, 1, 2]);
        trail.note(&window, 0);
        // Back at the same window: the activation is not offered there.
        assert_eq!(trail.offered(&here, &window, &options).open, vec![1, 2]);
        trail.note(&window, 1);
        // Both activations given there: passing alone, taken as a single answer is.
        let last = trail.offered(&here, &window, &options);
        assert_eq!(last.open, vec![2]);
        assert!(!last.real);
        // Passing never brings the seat back: it stays, however often it was given.
        trail.note(&window, 2);
        assert_eq!(trail.offered(&here, &window, &options).open, vec![2]);
        // Where the activation changed something, everything is offered.
        assert_eq!(trail.offered(&observation(3, 7000), &window, &options).open, vec![0, 1, 2]);
    }

    #[test]
    fn a_window_that_must_be_answered_keeps_its_activations() {
        let options = Options::default();
        let mut trail = Trail::default();
        let window = chain_window(true);
        let here = observation(3, 8000);
        trail.offered(&here, &window, &options);
        trail.note(&window, 0);
        assert_eq!(trail.offered(&here, &window, &options).open, vec![1]);
        trail.note(&window, 1);
        // Every activation given there already: all of them stay.
        assert_eq!(trail.offered(&here, &window, &options).open, vec![0, 1]);
    }

    #[test]
    fn a_step_answered_without_offered_keeps_nothing() {
        let options = Options::default();
        let mut trail = Trail::default();
        let decision = main_phase();
        let here = observation(3, 8000);
        // A pilot's step: noted, never offered.
        trail.note(&decision, 0);
        assert_eq!(trail.offered(&here, &decision, &options).open, vec![0, 1, 2]);
    }
}
