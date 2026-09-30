//! Rundtest des Delta-Formats auf allen Aufnahmepaaren in `spike/recordings`:
//! `apply(vorher, between(vorher, nachher)) == nachher`, auf Modellebene.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use relief_model::{perception, SemanticGraph, TreeDelta, TreeId};
use serde_json::Value;

fn recordings() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spike/recordings")
}

fn page_dirs() -> Vec<PathBuf> {
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
    dirs
}

/// Aufnahme als Modell. Tree-ID des Hauptdokuments ist die URL: CDP kennt
/// keine Tree-ID, und eine andere URL heißt in den Aufnahmen ein anderes
/// Dokument [Annahme; ein Neuladen derselben URL bliebe unerkannt].
fn model(dir: &Path, file: &str, cache: &mut BTreeMap<String, SemanticGraph>) -> SemanticGraph {
    cache
        .entry(file.to_string())
        .or_insert_with(|| {
            let text = std::fs::read_to_string(dir.join(file)).unwrap();
            let snap: AXSnapshot = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{}/{file}: {e}", dir.display()));
            perception::from_snapshot(&snap, &TreeId(snap.url.clone()))
        })
        .clone()
}

#[test]
fn delta_rundtest_auf_allen_aufnahmepaaren() {
    let dirs = page_dirs();
    assert!(!dirs.is_empty(), "keine Aufnahmen gefunden");
    let mut pairs = 0;
    for dir in &dirs {
        let index: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap())
                .unwrap();
        let mut cache = BTreeMap::new();
        let name = dir.file_name().unwrap().to_string_lossy();
        for step in index["schritte"].as_array().unwrap() {
            let (v, n) = (
                step["vorher"].as_str().unwrap(),
                step["nachher"].as_str().unwrap(),
            );
            let before = model(dir, v, &mut cache);
            let after = model(dir, n, &mut cache);

            let delta = TreeDelta::between(&before, &after);
            let mut applied = before.clone();
            applied
                .apply(&delta)
                .unwrap_or_else(|e| panic!("{name} {v}→{n}: {e}"));
            let mut expected = after.clone();
            expected.version = before.version.next();
            assert!(applied == expected, "{name} {v}→{n}: Rundtest verfehlt");
            assert_eq!(delta.is_empty(), v == n, "{name} {v}→{n}");

            // Die Delta übersteht die Serialisierung.
            let json = serde_json::to_string(&delta).unwrap();
            assert_eq!(serde_json::from_str::<TreeDelta>(&json).unwrap(), delta);

            let (created, changed, removed) = delta.trees.iter().fold((0, 0, 0), |a, u| {
                (
                    a.0 + u.created.len(),
                    a.1 + u.changed.len(),
                    a.2 + u.removed.len(),
                )
            });
            println!(
                "{name} {v}→{n}: {} → {} Knoten, Delta +{created} ~{changed} -{removed}, \
                 {} Bäume weg, {} Byte JSON",
                before.len(),
                after.len(),
                delta.removed_trees.len(),
                json.len()
            );
            pairs += 1;
        }
    }
    println!("{} Seiten, {pairs} Aufnahmepaare", dirs.len());
}

#[test]
fn jede_aufnahme_ergibt_ein_zusammenhaengendes_serialisierbares_modell() {
    for dir in page_dirs() {
        let index: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap())
                .unwrap();
        let mut cache = BTreeMap::new();
        let mut files = vec![index["anfang"].as_str().unwrap().to_string()];
        for step in index["schritte"].as_array().unwrap() {
            for key in ["vorher", "nachher"] {
                files.push(step[key].as_str().unwrap().to_string());
            }
        }
        files.sort();
        files.dedup();
        for file in files {
            let graph = model(&dir, &file, &mut cache);
            let at = format!("{}/{file}", dir.display());
            assert!(!graph.is_empty(), "{at}: leer");
            assert_eq!(
                graph.document_order().len(),
                graph.len(),
                "{at}: nicht jeder Knoten ist vom Hauptbaum aus erreichbar"
            );
            let json = serde_json::to_string(&graph).unwrap();
            let back: SemanticGraph = serde_json::from_str(&json).unwrap();
            assert!(back == graph, "{at}: Serialisierung verliert etwas");
        }
    }
}
