//! Gemeinsame Helfer der Aufnahme-Tests: Aufnahmen aus `spike/recordings`
//! laden und Befehle so auflösen, wie der CDP-Host es tut — ohne Browser.

#![allow(dead_code)] // Nicht jede Testdatei nutzt jeden Helfer.

use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use relief_interaction::{resolve, Command, Control, Graph, Resolution};
use relief_model::{perception, SemanticGraph, TreeId};
use serde::Deserialize;

pub fn recordings() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spike/recordings")
}

/// Eine aufgenommene Seite: `<aufgabendatei>/<NN>-<seite>/index.json`.
#[derive(Debug, Deserialize)]
pub struct Seite {
    #[serde(skip)]
    pub dir: PathBuf,
    /// `<aufgabendatei>/<NN>-<seite>`, z. B. `01-shop-clean/01-shop-clean`.
    #[serde(skip)]
    pub name: String,
    pub url: String,
    pub anfang: String,
    pub schritte: Vec<Schritt>,
}

/// Ein `do:`-Schritt. Die aufgezeichnete Antwort stammt aus altem Code und
/// ist bewusst nicht Teil der Struktur: Tests legen ihre Erwartungen selbst
/// fest.
#[derive(Debug, Deserialize)]
pub struct Schritt {
    pub eingabe: String,
    pub vorher: String,
    pub nachher: String,
}

impl Seite {
    pub fn laden(name: &str) -> Seite {
        let dir = recordings().join(name);
        let text = std::fs::read_to_string(dir.join("index.json"))
            .unwrap_or_else(|e| panic!("{name}/index.json: {e}"));
        let mut seite: Seite =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}/index.json: {e}"));
        seite.dir = dir;
        seite.name = name.to_string();
        seite
    }

    pub fn aufnahme(&self, datei: &str) -> AXSnapshot {
        let text = std::fs::read_to_string(self.dir.join(datei))
            .unwrap_or_else(|e| panic!("{}/{datei}: {e}", self.name));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}/{datei}: {e}", self.name))
    }

    pub fn anfang(&self) -> AXSnapshot {
        self.aufnahme(&self.anfang)
    }

    /// Der `n`-te Schritt mit dieser Eingabe (ab 0). Prüft die Eingabe, damit
    /// ein neu aufgenommenes `index.json` nicht still andere Schritte liefert.
    pub fn schritt(&self, eingabe: &str, n: usize) -> &Schritt {
        self.schritte
            .iter()
            .filter(|s| s.eingabe == eingabe)
            .nth(n)
            .unwrap_or_else(|| panic!("{}: kein Schritt „{eingabe}“ #{n}", self.name))
    }

    /// Aufnahmen vorher und nachher des Schritts.
    pub fn paar(&self, eingabe: &str, n: usize) -> (AXSnapshot, AXSnapshot) {
        let s = self.schritt(eingabe, n);
        (self.aufnahme(&s.vorher), self.aufnahme(&s.nachher))
    }

    /// Alle unterschiedlichen Aufnahmedateien der Seite, sortiert.
    pub fn dateien(&self) -> Vec<String> {
        let mut files: Vec<String> = std::iter::once(self.anfang.clone())
            .chain(
                self.schritte
                    .iter()
                    .flat_map(|s| [s.vorher.clone(), s.nachher.clone()]),
            )
            .collect();
        files.sort();
        files.dedup();
        files
    }
}

/// Alle aufgenommenen Seiten, sortiert nach Namen.
pub fn seiten() -> Vec<Seite> {
    let mut names = Vec::new();
    for task in std::fs::read_dir(recordings()).expect("spike/recordings fehlt") {
        let task = task.unwrap().path();
        if !task.is_dir() {
            continue;
        }
        for page in std::fs::read_dir(&task).unwrap() {
            let page = page.unwrap().path();
            if page.join("index.json").exists() {
                let rel = page.strip_prefix(recordings()).unwrap();
                names.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    names.sort();
    names.iter().map(|n| Seite::laden(n)).collect()
}

/// Aufnahme als Modell. Tree-ID des Hauptdokuments ist die URL, wie im
/// Rundtest von `relief-model` (`crates/relief-model/tests/recordings.rs`):
/// Eine andere URL heißt in den Aufnahmen ein anderes Dokument.
pub fn modell(snap: &AXSnapshot) -> SemanticGraph {
    perception::from_snapshot(snap, &TreeId(snap.url.clone()))
}

/// Graph der Aufnahme.
pub fn graph(snap: &AXSnapshot) -> Graph {
    Graph::build(&modell(snap))
}

/// Zielauflösung eines Befehls mit denselben Filtern wie der CDP-Host
/// (`crates/relief-cdp/src/main.rs`, `Host::handle`). `None` für Befehle
/// ohne Bedienelement als Ziel.
pub fn ziel<'g>(graph: &'g Graph, cmd: &Command) -> Option<Resolution<'g>> {
    Some(match cmd {
        Command::Focus(q) | Command::Activate(q) => resolve(graph, q, |_| true),
        Command::SetValue(q, _) => resolve(graph, q, |c| {
            matches!(
                c.role.as_str(),
                "textbox" | "searchbox" | "combobox" | "spinbutton"
            )
        }),
        Command::Select(Some(q), _) => resolve(graph, q, |c| !c.options.is_empty()),
        Command::Increment(q) | Command::Decrement(q) => resolve(graph, q, |c| {
            matches!(c.role.as_str(), "slider" | "spinbutton")
        }),
        _ => return None,
    })
}

/// Erreichbare Auswahlfelder mit dieser Option (Host: `pick_by_option`).
pub fn felder_mit_option<'g>(graph: &'g Graph, option: &str) -> Vec<&'g Control> {
    graph
        .reachable_controls()
        .filter(|c| c.options.iter().any(|o| o.eq_ignore_ascii_case(option)))
        .collect()
}

/// Kurzform eines Bedienelements für Vergleiche: `[rolle] Name`.
pub fn kurz(c: &Control) -> String {
    format!("[{}] {}", c.role, c.display_name())
}

/// Eindeutiges Ziel oder Testfehler mit dem, was stattdessen gefunden wurde.
pub fn eins<'g>(r: Option<Resolution<'g>>) -> &'g Control {
    match r {
        Some(Resolution::One(c)) => c,
        Some(Resolution::Many(cs)) => panic!(
            "mehrdeutig: {:?}",
            cs.iter().map(|c| kurz(c)).collect::<Vec<_>>()
        ),
        Some(Resolution::None) => panic!("nichts gefunden"),
        None => panic!("Befehl hat kein Bedienelement als Ziel"),
    }
}
