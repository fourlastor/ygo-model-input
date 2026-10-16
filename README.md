# ygo-model-input

A Yu-Gi-Oh! duel decision as a neural policy's input: what a seat of
[ygo-policies](https://github.com/fourlastor/ygo-policies) is asked, as the
arrays a network reads. It is the one definition of that input, shared by the
trainer and by whatever else seats a trained checkpoint, so that a network is
shown at a table what it was shown when it was trained.

Two crates, in the order a decision goes through them:

- **`seat/` (`ygo-model-seat`)** turns what a seat is asked into a view, and
  is the seat's policy.
  - `view`: the seat's `Observation` and `Decision` as a `View`, plain data
    with what the seat may see and nothing else.
  - `trail`: what a seat keeps of its own play (its earlier answers, the card
    of the command a selection follows from), and which decisions are a
    network's to take.
  - `caller`: a `Policy` that hands those decisions to a closure. What a
    network is not asked is answered as in training: the deck's pilot
    chooses the zone a card goes to, and a decision with a single answer
    takes it. Shuffling the hand and cancelling a selection are never
    offered, nor is a command of the Main or the Battle Phase that the seat
    gave at the same position earlier in the turn, which brought it back
    there.
- **`input/` (`ygo-model-input`)** turns a view into arrays.
  - `view`: the `View` itself.
  - `features`: what is printed on a card, from its row of the card database.
  - `pack`: a view as named arrays, and decisions padded into one batch.

What a network makes of the arrays is not here.

## Building

`input` depends on `serde` alone. It builds and tests on its own, and for
WebAssembly:

```sh
cd input && cargo test
```

`seat` names the types of `ygo-policies`, so it builds inside a project that
holds them. There is no workspace here: both crates are members of the
workspace of the project that mounts this repository, and `seat` takes the
policies from it.

```toml
[workspace]
members = ["...", "path/to/ygo-model-input/input", "path/to/ygo-model-input/seat"]

[workspace.dependencies]
ygo-policies = { path = "path/to/ygo-policies/crates/ygo-policies" }
ygo-policies-ocgcore = { path = "path/to/ygo-policies/crates/ygo-policies-ocgcore", default-features = false }
```

Without its default features `ygo-policies-ocgcore` leaves SQLite out, which
a browser cannot build; a project that reads a card database asks for them
where it does.

## Seating a network

```rust
use ygo_model_input::features::Features;
use ygo_model_input::pack::{batch, pack};
use ygo_model_seat::{Caller, Options, Progress, View};
use ygo_policies_ocgcore::Seat;

// The rows of the card database's `datas` table:
// id, type, atk, def, level, race, attribute, setcode.
let features = Features::from_rows(rows);

let progress = Progress::default();
let policy = Caller::new(pilot, Options::default(), progress.clone(), move |view: View| {
    let packed = pack(&view, &features).ok()?;
    let arrays = batch(&[&packed]).ok()?;
    // The network: one score an answer, and the answer it takes is a place
    // among `view.choices`. Without an answer the pilot's stands in.
    Some(first_choice(&arrays))
});
// `database` is the policies' own view of the cards (`CardDatabase`).
let mut seat = Seat::new(Box::new(policy), database, None);

// Every message of the duel the seat is sent, and every answer it gives:
progress.received(&message);
if let Some(response) = seat.feed(&message)? {
    progress.answered();
    send(response);
}
```

`pilot` is the policy of the seat's deck (`ygo_policies::registry`). A new
duel is a new seat and `progress.restart()`.

`progress.received` reads two things from the messages that a duel's runner
would know: that a selection begins, and that the other seat was asked one,
which a table that speaks EDOPro's protocol tells a client with
`MSG_WAITING`. A table that does not send it shows a network a duel it was
not trained on.

## What a checkpoint needs to be shown what it was trained on

- The engine, the card scripts and the card database of its training. A
  script decides what a question offers and what an answer is called, and
  both are in the input.
- The `ygo-policies` of its training: what a seat may see is decided there.
- This repository at the revision of its training. The numbers a view names
  locations, positions, phases, hints and kinds by (`seat/src/view.rs`) are
  what the network's embeddings are indexed by, and the arrays are its input
  layer.

## Changing it

The input is defined here: the trainer builds every array its network reads
with these crates and has no packing of its own. `input/vectors/decisions.json`
holds what they make of some 130 decisions of the trainer's duels, written by
its `tools/model_input_vectors.py`, and `input/tests/vectors.rs` holds `pack`
and `batch` to that, array for array, so that an input changed by mistake
fails there (and in the trainer's tests). A change of the input is made here,
with the vectors written again, and a network trained before the change needs
the revision before it.
