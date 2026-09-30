//! Aufnahmepaare aus `spike/recordings` als Modell, für Tests und Benchmarks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use relief_model::{perception, SemanticGraph, TreeId};
use serde_json::Value;

/// Ein Schritt einer Aufnahme: Stand vorher und nachher.
pub struct Pair {
    pub name: String,
    pub before: SemanticGraph,
    pub after: SemanticGraph,
}

fn recordings() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spike/recordings")
}

/// Alle Aufnahmepaare mit verschiedenen Ständen, sortiert nach Seite.
///
/// Tree-ID des Hauptdokuments ist die URL, wie im Rundtest von
/// `relief-model` (`crates/relief-model/tests/recordings.rs`).
pub fn pairs() -> Vec<Pair> {
    let mut dirs = Vec::new();
    for task in std::fs::read_dir(recordings()).expect("spike/recordings fehlt") {
        let task = task.unwrap().path();
        if !task.is_dir() {
            continue;
        }
        for page in std::fs::read_dir(&task).unwrap() {
            let page = page.unwrap().path();
            if page.join("index.json").exists() {
                dirs.push(page);
            }
        }
    }
    dirs.sort();

    let mut out = Vec::new();
    for dir in dirs {
        let index: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap())
                .unwrap();
        let mut cache: BTreeMap<String, AXSnapshot> = BTreeMap::new();
        let mut load = |file: &str| {
            cache
                .entry(file.to_string())
                .or_insert_with(|| {
                    let text = std::fs::read_to_string(dir.join(file)).unwrap();
                    serde_json::from_str(&text).unwrap()
                })
                .clone()
        };
        let page = dir.file_name().unwrap().to_string_lossy().to_string();
        for step in index["schritte"].as_array().unwrap() {
            let (v, n) = (
                step["vorher"].as_str().unwrap(),
                step["nachher"].as_str().unwrap(),
            );
            if v == n {
                continue;
            }
            let before = load(v);
            let after = load(n);
            out.push(Pair {
                name: format!("{page} {v}→{n}"),
                before: perception::from_snapshot(&before, &TreeId(before.url.clone())),
                after: perception::from_snapshot(&after, &TreeId(after.url.clone())),
            });
        }
    }
    out
}
