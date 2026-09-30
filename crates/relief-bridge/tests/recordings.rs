//! Die `cxx`-Grenzstrukturen auf allen Aufnahmepaaren: Hin- und Rückweg
//! verliert nichts außer den nur von CDP gemeldeten `ignored_reasons`, und
//! die Runtime kommt über die Grenze zum selben Graphen wie das Modell.

mod common;

use relief_bridge::{delta_from_ffi, delta_to_ffi, Runtime};
use relief_model::{SemanticGraph, TreeDelta};

/// Was die Grenze absichtlich nicht trägt (→ `src/ffi.rs`).
fn ohne_cdp_gruende(mut graph: SemanticGraph) -> SemanticGraph {
    for tree in graph.trees.values_mut() {
        for node in tree.nodes.values_mut() {
            node.ignored_reasons.clear();
        }
    }
    graph
}

#[test]
fn grenze_verliert_nichts_und_runtime_folgt_dem_modell() {
    let pairs = common::pairs();
    assert!(!pairs.is_empty(), "keine Aufnahmen gefunden");
    for pair in &pairs {
        let before = ohne_cdp_gruende(pair.before.clone());
        let after = ohne_cdp_gruende(pair.after.clone());

        let full = TreeDelta::between(&SemanticGraph::default(), &before);
        let step = TreeDelta::between(&before, &after);
        for delta in [&full, &step] {
            let back = delta_from_ffi(&delta_to_ffi(delta)).unwrap();
            assert!(back == *delta, "{}: Grenze verliert etwas", pair.name);
        }

        let mut step_from_1 = step.clone();
        step_from_1.base = before.version.next();
        let mut runtime = Runtime::new();
        runtime
            .apply(&delta_from_ffi(&delta_to_ffi(&full)).unwrap())
            .unwrap();
        runtime
            .apply(&delta_from_ffi(&delta_to_ffi(&step_from_1)).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", pair.name));
        let mut expected = after.clone();
        expected.version = before.version.next().next();
        assert!(
            *runtime.graph() == expected,
            "{}: Runtime weicht ab",
            pair.name
        );
    }
    println!("{} Aufnahmepaare", pairs.len());
}
