//! A decision as a view: what a seat of `ygo-policies` is asked, as the
//! plain data of `ygo-model-input`.
//!
//! Everything here is derived from the [`Observation`] and the [`Decision`]
//! a pilot would be given at the same point, so a network is shown what the
//! seat may see and nothing else.  The numbers a view names locations,
//! positions, phases, hints and kinds by are given here: they are what a
//! trained network's embeddings are indexed by, and none of them may move.

use ygo_policies::model::{
    CardRef, Choice, ChoiceKind, Coin, Decision, DecisionKind, Hint, Location, Member, Observation, Phase, Position,
};

pub use ygo_model_input::view::{Card, Happening, Link, Named, Offered, Past, Question, View};

pub fn location(location: Location) -> u8 {
    match location {
        Location::Deck => 1,
        Location::Hand => 2,
        Location::MonsterZone => 3,
        Location::SpellTrapZone => 4,
        Location::Graveyard => 5,
        Location::Banished => 6,
        Location::Extra => 7,
        Location::Overlay => 8,
        Location::Other => 9,
    }
}

/// OCGCore's own position bits.
pub fn position(position: Position) -> u8 {
    match (position.face_up, position.attack) {
        (true, true) => 1,
        (false, true) => 2,
        (true, false) => 4,
        (false, false) => 8,
    }
}

pub fn phase(phase: Option<Phase>) -> u8 {
    match phase {
        None => 0,
        Some(Phase::Draw) => 1,
        Some(Phase::Standby) => 2,
        Some(Phase::Main1) => 3,
        Some(Phase::BattleStart) => 4,
        Some(Phase::BattleStep) => 5,
        Some(Phase::Damage) => 6,
        Some(Phase::DamageCalculation) => 7,
        Some(Phase::Battle) => 8,
        Some(Phase::Main2) => 9,
        Some(Phase::End) => 10,
    }
}

pub fn decision_kind(kind: DecisionKind) -> u8 {
    match kind {
        DecisionKind::Idle => 1,
        DecisionKind::Battle => 2,
        DecisionKind::Chain { .. } => 3,
        DecisionKind::YesNo => 4,
        DecisionKind::Option => 5,
        DecisionKind::Position => 6,
        DecisionKind::SelectCards => 7,
        DecisionKind::SelectToggle => 8,
        DecisionKind::SelectSum => 9,
        DecisionKind::Place => 10,
        DecisionKind::Sort => 11,
        DecisionKind::Announce => 12,
        DecisionKind::Counter => 13,
        DecisionKind::Other => 14,
    }
}

pub fn choice_kind(kind: ChoiceKind) -> u8 {
    match kind {
        ChoiceKind::NormalSummon => 1,
        ChoiceKind::SpecialSummon => 2,
        ChoiceKind::ChangePosition => 3,
        ChoiceKind::SetMonster => 4,
        ChoiceKind::SetSpellTrap => 5,
        ChoiceKind::Activate => 6,
        ChoiceKind::Attack => 7,
        ChoiceKind::EnterBattle => 8,
        ChoiceKind::EnterMain2 => 9,
        ChoiceKind::EndTurn => 10,
        ChoiceKind::ShuffleHand => 11,
        ChoiceKind::Pass => 12,
        ChoiceKind::Yes => 13,
        ChoiceKind::No => 14,
        ChoiceKind::Option => 15,
        ChoiceKind::Position(Position { face_up: true, attack: true }) => 16,
        ChoiceKind::Position(Position { face_up: false, attack: true }) => 17,
        ChoiceKind::Position(Position { face_up: true, attack: false }) => 18,
        ChoiceKind::Position(Position { face_up: false, attack: false }) => 19,
        ChoiceKind::Cards => 20,
        ChoiceKind::Toggle => 21,
        ChoiceKind::Finish => 22,
        ChoiceKind::Cancel => 23,
        ChoiceKind::Place => 24,
        ChoiceKind::Sort => 25,
        ChoiceKind::Announce => 26,
        ChoiceKind::Counter => 27,
        ChoiceKind::Other => 28,
    }
}

