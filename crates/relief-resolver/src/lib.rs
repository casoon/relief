//! Relief: Resolver für fehlende Namen (→ `plan/spezifikation/06`).
//!
//! Browserfrei. Baut auf dem Vertrag in `relief-ai-contract` auf:
//!
//! | Baustein | Inhalt |
//! |---|---|
//! | [`resolve_node`], [`NEIGHBOURHOOD_NODES`] | Einen Knoten benennen lassen: Ausschnitt, Anbieter, strenge Prüfung |
//! | [`anthropic`] | Adapter für die Anthropic Messages API (Stufe `api`); Netzcode nur mit Feature `anthropic` |
//! | [`replay`] | Antworten aufzeichnen und wiedergeben (Tests, Auswertung ohne erneute Kosten) |
//! | [`sample`], [`calibrate`] | Kalibrierung an einer von Hand beschrifteten Stichprobe aus `spike/recordings` |
//!
//! Standard bleibt Stufe `none`: Ohne gewählten Anbieter fragt niemand ein
//! Modell.

#![forbid(unsafe_code)]

pub mod anthropic;
pub mod calibrate;
pub mod replay;
pub mod sample;

use relief_ai_contract::{Budget, FilteredInput, Hypothesis, ModelError, ModelProvider, Property};

/// Größe des Fensters um den Knoten in einer Resolver-Anfrage, in Knoten
/// (ohne die Vorfahren). Gemessen: Anfragegröße in `spezifikation/06`.
pub const NEIGHBOURHOOD_NODES: usize = 40;

/// Namen für einen Knoten erfragen, dessen Name fehlt.
///
/// Das Modell sieht nur den Ausschnitt um `id` ([`FilteredInput::excerpt`]).
/// `Ok(None)`: Stufe ohne Modell, `id` gibt es in der Eingabe nicht, oder das
/// Modell schlägt für den Knoten keinen Namen vor. Der Aufruf zählt gegen
/// `budget` (eines je Aufgabe oder Seite).
pub fn resolve_node(
    budget: &mut Budget,
    provider: &dyn ModelProvider,
    input: &FilteredInput,
    id: &str,
) -> Result<Option<Hypothesis>, ModelError> {
    let Some(excerpt) = input.excerpt(id, NEIGHBOURHOOD_NODES) else {
        return Ok(None);
    };
    let at = excerpt.node_ref(id).cloned();
    Ok(budget
        .resolve_missing(provider, excerpt)?
        .into_iter()
        .find(|h| Some(&h.node) == at.as_ref() && h.property == Property::Name))
}
