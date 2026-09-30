//! Die handgeschriebenen JSON-Schemas passen zu den Rust-Typen.

use std::collections::BTreeSet;

use relief_ai_contract::{
    HypothesesOutput, HypothesisOutput, IntentKind, IntentOutput, Property, TargetOutput,
    HYPOTHESES_SCHEMA, INTENT_SCHEMA, MAX_EVIDENCE, MAX_EVIDENCE_CHARS, MAX_HYPOTHESES,
    MAX_VALUE_CHARS,
};
use serde_json::Value;

fn parse(schema: &str) -> Value {
    serde_json::from_str(schema).expect("Schema ist kein JSON")
}

fn types(schema: &Value) -> Vec<&str> {
    match &schema["type"] {
        Value::String(t) => vec![t.as_str()],
        Value::Array(ts) => ts.iter().map(|t| t.as_str().unwrap()).collect(),
        other => panic!("type fehlt: {other}"),
    }
}

/// Ein serialisierter Wert erfüllt das Schema strukturell: Typen, und bei
/// Objekten sind `properties`, `required` und die Felder des Werts dieselben
/// Namen, `additionalProperties` ist `false`.
fn conforms(schema: &Value, value: &Value, path: &str) {
    let allowed = types(schema);
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_u64() && allowed.contains(&"integer") => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    assert!(
        allowed.contains(&kind),
        "{path}: {kind} nicht in {allowed:?}"
    );
    match value {
        Value::Object(fields) => {
            assert_eq!(schema["additionalProperties"], false, "{path}");
            let props: BTreeSet<&str> = schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            let required: BTreeSet<&str> = schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap())
                .collect();
            let actual: BTreeSet<&str> = fields.keys().map(String::as_str).collect();
            assert_eq!(props, required, "{path}: nicht jedes Feld ist Pflicht");
            assert_eq!(props, actual, "{path}: Schema und Typ nennen andere Felder");
            for (key, v) in fields {
                conforms(&schema["properties"][key], v, &format!("{path}.{key}"));
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                conforms(&schema["items"], v, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

fn names<T: serde::Serialize>(all: &[T]) -> Vec<Value> {
    all.iter()
        .map(|v| serde_json::to_value(v).unwrap())
        .collect()
}

#[test]
fn hypothesen_schema_passt_zu_typen() {
    let schema = parse(HYPOTHESES_SCHEMA);
    let example = HypothesesOutput {
        hypotheses: vec![HypothesisOutput {
            node: "t0:18".into(),
            property: Property::Name,
            value: "Warenkorb".into(),
            confidence: 0.5,
            evidence: vec!["href=/cart".into()],
        }],
    };
    let value = serde_json::to_value(&example).unwrap();
    conforms(&schema, &value, "$");
    assert_eq!(
        serde_json::from_value::<HypothesesOutput>(value).unwrap(),
        example
    );

    let list = &schema["properties"]["hypotheses"];
    let item = &list["items"]["properties"];
    assert_eq!(
        item["property"]["enum"].as_array().unwrap(),
        &names(&Property::ALL)
    );
    assert_eq!(list["maxItems"], MAX_HYPOTHESES);
    assert_eq!(item["value"]["maxLength"], MAX_VALUE_CHARS);
    assert_eq!(item["evidence"]["maxItems"], MAX_EVIDENCE);
    assert_eq!(item["evidence"]["items"]["maxLength"], MAX_EVIDENCE_CHARS);
    assert_eq!(
        (
            item["confidence"]["minimum"].as_f64(),
            item["confidence"]["maximum"].as_f64()
        ),
        (Some(0.0), Some(1.0))
    );
}

#[test]
fn intent_schema_passt_zu_typen() {
    let schema = parse(INTENT_SCHEMA);
    let full = IntentOutput {
        intent: IntentKind::SetValue,
        target: Some(TargetOutput {
            node: "t0:10".into(),
            graph_version: 7,
        }),
        value: Some("Müller".into()),
        confidence: 0.9,
        utterance: "trage Müller ein".into(),
    };
    let minimal = IntentOutput {
        intent: IntentKind::DescribePage,
        target: None,
        value: None,
        confidence: 0.9,
        utterance: "was ist hier".into(),
    };
    for example in [&full, &minimal] {
        let value = serde_json::to_value(example).unwrap();
        conforms(&schema, &value, "$");
        assert_eq!(
            &serde_json::from_value::<IntentOutput>(value).unwrap(),
            example
        );
    }

    // Genau die `Option`-Felder sind im Schema `null`-fähig.
    let value = serde_json::to_value(&minimal).unwrap();
    for (key, v) in value.as_object().unwrap() {
        assert_eq!(
            types(&schema["properties"][key]).contains(&"null"),
            v.is_null(),
            "{key}"
        );
    }
    assert_eq!(
        schema["properties"]["intent"]["enum"].as_array().unwrap(),
        &names(&IntentKind::ALL)
    );
    assert_eq!(schema["properties"]["value"]["maxLength"], MAX_VALUE_CHARS);
}

#[test]
fn pflichtfelder_und_unbekannte_felder_wie_im_schema() {
    // `null` ist erlaubt, Weglassen nicht (Schema: alle Felder required).
    let ok =
        r#"{"intent":"describe_page","target":null,"value":null,"confidence":1,"utterance":"x"}"#;
    assert!(serde_json::from_str::<IntentOutput>(ok).is_ok());
    let missing = r#"{"intent":"describe_page","value":null,"confidence":1,"utterance":"x"}"#;
    assert!(serde_json::from_str::<IntentOutput>(missing).is_err());
    let extra = r#"{"intent":"describe_page","target":null,"value":null,"confidence":1,"utterance":"x","action":"click"}"#;
    assert!(serde_json::from_str::<IntentOutput>(extra).is_err());
    let nested = r#"{"hypotheses":[{"node":"t0:1","property":"name","value":"x","confidence":1,"evidence":["e"],"then":"click"}]}"#;
    assert!(serde_json::from_str::<HypothesesOutput>(nested).is_err());
}
