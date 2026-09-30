//! Relief: Vertrag zwischen Runtime und Modellen.
//!
//! Browserfrei. Legt fest, was ein Modell sehen darf, was es liefern darf und
//! was aus seiner Ausgabe werden kann (→ `plan/spezifikation/06`, `07`, `05`):
//!
//! | Baustein | Inhalt |
//! |---|---|
//! | [`filter`], [`FilteredInput`], [`PrivacyContext`] | Privacy-Filter: einziger Weg zu einer Modelleingabe |
//! | [`ModelProvider`], [`Tier`], [`NoModel`] | Anbieter-Schnittstelle, Stufen `none`/`os`/`local`/`api` |
//! | [`resolve_missing`], [`propose_intent`] | Runtime-Aufrufe: Anbieter fragen, Ausgabe streng prüfen |
//! | [`Hypothesis`], [`HYPOTHESES_SCHEMA`] | fehlende Semantik als Hypothese mit Pflichtmetadaten |
//! | [`IntentProposal`], [`INTENT_SCHEMA`] | Intent-Vorschlag aus einer Nutzeräußerung |
//! | [`assess_risk`] | Hypothesen dürfen die Risikoklasse nur erhöhen |
//!
//! # Grenzen, die das Typsystem zieht
//!
//! - Ein [`ModelProvider`] bekommt nur einen [`ModelRequest`], und der
//!   entsteht nur aus einer [`FilteredInput`]. Die hat keinen öffentlichen
//!   Konstruktor und kein `Deserialize`; einziger Ursprung ist [`filter`].
//! - Ein Anbieter liefert nur Text. Zu [`Hypothesis`] oder [`IntentProposal`]
//!   wird er erst durch die Prüfung hier, gegen die Eingabe, aus der er
//!   entstand.
//! - Eine Hypothese ist eine Aussage, keine Aktion: Es gibt keinen Weg von
//!   [`Hypothesis`] zu einem Intent oder `ActionPlan`. Ein
//!   [`IntentProposal`] ist ein Vorschlag; ausgeführt wird erst nach der
//!   Validierung in der Runtime (Risiko, Bestätigung, → `spezifikation/05`).

#![forbid(unsafe_code)]

mod error;
mod hypothesis;
mod intent;
mod privacy;
mod provider;
mod risk;

pub use error::ValidationError;
pub use hypothesis::{
    calibrated_threshold, validate_hypotheses, HypothesesOutput, Hypothesis, HypothesisOutput,
    Property, CALIBRATED_THRESHOLDS, HYPOTHESES_SCHEMA, MAX_EVIDENCE, MAX_EVIDENCE_CHARS,
    MAX_HYPOTHESES, MAX_VALUE_CHARS,
};
pub use intent::{
    validate_intent, IntentKind, IntentOutput, IntentProposal, TargetOutput, UserUtterance,
    INTENT_SCHEMA, SCROLL_VALUES,
};
pub use privacy::{
    filter, FieldHint, FilteredInput, ModelNode, PageInfo, PrivacyContext, Redaction,
};
pub use provider::{
    propose_intent, resolve_missing, ModelError, ModelId, ModelProvider, ModelReply, ModelRequest,
    NoModel, ProviderError, Task, Tier, Usage,
};
pub use risk::{assess_risk, Risk};
