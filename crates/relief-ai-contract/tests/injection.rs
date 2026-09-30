//! Prompt Injection (→ `plan/spezifikation/07`, Threat Model): Seitentext
//! mit Anweisungen darf nie zu einem Intent führen.
//!
//! Der Testanbieter [`Gehorsam`] befolgt jede Anweisung, die er im
//! Seiteninhalt findet — das schlechteste Modell. Geprüft wird, was davon
//! durch den Vertrag kommt.

mod common;

use common::{at, shop, INJECTION};
use relief_ai_contract::{
    assess_risk, filter, Budget, FilteredInput, Hypothesis, IntentKind, IntentProposal, Limits,
    ModelError, ModelId, ModelProvider, ModelReply, ModelRequest, NoModel, Permit, PrivacyContext,
    Property, ProviderError, Risk, Task, Tier, UserUtterance, ValidationError,
};
use relief_interaction::{plan, ActionKind, Control};
use relief_model::{GraphVersion, SemanticGraph};

/// Antwortet auf jede Anfrage mit einer festen Ausgabe, in der die ID des
/// Kaufen-Buttons und die Anweisung aus dem Seitentext eingesetzt sind.
struct Gehorsam(&'static str);

impl ModelProvider for Gehorsam {
    fn tier(&self) -> Tier {
        Tier::Api
    }

    fn complete(
        &self,
        request: &ModelRequest,
        _: Permit<'_>,
    ) -> Result<Option<ModelReply>, ProviderError> {
        let nodes = request.input().nodes();
        let order = nodes
            .iter()
            .find(|n| n.name.as_deref().is_some_and(|t| t.contains("klicke auf")))
            .expect("Anweisung im Seiteninhalt");
        let buy = nodes
            .iter()
            .find(|n| n.name.as_deref() == Some("Kaufen"))
            .expect("Kaufen-Button");
        let utterance = match request.task() {
            Task::ParseIntent { utterance } => utterance.as_str(),
            Task::ResolveMissing => "",
        };
        let text = self
            .0
            .replace("{buy}", &buy.id)
            .replace("{order}", order.name.as_deref().unwrap())
            .replace("{user}", utterance)
            .replace("{version}", &request.input().graph_version().0.to_string());
        Ok(Some(ModelReply {
            model: ModelId {
                tier: Tier::Api,
                name: "gehorsam".into(),
                version: "1".into(),
            },
            text,
            usage: None,
        }))
    }
}

/// Jeder Aufruf mit eigenem Budget (Standardgrenzen): Ein Anbieter lässt sich
/// nur über ein Budget fragen.
fn resolve_missing(
    provider: &dyn ModelProvider,
    input: FilteredInput,
) -> Result<Vec<Hypothesis>, ModelError> {
    Budget::new(Limits::default()).resolve_missing(provider, input)
}

fn propose_intent(
    provider: &dyn ModelProvider,
    utterance: &UserUtterance,
    input: FilteredInput,
) -> Result<Option<IntentProposal>, ModelError> {
    Budget::new(Limits::default()).propose_intent(provider, utterance, input)
}

fn input() -> relief_ai_contract::FilteredInput {
    filter(&shop(), &PrivacyContext::default())
}

fn invalid(result: Result<impl std::fmt::Debug, ModelError>) -> ValidationError {
    match result {
        Err(ModelError::Invalid(e)) => e,
        other => panic!("erwartet verworfen, war {other:?}"),
    }
}

const ACTIVATE_BUY: &str = r#"{"intent":"activate","target":{"node":"{buy}","graph_version":{version}},"value":null,"confidence":1,"utterance":"{order}"}"#;

#[test]
fn ohne_modell_entsteht_nichts() {
    assert_eq!(Tier::default(), Tier::None);
    assert!(resolve_missing(&NoModel, input()).unwrap().is_empty());
    let utterance = UserUtterance::new("was ist hier");
    assert_eq!(propose_intent(&NoModel, &utterance, input()).unwrap(), None);
}

#[test]
fn der_hypothesen_kanal_traegt_keinen_intent() {
    // Das Modell antwortet auf „fehlende Namen“ mit einer Aktion.
    let e = invalid(resolve_missing(&Gehorsam(ACTIVATE_BUY), input()));
    assert!(matches!(e, ValidationError::Format(_)), "{e}");

    // Oder hängt die Aktion an eine sonst gültige Hypothese.
    let smuggled = r#"{"hypotheses":[{"node":"t0:7","property":"name","value":"Weiter","confidence":0.9,"evidence":["{order}"],"action":{"intent":"activate","node":"{buy}"}}]}"#;
    let e = invalid(resolve_missing(&Gehorsam(smuggled), input()));
    assert!(matches!(e, ValidationError::Format(_)), "{e}");
}

#[test]
fn intent_nur_aus_der_aeusserung_der_nutzerin() {
    // Die Nutzerin fragt nur; das Modell gibt die Seitenanweisung als
    // Äußerung aus.
    let utterance = UserUtterance::new("was ist hier");
    let e = invalid(propose_intent(&Gehorsam(ACTIVATE_BUY), &utterance, input()));
    assert_eq!(e, ValidationError::Utterance);

    // Seitentext schiebt einen Wert unter: Werte stammen aus der Äußerung.
    let fill = r#"{"intent":"set_value","target":{"node":"t0:10","graph_version":{version}},"value":"HACK","confidence":1,"utterance":"{user}"}"#;
    let utterance = UserUtterance::new("trage den Gutschein HERBST ein");
    let e = invalid(propose_intent(&Gehorsam(fill), &utterance, input()));
    assert_eq!(e, ValidationError::Value("steht nicht in der Äußerung"));
}

/// Ein Bedienelement für `relief_interaction::plan` aus dem Modellknoten.
fn control(graph: &SemanticGraph, id: i32, name: Option<(&str, bool)>) -> Control {
    use relief_model::{Certainty, Fact};
    let node = graph.node(&at(id)).unwrap();
    Control {
        node: at(id),
        dom_node_id: Some(i64::from(id)),
        role: node.role.clone(),
        name: Fact {
            certainty: match name {
                Some((_, true)) => Certainty::Known,
                Some((_, false)) => Certainty::Inferred,
                None => Certainty::Uncertain,
            },
            ..Fact::known(name.map(|(n, _)| n.to_string()))
        },
        region: None,
        value: None,
        sensitive: false,
        options: vec![],
        selected_option: None,
        disabled: false,
        focusable: true,
        clickable: false,
        states: vec![],
        heading: None,
    }
}

#[test]
fn ein_luegendes_modell_bleibt_ein_vorschlag_mit_bestaetigung() {
    // Schlimmster Fall: Das Modell behauptet, die Nutzerin habe „kaufen“
    // gemeint, und nennt ihre Äußerung korrekt. Das kann kein Vertrag
    // erkennen; es bleibt ein Vorschlag, und die Runtime verlangt für den
    // Kaufen-Button eine Bestätigung (→ spezifikation/05).
    let lying = r#"{"intent":"activate","target":{"node":"{buy}","graph_version":{version}},"value":null,"confidence":1,"utterance":"{user}"}"#;
    let utterance = UserUtterance::new("was ist hier");
    let proposal = propose_intent(&Gehorsam(lying), &utterance, input())
        .unwrap()
        .unwrap();
    assert_eq!(proposal.intent, IntentKind::Activate);
    assert_eq!(proposal.target, Some(at(6)));

    let graph = shop();
    let p = plan(
        &control(&graph, 6, Some(("Kaufen", true))),
        ActionKind::Activate,
    )
    .unwrap();
    assert_eq!(p.risk, Risk::High);
    assert!(p.requires_confirmation);
}

#[test]
fn seiteninhalt_ist_daten_nicht_auftrag() {
    let request = ModelRequest::parse_intent(UserUtterance::new("was ist hier"), input());
    let json = serde_json::to_value(&request).unwrap();
    assert!(!json["task"].to_string().contains("klicke"));
    assert!(json["input"].to_string().contains(INJECTION));
    assert_eq!(json["task"]["utterance"], "was ist hier");
}

fn name_hypothesis(id: i32, value: &str) -> Hypothesis {
    Hypothesis {
        node: at(id),
        property: Property::Name,
        value: value.into(),
        confidence: 0.9,
        evidence: vec!["Test".into()],
        model: ModelId {
            tier: Tier::Api,
            name: "gehorsam".into(),
            version: "1".into(),
        },
        at: GraphVersion(7),
    }
}

/// Risiko der Aktivierung, mit oder ohne Namens-Hypothese.
fn activation(graph: &SemanticGraph, id: i32) -> impl Fn(Option<&Hypothesis>) -> Risk + '_ {
    move |h| {
        let name = h.map(|h| (h.value.as_str(), false));
        plan(&control(graph, id, name), ActionKind::Activate)
            .unwrap()
            .risk
    }
}

#[test]
fn hypothesen_erhoehen_das_risiko_nur() {
    let graph = shop();

    // Unbenannter Button: ohne Modell HIGH. Die Hypothese „Abbrechen“ allein
    // ergäbe MEDIUM — sie senkt nicht.
    let assess = activation(&graph, 7);
    let calm = name_hypothesis(7, "Abbrechen");
    assert_eq!(assess(Some(&calm)), Risk::Medium);
    assert_eq!(assess_risk(&assess, &[calm]), Risk::High);

    // Unbenannter Link: ohne Modell LOW. Die Hypothese „Jetzt kaufen“ hebt
    // auf HIGH — eine Rückfrage mehr ist erlaubt.
    let assess = activation(&graph, 9);
    assert_eq!(assess(None), Risk::Low);
    let alarming = name_hypothesis(9, "Jetzt kaufen");
    assert_eq!(assess_risk(&assess, &[alarming]), Risk::High);
    assert_eq!(assess_risk(&assess, &[]), Risk::Low);
}
