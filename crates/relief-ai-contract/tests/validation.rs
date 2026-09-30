//! Strenge Prüfung von Modellausgaben gegen die Eingabe.

mod common;

use common::{at, shop};
use relief_ai_contract::{
    filter, validate_hypotheses, validate_intent, FieldHint, FilteredInput, IntentKind, ModelId,
    PrivacyContext, Property, Tier, UserUtterance, ValidationError,
};
use relief_model::{Certainty, GraphVersion, Source};

fn model() -> ModelId {
    ModelId {
        tier: Tier::Local,
        name: "test".into(),
        version: "1".into(),
    }
}

fn input() -> FilteredInput {
    filter(&shop(), &PrivacyContext::default())
}

fn hypothesis(node: &str, property: &str, value: &str, confidence: f32, evidence: &str) -> String {
    format!(
        r#"{{"node":"{node}","property":"{property}","value":"{value}","confidence":{confidence},"evidence":{evidence}}}"#
    )
}

fn hypotheses(items: &[String]) -> String {
    format!(r#"{{"hypotheses":[{}]}}"#, items.join(","))
}

fn check(items: &[String]) -> Result<Vec<relief_ai_contract::Hypothesis>, ValidationError> {
    validate_hypotheses(&hypotheses(items), &input(), &model())
}

#[test]
fn gueltige_hypothese_traegt_pflichtmetadaten() {
    let hs = check(&[hypothesis(
        "t0:7",
        "name",
        "In den Warenkorb",
        0.8,
        r#"["parent: form Bestellung","neighbour: Kaufen"]"#,
    )])
    .unwrap();
    let h = &hs[0];
    assert_eq!(h.node, at(7));
    assert_eq!(h.property, Property::Name);
    assert_eq!(h.at, GraphVersion(7));
    assert_eq!(h.model.to_string(), "local:test@1");

    let fact = h.fact();
    assert_eq!(fact.value.as_deref(), Some("In den Warenkorb"));
    assert_eq!(fact.certainty, Certainty::Uncertain);
    assert_eq!(fact.source, Source::Model("local:test@1".into()));
    assert_eq!((fact.confidence, fact.evidence.len()), (Some(0.8), 2));
}

#[test]
fn verstoesse_verwerfen_die_ganze_ausgabe() {
    let ok = hypothesis("t0:9", "name", "Merken", 0.5, r#"["icon: herz"]"#);
    let cases = [
        (
            hypothesis("t0:99", "name", "x", 0.5, r#"["e"]"#),
            ValidationError::UnknownNode("t0:99".into()),
        ),
        (
            hypothesis("t1:7", "name", "x", 0.5, r#"["e"]"#),
            ValidationError::UnknownNode("t1:7".into()),
        ),
        (
            // Chromium kennt den Namen schon.
            hypothesis("t0:6", "name", "Bezahlen", 0.5, r#"["e"]"#),
            ValidationError::NotMissing {
                node: "t0:6".into(),
                property: Property::Name,
            },
        ),
        (
            hypothesis("t0:7", "name", "x", 1.5, r#"["e"]"#),
            ValidationError::Confidence(1.5),
        ),
        (
            hypothesis("t0:7", "name", "x", -0.1, r#"["e"]"#),
            ValidationError::Confidence(-0.1),
        ),
        (
            hypothesis("t0:7", "name", "  ", 0.5, r#"["e"]"#),
            ValidationError::Value("leer"),
        ),
        (
            hypothesis("t0:7", "name", &"x".repeat(201), 0.5, r#"["e"]"#),
            ValidationError::Value("zu lang"),
        ),
        (
            hypothesis("t0:7", "name", "x", 0.5, "[]"),
            ValidationError::Evidence("fehlt"),
        ),
        (
            hypothesis(
                "t0:7",
                "name",
                "x",
                0.5,
                r#"["1","2","3","4","5","6","7","8","9"]"#,
            ),
            ValidationError::Evidence("zu viele Einträge"),
        ),
        (
            hypothesis("t0:7", "description", "x", 0.5, r#"[""]"#),
            ValidationError::Evidence("Eintrag leer oder zu lang"),
        ),
        (
            ok.clone(),
            ValidationError::Duplicate {
                node: "t0:9".into(),
                property: Property::Name,
            },
        ),
    ];
    for (bad, expected) in cases {
        assert_eq!(check(&[ok.clone(), bad]).unwrap_err(), expected);
    }

    let too_many: Vec<String> = (0..51).map(|_| ok.clone()).collect();
    assert_eq!(check(&too_many).unwrap_err(), ValidationError::TooMany(51));

    for raw in [
        "",
        "Der Button heißt Warenkorb.",
        r#"[{"node":"t0:7"}]"#,
        r#"{"hypotheses":[{"node":"t0:7","property":"role","value":"x","confidence":0.5,"evidence":["e"]}]}"#,
        r#"{"hypotheses":[{"node":"t0:7","property":"name","value":"x","confidence":0.5}]}"#,
        r#"{"hypotheses":[],"note":"x"}"#,
    ] {
        assert!(
            matches!(
                validate_hypotheses(raw, &input(), &model()),
                Err(ValidationError::Format(_))
            ),
            "{raw}"
        );
    }
}

#[test]
fn keine_hypothese_zu_sensiblen_feldern() {
    let mut context = PrivacyContext::default();
    context.fields.insert(
        at(10),
        FieldHint {
            input_type: None,
            autocomplete: Some("cc-number".into()),
        },
    );
    let input = filter(&shop(), &context);
    let raw = hypotheses(&[hypothesis("t0:10", "description", "x", 0.5, r#"["e"]"#)]);
    assert_eq!(
        validate_hypotheses(&raw, &input, &model()).unwrap_err(),
        ValidationError::Redacted("t0:10".into())
    );
}

fn intent(kind: &str, target: &str, value: &str, utterance: &str) -> String {
    format!(
        r#"{{"intent":"{kind}","target":{target},"value":{value},"confidence":0.9,"utterance":"{utterance}"}}"#
    )
}

fn check_intent(
    raw: &str,
    utterance: &str,
) -> Result<relief_ai_contract::IntentProposal, ValidationError> {
    validate_intent(raw, &input(), &UserUtterance::new(utterance), &model())
}

#[test]
fn gueltiger_intent_verweist_auf_den_knoten() {
    let p = check_intent(
        &intent(
            "activate",
            r#"{"node":"t0:8","graph_version":7}"#,
            "null",
            "öffne den Einkaufswagen",
        ),
        "öffne den Einkaufswagen",
    )
    .unwrap();
    assert_eq!(p.intent, IntentKind::Activate);
    assert_eq!(p.target, Some(at(8)));
    assert_eq!(p.graph_version, GraphVersion(7));

    let p = check_intent(
        &intent(
            "set_value",
            r#"{"node":"t0:10","graph_version":7}"#,
            r#""HERBST""#,
            "Gutschein HERBST eintragen",
        ),
        "Gutschein HERBST eintragen",
    )
    .unwrap();
    assert_eq!(p.value.as_deref(), Some("HERBST"));

    let p = check_intent(&intent("scroll", "null", r#""down""#, "runter"), "runter").unwrap();
    assert_eq!(p.target, None);
}

#[test]
fn intent_verstoesse() {
    let target = r#"{"node":"t0:8","graph_version":7}"#;
    let cases = [
        (
            intent("activate", "null", "null", "öffne x"),
            "öffne x",
            ValidationError::Target("fehlt"),
        ),
        (
            intent("describe_page", target, "null", "was ist hier"),
            "was ist hier",
            ValidationError::Target("nicht vorgesehen"),
        ),
        (
            intent(
                "activate",
                r#"{"node":"t0:8","graph_version":6}"#,
                "null",
                "öffne x",
            ),
            "öffne x",
            ValidationError::GraphVersion {
                expected: 7,
                found: 6,
            },
        ),
        (
            intent(
                "activate",
                r#"{"node":"t0:42","graph_version":7}"#,
                "null",
                "öffne x",
            ),
            "öffne x",
            ValidationError::UnknownNode("t0:42".into()),
        ),
        (
            intent("activate", target, r#""x""#, "öffne x"),
            "öffne x",
            ValidationError::Value("nicht vorgesehen"),
        ),
        (
            intent("set_value", target, "null", "fülle x"),
            "fülle x",
            ValidationError::Value("fehlt"),
        ),
        (
            intent("scroll", "null", r#""seitwärts""#, "scroll"),
            "scroll",
            ValidationError::Value("unbekannte Scrollrichtung"),
        ),
        (
            intent("activate", target, "null", "öffne x"),
            "öffne y",
            ValidationError::Utterance,
        ),
    ];
    for (raw, utterance, expected) in cases {
        assert_eq!(
            check_intent(&raw, utterance).unwrap_err(),
            expected,
            "{raw}"
        );
    }
    for raw in [
        intent("buy", "null", "null", "x"),
        r#"{"intent":"activate","target":{"node":"t0:8"},"value":null,"confidence":0.9,"utterance":"x"}"#.into(),
        r#"{"intent":"describe_page","confidence":0.9,"utterance":"x"}"#.into(),
    ] {
        assert!(
            matches!(check_intent(&raw, "x"), Err(ValidationError::Format(_))),
            "{raw}"
        );
    }
}