pub fn hint(hint: Hint) -> u8 {
    match hint {
        Hint::None => 0,
        Hint::Release => 1,
        Hint::Discard => 2,
        Hint::Destroy => 3,
        Hint::Banish => 4,
        Hint::ToGraveyard => 5,
        Hint::ReturnToHand => 6,
        Hint::AddToHand => 7,
        Hint::ToDeck => 8,
        Hint::Summon => 9,
        Hint::SpecialSummon => 10,
        Hint::Set => 11,
        Hint::FusionMaterial => 12,
        Hint::SynchroMaterial => 13,
        Hint::XyzMaterial => 14,
        Hint::Tribute => 15,
        Hint::FaceUp => 16,
        Hint::FaceDown => 17,
        Hint::Attack => 18,
        Hint::Defense => 19,
        Hint::Equip => 20,
        Hint::Control => 21,
        Hint::AttackTarget => 22,
        Hint::Target => 23,
        Hint::Effect => 24,
        Hint::Confirm => 25,
        Hint::Other(_) => 26,
    }
}

/// The names behind the numbers above, for tables and reports: `[name,
/// number]` pairs for each vocabulary.
pub fn vocabulary() -> serde_json::Value {
    let named = |pairs: &[(&str, u8)]| pairs.iter().map(|(name, id)| serde_json::json!([name, id])).collect::<Vec<_>>();
    let up = |attack| Position { face_up: true, attack };
    let down = |attack| Position { face_up: false, attack };
    serde_json::json!({
        "location": named(&[
            ("Deck", location(Location::Deck)), ("Hand", location(Location::Hand)),
            ("MonsterZone", location(Location::MonsterZone)), ("SpellTrapZone", location(Location::SpellTrapZone)),
            ("Graveyard", location(Location::Graveyard)), ("Banished", location(Location::Banished)),
            ("Extra", location(Location::Extra)), ("Overlay", location(Location::Overlay)), ("Other", location(Location::Other)),
        ]),
        "position": named(&[
            ("FaceUpAttack", position(up(true))), ("FaceDownAttack", position(down(true))),
            ("FaceUpDefense", position(up(false))), ("FaceDownDefense", position(down(false))),
        ]),
        "phase": named(&[
            ("Draw", phase(Some(Phase::Draw))), ("Standby", phase(Some(Phase::Standby))), ("Main1", phase(Some(Phase::Main1))),
            ("BattleStart", phase(Some(Phase::BattleStart))), ("BattleStep", phase(Some(Phase::BattleStep))),
            ("Damage", phase(Some(Phase::Damage))), ("DamageCalculation", phase(Some(Phase::DamageCalculation))),
            ("Battle", phase(Some(Phase::Battle))), ("Main2", phase(Some(Phase::Main2))), ("End", phase(Some(Phase::End))),
        ]),
        "decision": named(&[
            ("Idle", decision_kind(DecisionKind::Idle)), ("Battle", decision_kind(DecisionKind::Battle)),
            ("Chain", decision_kind(DecisionKind::Chain { forced: false, triggers: false })),
            ("YesNo", decision_kind(DecisionKind::YesNo)), ("Option", decision_kind(DecisionKind::Option)),
            ("Position", decision_kind(DecisionKind::Position)), ("SelectCards", decision_kind(DecisionKind::SelectCards)),
            ("SelectToggle", decision_kind(DecisionKind::SelectToggle)), ("SelectSum", decision_kind(DecisionKind::SelectSum)),
            ("Place", decision_kind(DecisionKind::Place)), ("Sort", decision_kind(DecisionKind::Sort)),
            ("Announce", decision_kind(DecisionKind::Announce)), ("Counter", decision_kind(DecisionKind::Counter)),
            ("Other", decision_kind(DecisionKind::Other)),
        ]),
        "choice": named(&[
            ("NormalSummon", choice_kind(ChoiceKind::NormalSummon)), ("SpecialSummon", choice_kind(ChoiceKind::SpecialSummon)),
            ("ChangePosition", choice_kind(ChoiceKind::ChangePosition)), ("SetMonster", choice_kind(ChoiceKind::SetMonster)),
            ("SetSpellTrap", choice_kind(ChoiceKind::SetSpellTrap)), ("Activate", choice_kind(ChoiceKind::Activate)),
            ("Attack", choice_kind(ChoiceKind::Attack)), ("EnterBattle", choice_kind(ChoiceKind::EnterBattle)),
            ("EnterMain2", choice_kind(ChoiceKind::EnterMain2)), ("EndTurn", choice_kind(ChoiceKind::EndTurn)),
            ("ShuffleHand", choice_kind(ChoiceKind::ShuffleHand)), ("Pass", choice_kind(ChoiceKind::Pass)),
            ("Yes", choice_kind(ChoiceKind::Yes)), ("No", choice_kind(ChoiceKind::No)), ("Option", choice_kind(ChoiceKind::Option)),
            ("PositionFaceUpAttack", choice_kind(ChoiceKind::Position(up(true)))),
            ("PositionFaceDownAttack", choice_kind(ChoiceKind::Position(down(true)))),
            ("PositionFaceUpDefense", choice_kind(ChoiceKind::Position(up(false)))),
            ("PositionFaceDownDefense", choice_kind(ChoiceKind::Position(down(false)))),
            ("Cards", choice_kind(ChoiceKind::Cards)), ("Toggle", choice_kind(ChoiceKind::Toggle)),
            ("Finish", choice_kind(ChoiceKind::Finish)), ("Cancel", choice_kind(ChoiceKind::Cancel)),
            ("Place", choice_kind(ChoiceKind::Place)), ("Sort", choice_kind(ChoiceKind::Sort)),
            ("Announce", choice_kind(ChoiceKind::Announce)), ("Counter", choice_kind(ChoiceKind::Counter)),
            ("Other", choice_kind(ChoiceKind::Other)),
        ]),
        "hint": named(&[
            ("None", hint(Hint::None)), ("Release", hint(Hint::Release)), ("Discard", hint(Hint::Discard)),
            ("Destroy", hint(Hint::Destroy)), ("Banish", hint(Hint::Banish)), ("ToGraveyard", hint(Hint::ToGraveyard)),
            ("ReturnToHand", hint(Hint::ReturnToHand)), ("AddToHand", hint(Hint::AddToHand)), ("ToDeck", hint(Hint::ToDeck)),
            ("Summon", hint(Hint::Summon)), ("SpecialSummon", hint(Hint::SpecialSummon)), ("Set", hint(Hint::Set)),
            ("FusionMaterial", hint(Hint::FusionMaterial)), ("SynchroMaterial", hint(Hint::SynchroMaterial)),
            ("XyzMaterial", hint(Hint::XyzMaterial)), ("Tribute", hint(Hint::Tribute)), ("FaceUp", hint(Hint::FaceUp)),
            ("FaceDown", hint(Hint::FaceDown)), ("Attack", hint(Hint::Attack)), ("Defense", hint(Hint::Defense)),
            ("Equip", hint(Hint::Equip)), ("Control", hint(Hint::Control)), ("AttackTarget", hint(Hint::AttackTarget)),
            ("Target", hint(Hint::Target)), ("Effect", hint(Hint::Effect)), ("Confirm", hint(Hint::Confirm)),
            ("Other", hint(Hint::Other(0))),
        ]),
    })
}

