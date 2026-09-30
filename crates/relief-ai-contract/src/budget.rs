//! Feste Grenzen je Seite oder Aufgabe für Modellaufrufe
//! (→ `plan/spezifikation/07`, Ressourcenmissbrauch).
//!
//! Ein [`Budget`] gilt für eine Aufgabe auf einer Seite; der Host legt für
//! jede neue Aufgabe oder Seite ein neues an. Es zählt Aufrufe, gleiche
//! Anfragen, Zeit und Tokens und prüft die Größe der Eingabe. Die erste
//! Überschreitung beendet die Aufgabe: Danach ruft das Budget keinen Anbieter
//! mehr auf und liefert nur noch dieselbe [`LimitExceeded`].
//!
//! Das Budget ist der einzige Weg zu einem Anbieter: Nur es stellt die
//! [`Permit`] aus, die [`ModelProvider::complete`] verlangt. Ein
//! Modellaufruf der Runtime kann es also nicht umgehen.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use relief_interaction::{Limit, SecurityEvent};

use crate::{
    validate_hypotheses, validate_intent, FilteredInput, Hypothesis, IntentProposal, ModelError,
    ModelProvider, ModelReply, ModelRequest, Permit, UserUtterance,
};

/// Obergrenzen für eine Aufgabe.
///
/// Die Standardwerte sind eine [Annahme], nicht gemessen: groß genug für
/// die Aufnahmen und einen Resolver-Ausschnitt, klein genug, dass eine
/// Schleife nach Sekunden und wenigen Cent endet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Knoten in einer Modelleingabe.
    pub max_nodes: usize,
    pub max_calls: u32,
    /// Wie oft dieselbe Anfrage gestellt werden darf (1 = keine Wiederholung).
    pub max_repeats: u32,
    pub max_time: Duration,
    /// Tokens (Eingabe plus Ausgabe), soweit der Anbieter sie meldet.
    pub max_tokens: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_nodes: 5_000,
            max_calls: 20,
            max_repeats: 2,
            max_time: Duration::from_secs(120),
            max_tokens: 100_000,
        }
    }
}

/// Eine Grenze ist erreicht; die Aufgabe ist beendet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitExceeded {
    pub limit: Limit,
    /// Verbrauch, der die Grenze überschritten hätte oder hat.
    pub used: u64,
    pub max: u64,
}

impl LimitExceeded {
    /// Eintrag fürs Security-Log.
    pub fn event(&self) -> SecurityEvent {
        SecurityEvent::abort(self.limit)
    }
}

impl fmt::Display for LimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Aufgabe abgebrochen: Grenze für {} erreicht ({} bei höchstens {}). \
             Für diese Aufgabe wird kein Modell mehr gefragt.",
            self.limit.label(),
            self.used,
            self.max
        )
    }
}

/// Verbrauch einer Aufgabe gegen ihre [`Limits`].
#[derive(Debug)]
pub struct Budget {
    limits: Limits,
    started: Instant,
    calls: u32,
    tokens: u64,
    requests: HashMap<u64, u32>,
    stopped: Option<LimitExceeded>,
}

impl Budget {
    pub fn new(limits: Limits) -> Self {
        Budget {
            limits,
            started: Instant::now(),
            calls: 0,
            tokens: 0,
            requests: HashMap::new(),
            stopped: None,
        }
    }

    /// Gestellte Anfragen (Aufrufe des Anbieters).
    pub fn calls(&self) -> u32 {
        self.calls
    }

    /// Gemeldete Tokens.
    pub fn tokens(&self) -> u64 {
        self.tokens
    }

    /// Die Grenze, an der die Aufgabe endete.
    pub fn stopped(&self) -> Option<LimitExceeded> {
        self.stopped
    }

    /// Fehlende Semantik erfragen und die Ausgabe streng prüfen. Mit `none`
    /// immer leer.
    pub fn resolve_missing(
        &mut self,
        provider: &dyn ModelProvider,
        input: FilteredInput,
    ) -> Result<Vec<Hypothesis>, ModelError> {
        let request = ModelRequest::resolve_missing(input);
        let Some(reply) = self.complete(provider, &request)? else {
            return Ok(Vec::new());
        };
        validate_hypotheses(&reply.text, request.input(), &reply.model).map_err(ModelError::Invalid)
    }

    /// Eine Äußerung der Nutzerin in einen Intent-Vorschlag übersetzen lassen.
    /// Mit `none` immer `None`; dann bleibt es beim deterministischen Parser.
    pub fn propose_intent(
        &mut self,
        provider: &dyn ModelProvider,
        utterance: &UserUtterance,
        input: FilteredInput,
    ) -> Result<Option<IntentProposal>, ModelError> {
        let request = ModelRequest::parse_intent(utterance.clone(), input);
        let Some(reply) = self.complete(provider, &request)? else {
            return Ok(None);
        };
        validate_intent(&reply.text, request.input(), utterance, &reply.model)
            .map(Some)
            .map_err(ModelError::Invalid)
    }

    /// Grenzen vor dem Aufruf prüfen, aufrufen, Tokens danach zählen. Die
    /// Antwort ist ungeprüfter Text; zu Hypothese oder Intent wird sie nur
    /// über [`Budget::resolve_missing`] bzw. [`Budget::propose_intent`] (oder
    /// [`crate::validate_hypotheses`]). Für Messläufe, die jede Antwort sehen
    /// müssen (Kalibrierung).
    pub fn complete(
        &mut self,
        provider: &dyn ModelProvider,
        request: &ModelRequest,
    ) -> Result<Option<ModelReply>, ModelError> {
        if let Some(e) = self.stopped {
            return Err(ModelError::Limit(e));
        }
        let limits = self.limits;
        let nodes = request.input().nodes().len();
        if nodes > limits.max_nodes {
            return Err(self.stop(Limit::TreeSize, nodes as u64, limits.max_nodes as u64));
        }
        let elapsed = self.started.elapsed();
        if elapsed >= limits.max_time {
            return Err(self.stop(Limit::Time, elapsed.as_secs(), limits.max_time.as_secs()));
        }
        if self.calls >= limits.max_calls {
            return Err(self.stop(
                Limit::ModelCalls,
                u64::from(self.calls) + 1,
                u64::from(limits.max_calls),
            ));
        }
        let seen = self.requests.entry(fingerprint(request)).or_default();
        *seen += 1;
        if *seen > limits.max_repeats {
            let seen = u64::from(*seen);
            return Err(self.stop(Limit::Repeats, seen, u64::from(limits.max_repeats)));
        }

        self.calls += 1;
        let reply = provider
            .complete(request, Permit::new())
            .map_err(ModelError::Provider)?;
        if let Some(usage) = reply.as_ref().and_then(|r| r.usage) {
            self.tokens += usage.input_tokens + usage.output_tokens;
            if self.tokens > limits.max_tokens {
                // Bezahlt ist die Antwort schon; verwendet wird sie nicht.
                return Err(self.stop(Limit::Cost, self.tokens, limits.max_tokens));
            }
        }
        Ok(reply)
    }

    fn stop(&mut self, limit: Limit, used: u64, max: u64) -> ModelError {
        let e = LimitExceeded { limit, used, max };
        self.stopped = Some(e);
        ModelError::Limit(e)
    }
}

/// Gleiche Anfrage = gleicher Auftrag auf gleicher Eingabe.
fn fingerprint(request: &ModelRequest) -> u64 {
    let mut h = DefaultHasher::new();
    serde_json::to_string(request)
        .expect("ModelRequest ist serialisierbar")
        .hash(&mut h);
    h.finish()
}
