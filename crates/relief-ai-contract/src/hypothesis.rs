//! Hypothesen zu fehlender Semantik (→ `plan/spezifikation/06`).
//!
//! Ein Modell liefert [`HypothesesOutput`] als JSON nach
//! [`HYPOTHESES_SCHEMA`]; [`validate_hypotheses`] prüft sie gegen die Eingabe
//! und ergänzt, was das Modell nicht selbst angeben kann: Knoten, Modell und
//! Stand. Pflichtmetadaten jeder Inferenz: Wert, Confidence, Evidence,
//! Modell, Stand.

use std::collections::BTreeSet;
use std::fmt;

use relief_model::{Certainty, Fact, GraphVersion, NodeRef, Source};
use serde::{Deserialize, Serialize};

use crate::{FilteredInput, ModelId, Redaction, ValidationError};

/// JSON-Schema der Modellausgabe, handgeschrieben; dass es zu den Typen
/// passt, prüft ein Test.
pub const HYPOTHESES_SCHEMA: &str = include_str!("../schema/hypotheses.schema.json");

/// Höchstzahl Hypothesen je Ausgabe.
pub const MAX_HYPOTHESES: usize = 50;
/// Höchstlänge eines Werts in Zeichen (auch für Intent-Werte).
pub const MAX_VALUE_CHARS: usize = 200;
/// Höchstzahl Evidence-Einträge je Hypothese.
pub const MAX_EVIDENCE: usize = 8;
pub const MAX_EVIDENCE_CHARS: usize = 200;

/// Welche fehlende Eigenschaft rekonstruiert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Property {
    Name,
    Description,
}

impl Property {
    pub const ALL: [Property; 2] = [Property::Name, Property::Description];
}

/// Ausgabe eines Modells, wie das Schema sie beschreibt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesesOutput {
    pub hypotheses: Vec<HypothesisOutput>,
}

/// Eine Hypothese, wie das Modell sie liefert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesisOutput {
    /// Lokale ID aus der Eingabe, z. B. `t0:18`.
    pub node: String,
    pub property: Property,
    pub value: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

/// Eine geprüfte Hypothese mit Pflichtmetadaten.
///
/// Eine Aussage, keine Aktion: Es gibt keinen Weg von hier zu einem Intent
/// oder `ActionPlan`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hypothesis {
    pub node: NodeRef,
    pub property: Property,
    pub value: String,
    /// Vom Modell genannt, **nicht kalibriert** (→ `spezifikation/06`).
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub model: ModelId,
    /// Stand des Graphen, aus dem die Eingabe entstand.
    pub at: GraphVersion,
}

/// Gemessene Schwellen je Modell: ab dieser vom Modell genannten Confidence
/// ist eine Hypothese [`Certainty::Inferred`].
///
/// Schlüssel ist [`ModelId`] als Text (`api:anthropic@claude-haiku-4-5-20251001`).
/// Eine Schwelle gilt nur für das Modell, an dem sie gemessen wurde; die
/// Zahlen verschiedener Modelle sind nicht vergleichbar. Eingetragen wird
/// nur mit Messung in `plan/spezifikation/06` (Werkzeug: `relief-resolver
/// kalibrieren`). Leer: noch keine Messung, alle Hypothesen bleiben
/// [`Certainty::Uncertain`].
pub const CALIBRATED_THRESHOLDS: &[(&str, f32)] = &[];

/// Gemessene Schwelle für ein Modell, falls es eine gibt.
pub fn calibrated_threshold(model: &ModelId) -> Option<f32> {
    let model = model.to_string();
    CALIBRATED_THRESHOLDS
        .iter()
        .find(|(m, _)| *m == model)
        .map(|&(_, t)| t)
}

impl Hypothesis {
    /// Als Aussage fürs Modell.
    ///
    /// [`Certainty::Inferred`] nur, wenn für das Modell eine gemessene
    /// Schwelle vorliegt ([`CALIBRATED_THRESHOLDS`]) und die Confidence sie
    /// erreicht; sonst [`Certainty::Uncertain`]. Eine vom Modell genannte
    /// Zahl allein ist keine Wahrscheinlichkeit.
    pub fn fact(&self) -> Fact<String> {
        let inferred = calibrated_threshold(&self.model).is_some_and(|t| self.confidence >= t);
        Fact {
            value: Some(self.value.clone()),
            certainty: if inferred {
                Certainty::Inferred
            } else {
                Certainty::Uncertain
            },
            source: Source::Model(self.model.to_string()),
            confidence: Some(self.confidence),
            evidence: self.evidence.clone(),
        }
    }
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Property::Name => "name",
            Property::Description => "description",
        })
    }
}

/// Modellausgabe streng prüfen. Ein Verstoß verwirft die ganze Ausgabe.
///
/// Geprüft: Schema (keine unbekannten oder fehlenden Felder), Anzahl,
/// Wertebereiche, Knoten existiert in der Eingabe, ist nicht sensibel
/// geschwärzt, und die Eigenschaft fehlt dort tatsächlich.
pub fn validate_hypotheses(
    raw: &str,
    input: &FilteredInput,
    model: &ModelId,
) -> Result<Vec<Hypothesis>, ValidationError> {
    let output: HypothesesOutput =
        serde_json::from_str(raw).map_err(|e| ValidationError::Format(e.to_string()))?;
    if output.hypotheses.len() > MAX_HYPOTHESES {
        return Err(ValidationError::TooMany(output.hypotheses.len()));
    }
    let mut seen = BTreeSet::new();
    output
        .hypotheses
        .into_iter()
        .map(|h| {
            let target = input
                .target(&h.node)
                .ok_or_else(|| ValidationError::UnknownNode(h.node.clone()))?;
            if target.redaction == Some(Redaction::Sensitive) {
                return Err(ValidationError::Redacted(h.node));
            }
            let missing = match h.property {
                Property::Name => target.missing_name,
                Property::Description => target.missing_description,
            };
            if !missing {
                return Err(ValidationError::NotMissing {
                    node: h.node,
                    property: h.property,
                });
            }
            if !seen.insert((h.node.clone(), h.property)) {
                return Err(ValidationError::Duplicate {
                    node: h.node,
                    property: h.property,
                });
            }
            check_confidence(h.confidence)?;
            check_value(&h.value)?;
            if h.evidence.is_empty() {
                return Err(ValidationError::Evidence("fehlt"));
            }
            if h.evidence.len() > MAX_EVIDENCE {
                return Err(ValidationError::Evidence("zu viele Einträge"));
            }
            if h.evidence
                .iter()
                .any(|e| e.trim().is_empty() || e.chars().count() > MAX_EVIDENCE_CHARS)
            {
                return Err(ValidationError::Evidence("Eintrag leer oder zu lang"));
            }
            Ok(Hypothesis {
                node: target.at.clone(),
                property: h.property,
                value: h.value,
                confidence: h.confidence,
                evidence: h.evidence,
                model: model.clone(),
                at: input.graph_version(),
            })
        })
        .collect()
}

pub(crate) fn check_confidence(c: f32) -> Result<(), ValidationError> {
    if (0.0..=1.0).contains(&c) {
        Ok(())
    } else {
        Err(ValidationError::Confidence(c))
    }
}

pub(crate) fn check_value(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::Value("leer"))
    } else if value.chars().count() > MAX_VALUE_CHARS {
        Err(ValidationError::Value("zu lang"))
    } else {
        Ok(())
    }
}
