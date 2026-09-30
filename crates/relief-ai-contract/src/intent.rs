//! Intent-Vorschläge aus einer Nutzeräußerung (→ `plan/spezifikation/05`).
//!
//! Ein Modell übersetzt höchstens, was die Nutzerin gesagt hat, in einen
//! strukturierten Intent. Der Vorschlag ist keine Aktion: Ausführung erst
//! nach Validierung, Risikoeinstufung und ggf. Bestätigung in der Runtime.

use relief_model::{GraphVersion, NodeRef};
use serde::{Deserialize, Deserializer, Serialize};

use crate::hypothesis::{check_confidence, check_value};
use crate::{FilteredInput, ModelId, ValidationError};

/// JSON-Schema der Modellausgabe, handgeschrieben; dass es zu den Typen
/// passt, prüft ein Test.
pub const INTENT_SCHEMA: &str = include_str!("../schema/intent.schema.json");

/// Zulässige Werte für [`IntentKind::Scroll`].
pub const SCROLL_VALUES: [&str; 4] = ["down", "up", "top", "bottom"];

/// Intent-Katalog (→ `spezifikation/05`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    DescribePage,
    ListActions,
    ListHeadings,
    ListLandmarks,
    ListLinks,
    ListForms,
    ReadRegion,
    InspectControl,
    WhereAmI,
    NavigateTo,
    Focus,
    Activate,
    SetValue,
    Select,
    Increment,
    Decrement,
    Scroll,
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Required,
    Optional,
    Forbidden,
}

impl IntentKind {
    pub const ALL: [IntentKind; 18] = [
        IntentKind::DescribePage,
        IntentKind::ListActions,
        IntentKind::ListHeadings,
        IntentKind::ListLandmarks,
        IntentKind::ListLinks,
        IntentKind::ListForms,
        IntentKind::ReadRegion,
        IntentKind::InspectControl,
        IntentKind::WhereAmI,
        IntentKind::NavigateTo,
        IntentKind::Focus,
        IntentKind::Activate,
        IntentKind::SetValue,
        IntentKind::Select,
        IntentKind::Increment,
        IntentKind::Decrement,
        IntentKind::Scroll,
        IntentKind::Dismiss,
    ];

    fn target(self) -> Rule {
        use IntentKind::*;
        match self {
            InspectControl | NavigateTo | Focus | Activate | SetValue | Increment | Decrement => {
                Rule::Required
            }
            ReadRegion | Select | Dismiss => Rule::Optional,
            DescribePage | ListActions | ListHeadings | ListLandmarks | ListLinks | ListForms
            | WhereAmI | Scroll => Rule::Forbidden,
        }
    }

    fn value(self) -> Rule {
        match self {
            IntentKind::SetValue | IntentKind::Select | IntentKind::Scroll => Rule::Required,
            _ => Rule::Forbidden,
        }
    }
}

/// Was die Nutzerin eingegeben oder gesagt hat.
///
/// Entsteht in der Eingabeschicht (Befehlsleiste, Spracherkennung), nie aus
/// Seiteninhalt. Ein Modell bekommt sie getrennt von der [`FilteredInput`],
/// die nur Daten ist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct UserUtterance(String);

impl UserUtterance {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Ausgabe eines Modells, wie das Schema sie beschreibt. `target` und
/// `value` müssen vorhanden sein, dürfen aber `null` sein.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentOutput {
    pub intent: IntentKind,
    #[serde(deserialize_with = "nullable")]
    pub target: Option<TargetOutput>,
    #[serde(deserialize_with = "nullable")]
    pub value: Option<String>,
    pub confidence: f32,
    /// Die Äußerung, aus der der Intent stammt; muss die der Nutzerin sein.
    pub utterance: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetOutput {
    /// Lokale ID aus der Eingabe, z. B. `t0:18`.
    pub node: String,
    pub graph_version: u64,
}

/// Mit `deserialize_with` behandelt serde ein fehlendes `Option`-Feld als
/// Fehler statt als `None`; das Schema verlangt jedes Feld.
fn nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

/// Ein geprüfter Intent-Vorschlag. Noch keine Aktion.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IntentProposal {
    pub intent: IntentKind,
    pub target: Option<NodeRef>,
    /// Stand, auf den sich `target` bezieht; die Runtime prüft vor der
    /// Ausführung, ob er noch gilt (→ `spezifikation/05`, Validierung).
    pub graph_version: GraphVersion,
    pub value: Option<String>,
    /// Vom Modell genannt, nicht kalibriert.
    pub confidence: f32,
    pub utterance: String,
    pub model: ModelId,
}

/// Modellausgabe für einen Intent streng prüfen.
///
/// Geprüft: Schema, Confidence, die Äußerung ist wörtlich die der Nutzerin,
/// Ziel und Wert passen zum Intent, das Ziel existiert in der Eingabe und
/// bezieht sich auf ihren Stand; ein Wert für `set_value` steht in der
/// Äußerung (ein Modell erfindet keine Eingaben), `scroll` nur mit
/// [`SCROLL_VALUES`].
pub fn validate_intent(
    raw: &str,
    input: &FilteredInput,
    utterance: &UserUtterance,
    model: &ModelId,
) -> Result<IntentProposal, ValidationError> {
    let out: IntentOutput =
        serde_json::from_str(raw).map_err(|e| ValidationError::Format(e.to_string()))?;
    check_confidence(out.confidence)?;
    if out.utterance.trim() != utterance.as_str().trim() {
        return Err(ValidationError::Utterance);
    }

    let target = match (out.intent.target(), out.target) {
        (Rule::Required, None) => return Err(ValidationError::Target("fehlt")),
        (Rule::Forbidden, Some(_)) => return Err(ValidationError::Target("nicht vorgesehen")),
        (_, None) => None,
        (_, Some(t)) => {
            let at = input
                .node_ref(&t.node)
                .ok_or_else(|| ValidationError::UnknownNode(t.node.clone()))?;
            let expected = input.graph_version().0;
            if t.graph_version != expected {
                return Err(ValidationError::GraphVersion {
                    expected,
                    found: t.graph_version,
                });
            }
            Some(at.clone())
        }
    };

    match (out.intent.value(), &out.value) {
        (Rule::Required, None) => return Err(ValidationError::Value("fehlt")),
        (Rule::Forbidden, Some(_)) => return Err(ValidationError::Value("nicht vorgesehen")),
        (_, Some(v)) => {
            check_value(v)?;
            match out.intent {
                IntentKind::Scroll if !SCROLL_VALUES.contains(&v.as_str()) => {
                    return Err(ValidationError::Value("unbekannte Scrollrichtung"));
                }
                IntentKind::SetValue
                    if !utterance
                        .as_str()
                        .to_lowercase()
                        .contains(&v.trim().to_lowercase()) =>
                {
                    return Err(ValidationError::Value("steht nicht in der Äußerung"));
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(IntentProposal {
        intent: out.intent,
        target,
        graph_version: input.graph_version(),
        value: out.value,
        confidence: out.confidence,
        utterance: out.utterance,
        model: model.clone(),
    })
}
