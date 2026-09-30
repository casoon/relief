//! Kosten der Grenze auf den Aufnahmen in `spike/recordings`
//! (→ `plan/spezifikation/02`, „Bridge-Messung“).
//!
//! Eingaben, jeweils aus einem Aufnahmepaar:
//! - `typisch`: Delta mit dem mittleren JSON-Umfang aller Paare,
//! - `groesste`: Delta mit dem größten JSON-Umfang,
//! - `*_voll`: der ganze Baum „nachher“ desselben Paars als Delta auf einen
//!   leeren Graphen (Vollsnapshot bzw. Neuaufbau).
//!
//! Je Eingabe:
//! - `a_fuellen`: Modell → `cxx`-Strukturen (Ersatz für das Füllen im
//!   C++-Adapter, das hier nicht messbar ist),
//! - `a_uebernehmen`: `cxx`-Strukturen → Modell (Rust-Seite von Variante A
//!   und B),
//! - `b_json_serialisieren` / `b_json_deserialisieren`: Obergrenze für eine
//!   Serialisierung je Update (Mojo selbst ist nur im Fork messbar),
//! - `kopie`: tiefe Kopie der Delta, Untergrenze jeder Serialisierung,
//! - `anwenden`: `Runtime::apply` (bei `*_voll`: Neuaufbau des Modells).
//!
//! Dazu `interaction_neuaufbau`: `relief_interaction::Graph::build` auf dem
//! Modell „nachher“, heute der einzige Weg, den Interaction Graph zu
//! aktualisieren.

#[path = "../tests/common/mod.rs"]
mod common;

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use relief_bridge::{delta_from_ffi, delta_to_ffi, Runtime};
use relief_model::{SemanticGraph, TreeDelta};

struct Input {
    name: &'static str,
    base: SemanticGraph,
    delta: TreeDelta,
}

fn inputs() -> (Vec<Input>, Vec<(&'static str, common::Pair)>) {
    let mut pairs: Vec<(usize, common::Pair)> = common::pairs()
        .into_iter()
        .map(|p| {
            let size = serde_json::to_vec(&TreeDelta::between(&p.before, &p.after))
                .unwrap()
                .len();
            (size, p)
        })
        .collect();
    pairs.sort_by_key(|(size, _)| *size);
    let largest = pairs.pop().unwrap().1;
    let typical = pairs.swap_remove(pairs.len() / 2).1;

    let mut out = Vec::new();
    for (name, full_name, pair) in [
        ("typisch", "typisch_voll", &typical),
        ("groesste", "groesste_voll", &largest),
    ] {
        out.push(Input {
            name,
            base: pair.before.clone(),
            delta: TreeDelta::between(&pair.before, &pair.after),
        });
        out.push(Input {
            name: full_name,
            base: SemanticGraph::default(),
            delta: TreeDelta::between(&SemanticGraph::default(), &pair.after),
        });
    }
    for input in &out {
        let nodes: usize = input
            .delta
            .trees
            .iter()
            .map(|u| u.created.len() + u.changed.len())
            .sum();
        let removed: usize = input.delta.trees.iter().map(|u| u.removed.len()).sum();
        eprintln!(
            "{:<14} Basis {:>5} Knoten, Delta {:>5} Knoten neu/geändert, {:>4} entfernt, {:>7} Byte JSON",
            input.name,
            input.base.len(),
            nodes,
            removed,
            serde_json::to_vec(&input.delta).unwrap().len()
        );
    }
    eprintln!("typisch:  {}", typical.name);
    eprintln!("groesste: {}", largest.name);
    (out, vec![("typisch", typical), ("groesste", largest)])
}

fn bench(c: &mut Criterion) {
    let (inputs, pairs) = inputs();

    for input in &inputs {
        let mut g = c.benchmark_group(format!("delta/{}", input.name));
        let ffi = delta_to_ffi(&input.delta);
        let json = serde_json::to_vec(&input.delta).unwrap();

        g.bench_function("a_fuellen", |b| {
            b.iter(|| delta_to_ffi(black_box(&input.delta)))
        });
        g.bench_function("a_uebernehmen", |b| {
            b.iter(|| delta_from_ffi(black_box(&ffi)).unwrap())
        });
        g.bench_function("b_json_serialisieren", |b| {
            b.iter(|| serde_json::to_vec(black_box(&input.delta)).unwrap())
        });
        g.bench_function("b_json_deserialisieren", |b| {
            b.iter(|| serde_json::from_slice::<TreeDelta>(black_box(&json)).unwrap())
        });
        g.bench_function("kopie", |b| b.iter(|| black_box(&input.delta).clone()));
        g.bench_function("anwenden", |b| {
            b.iter_batched(
                || {
                    let mut runtime = Runtime::new();
                    if !input.base.is_empty() {
                        runtime
                            .apply(&TreeDelta::between(&SemanticGraph::default(), &input.base))
                            .unwrap();
                    }
                    let mut delta = input.delta.clone();
                    delta.base = runtime.graph().version;
                    (runtime, delta)
                },
                |(mut runtime, delta)| {
                    runtime.apply(&delta).unwrap();
                    runtime
                },
                BatchSize::LargeInput,
            )
        });
        g.finish();
    }

    let mut g = c.benchmark_group("interaction_neuaufbau");
    for (name, pair) in &pairs {
        g.bench_function(*name, |b| {
            b.iter(|| relief_interaction::Graph::build(black_box(&pair.after)))
        });
    }
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    targets = bench
}
criterion_main!(benches);