/// The choices of `decision` a network may take: every one but hand
/// shuffling, which changes nothing, and the cancelling of a selection,
/// which only goes back.  Passing a chain window, answering no and
/// finishing a selection stay.
pub fn open_choices(decision: &Decision) -> Vec<usize> {
    decision
        .choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| !matches!(choice.kind, ChoiceKind::ShuffleHand | ChoiceKind::Cancel))
        .map(|(index, _)| index)
        .collect()
}

/// The entry an answer leaves in the seat's history.
pub fn past(choice: &Choice) -> Past {
    Past {
        kind: choice_kind(choice.kind),
        code: choice.code().unwrap_or(0),
        location: choice.at().map_or(0, |at| location(at.location)),
        description: choice.description,
    }
}

/// A card named earlier, as `observation` shows it now: it keeps its place
/// in [`View::cards`] only while the same card still lies where it was named.
pub fn named_now(observation: &Observation, member: &Member) -> Named {
    let found = observation.cards.iter().position(|card| card.at == member.at && card.code == member.code);
    Named {
        card: found.map_or(-1, |index| index as i32),
        code: member.code.unwrap_or(0),
        controller: u8::from(member.at.controller != observation.me),
        location: location(member.at.location),
        position: found.map_or(0, |index| {
            let card = &observation.cards[index];
            if matches!(card.at.location, Location::MonsterZone | Location::SpellTrapZone) {
                position(card.position)
            } else {
                0
            }
        }),
        value: member.value,
        required: member.required,
    }
}

