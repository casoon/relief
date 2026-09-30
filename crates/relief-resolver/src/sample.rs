//! Von Hand beschriftete Stichprobe unbenannter Controls aus
//! `spike/recordings` (Datei `spike/kalibrierung/fehlende-namen.json`).

use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use relief_ai_contract::{filter, FilteredInput, PrivacyContext};
use relief_model::{perception, TreeId};
use serde::Deserialize;

/// Die Stichprobe, wie sie im Repo liegt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    /// Wozu und nach welcher Regel beschriftet.
    #[serde(rename = "beschreibung")]
    pub description: Vec<String>,
    /// Verzeichnis der Aufnahmen, relativ zur Stichprobendatei.
    #[serde(rename = "aufnahmen")]
    pub recordings: PathBuf,
    #[serde(rename = "stichprobe")]
    pub items: Vec<Item>,
    /// Gesichtete, aber nicht beschriftbare Controls, mit Grund.
    #[serde(rename = "ausgeschlossen")]
    pub excluded: Vec<Excluded>,
}

/// Ein unbenanntes Control mit Soll-Namen.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Seitenverzeichnis unter `aufnahmen`, z. B. `02-shop-broken/01-shop-broken`.
    #[serde(rename = "aufnahme")]
    pub recording: String,
    /// AX-Knoten-ID im Hauptdokument der Anfangsaufnahme.
    #[serde(rename = "knoten")]
    pub node: i32,
    #[serde(rename = "rolle")]
    pub role: String,
    /// Name, den eine sehende Person vergäbe.
    #[serde(rename = "soll")]
    pub expected: String,
    /// Wortteile, von denen einer im vorgeschlagenen Namen stehen muss
    /// (klein geschrieben); siehe [`Item::matches`].
    #[serde(rename = "akzeptiert")]
    pub accepted: Vec<String>,
    #[serde(rename = "begruendung")]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Excluded {
    #[serde(rename = "aufnahme")]
    pub recording: String,
    #[serde(rename = "knoten")]
    pub node: i32,
    #[serde(rename = "grund")]
    pub reason: String,
}

impl Item {
    /// Trefferregel: Der Vorschlag enthält, ohne Groß-/Kleinschreibung und
    /// mit zusammengezogenen Leerzeichen, einen der akzeptierten Wortteile.
    /// Grob, aber nachvollziehbar; Grenzfälle liest man in der Aufzeichnung
    /// nach.
    pub fn matches(&self, proposed: &str) -> bool {
        let proposed = normalize(proposed);
        self.accepted
            .iter()
            .any(|a| proposed.contains(&normalize(a)))
    }
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Sample {
    /// Stichprobe laden; Pfade werden relativ zur Datei aufgelöst.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut sample: Sample =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        sample.recordings = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&sample.recordings);
        Ok(sample)
    }

    /// Gefilterte Anfangsaufnahme einer Seite, wie der Resolver sie sieht.
    pub fn page(&self, recording: &str) -> Result<FilteredInput, String> {
        let dir = self.recordings.join(recording);
        let index: serde_json::Value = read_json(&dir.join("index.json"))?;
        let first = index["anfang"]
            .as_str()
            .ok_or_else(|| format!("{}: kein „anfang“", dir.display()))?;
        let snapshot: AXSnapshot = read_json(&dir.join(first))?;
        let graph = perception::from_snapshot(&snapshot, &TreeId(snapshot.url.clone()));
        Ok(filter(&graph, &PrivacyContext::default()))
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Lokale ID eines Knotens im Hauptdokument, z. B. `t0:15`. Der Filter
/// nummeriert Bäume in Dokumentreihenfolge; das Hauptdokument ist `t0`.
pub fn local_id(input: &FilteredInput, node: i32) -> Option<String> {
    let id = format!("t0:{node}");
    input.node_ref(&id).map(|_| id)
}

/// Rollen, die ein Name bedienbar macht: Kandidaten für den Resolver.
/// Formularfelder fehlen: Ihren Namen rät kein Modell, und der Filter
/// schwärzt sie ohnehin teilweise.
pub const CONTROL_ROLES: &[&str] = &["button", "link", "menuitem", "tab"];

/// Controls ohne Namen: was ein Resolver auf der Seite benennen müsste.
pub fn unnamed_controls(input: &FilteredInput) -> Vec<&str> {
    input
        .nodes()
        .iter()
        .filter(|n| {
            n.name.is_none() && n.redacted.is_none() && CONTROL_ROLES.contains(&n.role.as_str())
        })
        .map(|n| n.id.as_str())
        .collect()
}
