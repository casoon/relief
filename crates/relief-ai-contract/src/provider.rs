//! Modell-Anbieter in Stufen (→ `plan/spezifikation/06`, Modellstrategie).
//!
//! Kein Anbieter ist hier eingebaut; Adapter implementieren
//! [`ModelProvider`]. Die Runtime ruft sie nur über [`resolve_missing`] und
//! [`propose_intent`] auf, die jede Ausgabe gegen den Vertrag prüfen.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    validate_hypotheses, validate_intent, FilteredInput, Hypothesis, IntentProposal, LimitExceeded,
    UserUtterance, ValidationError, HYPOTHESES_SCHEMA, INTENT_SCHEMA,
};

/// Wie weit Relief für Modelle geht, von der Nutzerin gewählt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Nur deterministisch. Standard; jede Funktion muss damit gehen.
    #[default]
    None,
    /// Modelle, die das Betriebssystem mitbringt.
    Os,
    /// Selbst installierte lokale Runtime.
    Local,
    /// Cloud-Modell mit eigenem API-Key, nur mit Zustimmung (→ `spezifikation/07`).
    Api,
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Tier::None => "none",
            Tier::Os => "os",
            Tier::Local => "local",
            Tier::Api => "api",
        })
    }
}

/// Welches Modell eine Ausgabe erzeugt hat; geht als
/// [`relief_model::Source::Model`] in jede Aussage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ModelId {
    pub tier: Tier,
    pub name: String,
    pub version: String,
}

impl fmt::Display for ModelId {
    /// `local:<name>@<version>`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}@{}", self.tier, self.name, self.version)
    }
}

/// Was das Modell tun soll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Task {
    /// Nur fehlende Semantik rekonstruieren; Ausgabe nach
    /// [`HYPOTHESES_SCHEMA`].
    ResolveMissing,
    /// Die Äußerung in einen Intent übersetzen; Ausgabe nach
    /// [`INTENT_SCHEMA`].
    ParseIntent { utterance: UserUtterance },
}

/// Anfrage an einen Anbieter. Entsteht nur aus einer [`FilteredInput`].
///
/// Serialisiert trennt sie Auftrag (`task`, mit der Äußerung der Nutzerin)
/// und Seiteninhalt (`input`, nur Daten).
#[derive(Debug, Clone, Serialize)]
pub struct ModelRequest {
    task: Task,
    input: FilteredInput,
}

impl ModelRequest {
    pub fn resolve_missing(input: FilteredInput) -> Self {
        Self {
            task: Task::ResolveMissing,
            input,
        }
    }

    pub fn parse_intent(utterance: UserUtterance, input: FilteredInput) -> Self {
        Self {
            task: Task::ParseIntent { utterance },
            input,
        }
    }

    pub fn task(&self) -> &Task {
        &self.task
    }

    pub fn input(&self) -> &FilteredInput {
        &self.input
    }

    /// JSON-Schema, dem die Antwort folgen muss.
    pub fn schema(&self) -> &'static str {
        match self.task {
            Task::ResolveMissing => HYPOTHESES_SCHEMA,
            Task::ParseIntent { .. } => INTENT_SCHEMA,
        }
    }
}

/// Rohe Antwort eines Anbieters. Wird erst durch die Prüfung zu etwas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReply {
    pub model: ModelId,
    /// JSON nach [`ModelRequest::schema`].
    pub text: String,
    /// Verbrauch, soweit der Anbieter ihn meldet (Kosten je Anfrage,
    /// → `spezifikation/06`).
    pub usage: Option<Usage>,
}

/// Tokens einer Anfrage, wie der Anbieter sie abrechnet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Anbieter nicht erreichbar oder Aufruf gescheitert (offline, Key
/// ungültig, Kontingent). Die Runtime fällt dann auf `none` zurück.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError(pub String);

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Modell nicht verfügbar: {}", self.0)
    }
}

impl std::error::Error for ProviderError {}

/// Ein Modell-Anbieter.
///
/// Nimmt nur eine [`ModelRequest`] an, also nur gefilterte Eingaben, und
/// liefert nur Text. Führt nichts aus. Synchron: Die Runtime ruft Anbieter
/// außerhalb des Update-Pfads auf (→ `spezifikation/06`, Performance).
pub trait ModelProvider {
    fn tier(&self) -> Tier;

    /// `Ok(None)`: Diese Stufe hat kein Modell.
    fn complete(&self, request: &ModelRequest) -> Result<Option<ModelReply>, ProviderError>;
}

/// Stufe `none`: kein Modell, liefert nie etwas. Standard.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoModel;

impl ModelProvider for NoModel {
    fn tier(&self) -> Tier {
        Tier::None
    }

    fn complete(&self, _: &ModelRequest) -> Result<Option<ModelReply>, ProviderError> {
        Ok(None)
    }
}

/// Anbieter gescheitert, Ausgabe verletzt den Vertrag oder die Aufgabe ist
/// an einer Grenze beendet ([`crate::Budget`]).
#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    Provider(ProviderError),
    Invalid(ValidationError),
    Limit(LimitExceeded),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Provider(e) => e.fmt(f),
            ModelError::Invalid(e) => e.fmt(f),
            ModelError::Limit(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ModelError {}

/// Fehlende Semantik erfragen. Mit `none` immer leer.
pub fn resolve_missing(
    provider: &dyn ModelProvider,
    input: FilteredInput,
) -> Result<Vec<Hypothesis>, ModelError> {
    let request = ModelRequest::resolve_missing(input);
    let Some(reply) = provider.complete(&request).map_err(ModelError::Provider)? else {
        return Ok(Vec::new());
    };
    validate_hypotheses(&reply.text, request.input(), &reply.model).map_err(ModelError::Invalid)
}

/// Eine Äußerung der Nutzerin in einen Intent-Vorschlag übersetzen lassen.
/// Mit `none` immer `None`; dann bleibt es beim deterministischen Parser.
pub fn propose_intent(
    provider: &dyn ModelProvider,
    utterance: &UserUtterance,
    input: FilteredInput,
) -> Result<Option<IntentProposal>, ModelError> {
    let request = ModelRequest::parse_intent(utterance.clone(), input);
    let Some(reply) = provider.complete(&request).map_err(ModelError::Provider)? else {
        return Ok(None);
    };
    validate_intent(&reply.text, request.input(), utterance, &reply.model)
        .map(Some)
        .map_err(ModelError::Invalid)
}
