//! Herkunft einer Aussage (→ `plan/spezifikation/03`, „Fact“).

use serde::{Deserialize, Serialize};

/// Wie sicher eine Aussage ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Certainty {
    /// Direkt aus Autorensemantik (HTML, ARIA), von Chromium berechnet.
    Known,
    /// Rekonstruiert (Regel, DOM-Kontext, Modell), Confidence über Schwelle,
    /// mit Evidence.
    Inferred,
    /// Hypothese unter Schwelle. Nie als Tatsache ausgeben, nie Grundlage
    /// einer Aktion ohne Rückfrage.
    Uncertain,
}

/// Wer eine Aussage erzeugt hat.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "id")]
pub enum Source {
    /// Chromium-Accessibility-Tree (vereint HTML- und ARIA-Semantik).
    Chromium,
    /// Deterministische Regel des Rust-Cores, mit Regel-ID.
    Rule(String),
    /// Modell, mit Name und Version.
    Model(String),
}

/// Eine Aussage mit Herkunft.
///
/// `value: None` mit [`Certainty::Known`] heißt: Chromium meldet nachweislich
/// keinen Wert (z. B. ein Button ohne Namen). Das ist eine andere Aussage als
/// ein unsicherer Wert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact<T> {
    pub value: Option<T>,
    pub certainty: Certainty,
    pub source: Source,
    /// Nur bei [`Certainty::Inferred`] und [`Certainty::Uncertain`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

impl<T> Fact<T> {
    /// Was Chromium meldet, ohne Deutung.
    pub fn known(value: Option<T>) -> Self {
        Self {
            value,
            certainty: Certainty::Known,
            source: Source::Chromium,
            confidence: None,
            evidence: Vec::new(),
        }
    }
}

impl<T> Default for Fact<T> {
    fn default() -> Self {
        Self::known(None)
    }
}
