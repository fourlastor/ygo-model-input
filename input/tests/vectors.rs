//! The arrays held to what they were made.
//!
//! `vectors/decisions.json` is written with this crate by the trainer's
//! `tools/model_input_vectors.py`, when the input is changed on purpose:
//! decisions of real duels and a few made by hand, each as a view, with the
//! name, the type, the shape and a digest of the bytes of every array made
//! of it, alone and with all the others in one batch.  What this crate
//! makes of the same views now is compared with it: an input changed by
//! mistake fails here.

use serde::Deserialize;
use ygo_model_input::features::Features;
use ygo_model_input::pack::{batch, pack, Array, Data, Packed};
use ygo_model_input::view::View;

/// An array as the vectors tell it: name, type, shape, digest.
type Told = (String, String, Vec<usize>, String);

#[derive(Deserialize)]
struct Decision {
    view: View,
    arrays: Vec<Told>,
}

#[derive(Deserialize)]
struct Vectors {
    cards: Vec<[i64; 8]>,
    decisions: Vec<Decision>,
    batch: Vec<Told>,
}

/// FNV-1a, 64 bits, of an array's bytes as NumPy keeps them.
fn digest(data: &Data) -> String {
    let bytes: Vec<u8> = match data {
        Data::I64(values) => values.iter().flat_map(|value| value.to_le_bytes()).collect(),
        Data::F32(values) => values.iter().flat_map(|value| value.to_le_bytes()).collect(),
        Data::Bool(values) => values.clone(),
    };
    let found = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |found, byte| {
        (found ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{found:016x}")
}

fn told(name: &str, array: &Array) -> Told {
    (name.to_owned(), array.data.kind().to_owned(), array.shape.clone(), digest(&array.data))
}

struct Reference {
    vectors: Vectors,
    features: Features,
}

fn reference() -> Reference {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/vectors/decisions.json");
    let text = std::fs::read_to_string(path).expect("the vectors are in the repository");
    let vectors: Vectors = serde_json::from_str(&text).expect("the vectors are of this crate's view");
    let features = Features::from_rows(vectors.cards.iter().copied());
    Reference { vectors, features }
}

impl Reference {
    fn packed(&self, view: &View) -> Packed {
        pack(view, &self.features).expect("a decision of the vectors has answers")
    }
}

#[test]
fn every_decision_gives_the_arrays_it_gave() {
    let reference = reference();
    assert!(reference.vectors.decisions.len() > 100);
    for (index, decision) in reference.vectors.decisions.iter().enumerate() {
        let packed = reference.packed(&decision.view);
        let ours: Vec<Told> = packed.arrays.iter().map(|(name, array)| told(name, array)).collect();
        for (mine, theirs) in ours.iter().zip(&decision.arrays) {
            assert_eq!(mine, theirs, "decision {index}");
        }
        assert_eq!(ours.len(), decision.arrays.len(), "decision {index}: other arrays");
    }
}

#[test]
fn the_decisions_together_give_the_batch_they_gave() {
    let reference = reference();
    let packed: Vec<Packed> =
        reference.vectors.decisions.iter().map(|decision| reference.packed(&decision.view)).collect();
    let together = batch(&packed.iter().collect::<Vec<_>>()).expect("the decisions make a batch");
    let ours: Vec<Told> = together.arrays.iter().map(|(name, array)| told(name, array)).collect();
    for (mine, theirs) in ours.iter().zip(&reference.vectors.batch) {
        assert_eq!(mine, theirs);
    }
    assert_eq!(ours.len(), reference.vectors.batch.len(), "other arrays");
}

/// What the tests above rest on: another decision is told apart.
#[test]
fn the_comparison_sees_a_difference() {
    let reference = reference();
    let decision = &reference.vectors.decisions[0];
    let mut later = decision.view.clone();
    later.turn += 1;
    let packed = reference.packed(&later);
    let differing: Vec<&str> = packed
        .arrays
        .iter()
        .zip(&decision.arrays)
        .filter(|((name, array), theirs)| told(name, array) != **theirs)
        .map(|((name, _), _)| name.as_str())
        .collect();
    assert_eq!(differing, ["scalars"]);
}