/// What `observation` and `decision` show, for the seat `observation.me`.
/// `open` are the choices to offer (see [`open_choices`]).
pub fn view(observation: &Observation, decision: &Decision, open: &[usize]) -> View {
    let me = observation.me;
    let relative = |player: u8| u8::from(player != me);
    let place = |at: CardRef| observation.cards.iter().position(|card| card.at == at).map_or(-1, |index| index as i32);
    let on_field = |location: Location| matches!(location, Location::MonsterZone | Location::SpellTrapZone);
    let mut cards: Vec<Card> = observation
        .cards
        .iter()
        .map(|card| Card {
            controller: relative(card.at.controller),
            location: location(card.at.location),
            code: card.code.unwrap_or(0),
            position: if on_field(card.at.location) { position(card.position) } else { 0 },
            attack: card.attack,
            defense: card.defense,
            level: card.level,
            can_attack: card.can_attack,
            battles: card.battles,
            counters: card.counters,
            coin: match card.coin_effect.map(|effect| effect.result) {
                None => 0,
                Some(Coin::Tails) => 1,
                Some(Coin::Heads) => 2,
            },
            selected: decision.selected.contains(&card.at),
            attacking: observation.battle_attacker == Some(card.at),
            attacked: observation.battle_target == Some(card.at),
            event: observation.event_cards.iter().any(|(at, _)| *at == card.at),
            chained: false,
            targeted: false,
        })
        .collect();
    let chain: Vec<Link> = observation
        .chain
        .iter()
        .map(|link| Link {
            code: link.code,
            controller: relative(link.controller),
            source: place(link.source),
            targets: link.targets.iter().map(|target| place(*target)).collect(),
        })
        .collect();
    for link in &chain {
        if let Some(card) = cards.get_mut(link.source as usize) {
            card.chained = true;
        }
        for target in &link.targets {
            if let Some(card) = cards.get_mut(*target as usize) {
                card.targeted = true;
            }
        }
    }
    let named = |member: &Member| {
        let index = place(member.at);
        Named {
            card: index,
            code: member.code.unwrap_or(0),
            controller: relative(member.at.controller),
            location: location(member.at.location),
            position: cards.get(index as usize).map_or(0, |card| card.position),
            value: member.value,
            required: member.required,
        }
    };
    let choices = open
        .iter()
        .map(|index| {
            let choice = &decision.choices[*index];
            Offered {
                kind: choice_kind(choice.kind),
                card: choice.card.as_ref().map(named),
                members: choice.members.iter().map(named).collect(),
                description: choice.description,
            }
        })
        .collect();
    let (forced, triggers) = match decision.kind {
        DecisionKind::Chain { forced, triggers } => (forced, triggers),
        _ => (false, false),
    };
    let pile = |location: Location| [observation.pile_size(me, location), observation.pile_size(1 - me, location)];
    View {
        player: me,
        answer: 0,
        step: 0,
        transaction: 0,
        turn: observation.turn,
        turn_player: observation.turn_player.map_or(-1, |player| relative(player) as i8),
        phase: phase(observation.phase),
        life_points: [observation.life_points[me as usize], observation.life_points[1 - me as usize]],
        piles: [
            pile(Location::Deck),
            pile(Location::Hand),
            pile(Location::Graveyard),
            pile(Location::Banished),
            pile(Location::Extra),
        ],
        summon_used: observation.summon_used,
        chain,
        events: observation
            .event_cards
            .iter()
            .map(|(at, code)| Happening {
                card: place(*at),
                code: code.unwrap_or(0),
                controller: relative(at.controller),
                location: location(at.location),
            })
            .collect(),
        coin_toss: observation.coin_toss.as_ref().map(|toss| {
            let heads = toss.results.iter().filter(|coin| **coin == Coin::Heads).count() as u8;
            [heads, toss.results.len() as u8 - heads]
        }),
        question: Question {
            kind: decision_kind(decision.kind),
            forced,
            triggers,
            hint: hint(decision.hint),
            minimum: decision.minimum,
            maximum: decision.maximum,
            subject: decision.subject.unwrap_or(0),
        },
        choices,
        acting: None,
        label: None,
        pilot: None,
        chosen: None,
        history: Vec::new(),
        cards,
    }
}
