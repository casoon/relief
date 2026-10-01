//! Eigene CDP-Domäne `Relief.*` (Paket 45): Playwright und andere
//! CDP-Werkzeuge fragen das Seitenmodell des Forks ab, ohne das Testsystem
//! zu wechseln (→ `plan/spezifikation/12`, Linie B).
//!
//! | Methode | Parameter | Ergebnis |
//! |---|---|---|
//! | `Relief.getPageModel` | — | Seitentyp mit Herkunft, Gruppen mit ihren Bedienelementen, primäre Aktion |
//! | `Relief.assert` | `assertion`: Zusicherung wie `assert:` in Aufgabendateien | Befunde (`a11y-report`), Zahl der Fehler |
//!
//! Zusicherungen, die DOM-Fakten oder eine Tab-Folge brauchen
//! (`namen-wie-accname`, `tabfolge`), stehen im Fork nicht bereit; sie
//! liefern `Untested` bzw. einen Fehler, nie ein Bestanden.

use relief_interaction::assertions::{check, Assertion, Observed};
use relief_interaction::{focused, respond, Graph};
use serde_json::{json, Value};

use crate::runtime::Runtime;

/// Eine Methode der Domäne; Fehler: Text für die CDP-Fehlerantwort.
pub fn command(runtime: &Runtime, method: &str, params: &str) -> Result<Value, String> {
    let params: Value = if params.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(params).map_err(|e| format!("Parameter kein JSON: {e}"))?
    };
    let model = runtime.graph();
    match method {
        "Relief.getPageModel" => {
            let graph = Graph::build(model);
            let label = |i: usize| respond::control_line(&graph.controls[i]);
            let groups: Vec<Value> = graph
                .page
                .groups
                .iter()
                .map(|g| {
                    json!({
                        "kind": g.kind,
                        "name": g.name,
                        "controls": g.controls.iter().map(|&i| label(i)).collect::<Vec<_>>(),
                    })
                })
                .collect();
            let primary = graph.page.primary.as_ref().map(|p| {
                json!({
                    "control": p.value.map(label),
                    "certainty": p.certainty,
                })
            });
            Ok(json!({
                "pageType": graph.page.kind,
                "groups": groups,
                "primaryAction": primary,
                "controls": graph.controls.len(),
            }))
        }
        "Relief.assert" => {
            let text = params
                .get("assertion")
                .and_then(Value::as_str)
                .ok_or("Parameter „assertion“ fehlt")?;
            let assertion = Assertion::parse(text)?;
            if assertion.needs_tab_walk() {
                return Err(format!(
                    "„{text}“ braucht eine beobachtete Tab-Folge; im Fork nicht verfügbar."
                ));
            }
            let findings = check(
                &assertion,
                &Observed {
                    model,
                    dom: None,
                    focus: focused(model),
                    before: None,
                    tab_sequence: None,
                },
            );
            let failed = findings
                .iter()
                .filter(|f| f.outcome == a11y_report::Outcome::Fail)
                .count();
            Ok(json!({ "findings": findings, "failed": failed }))
        }
        other => Err(format!("Unbekannte Methode {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relief_model::{
        Fact, NodeId, Role, SemanticGraph, SemanticNode, TreeData, TreeDelta, TreeId, TreeUpdate,
    };

    fn runtime(field_name: Option<&str>) -> Runtime {
        let node = |id: i32, role: &str, name: Option<&str>, children: &[i32]| {
            let mut n = SemanticNode::new(NodeId(id), Role::from_name(role));
            n.name = Fact::known(name.map(String::from));
            n.children = children.iter().map(|c| NodeId(*c)).collect();
            if id != 1 {
                n.parent = Some(NodeId(if id == 2 { 1 } else { 2 }));
            }
            n
        };
        let tree = TreeId("t".into());
        let mut rt = Runtime::new();
        rt.apply(&TreeDelta {
            base: SemanticGraph::default().version,
            root: Some(tree.clone()),
            removed_trees: vec![],
            trees: vec![TreeUpdate {
                tree,
                data: Some(TreeData::default()),
                root: Some(NodeId(1)),
                removed: vec![],
                created: vec![
                    node(1, "rootWebArea", Some("Kontakt"), &[2]),
                    node(2, "form", Some("Kontakt"), &[3, 4]),
                    node(3, "textbox", field_name, &[]),
                    node(4, "button", Some("Senden"), &[]),
                ],
                changed: vec![],
                bounds: vec![],
            }],
        })
        .unwrap();
        rt
    }

    #[test]
    fn seitenmodell_und_zusicherung() {
        let rt = runtime(Some("Name"));
        let page = command(&rt, "Relief.getPageModel", "").unwrap();
        assert_eq!(page["controls"], 2, "{page}");
        assert!(page["pageType"]["value"].is_string(), "{page}");

        let ok = command(&rt, "Relief.assert", r#"{"assertion":"feldnamen"}"#).unwrap();
        assert_eq!(ok["failed"], 0, "{ok}");
        let kaputt = command(
            &runtime(None),
            "Relief.assert",
            r#"{"assertion":"feldnamen"}"#,
        )
        .unwrap();
        assert_eq!(kaputt["failed"], 1, "{kaputt}");
        assert_eq!(
            kaputt["findings"][0]["rule_id"], "form/field-name",
            "{kaputt}"
        );
    }

    #[test]
    fn fehler_statt_bestanden() {
        let rt = runtime(Some("Name"));
        assert!(command(&rt, "Relief.assert", r#"{"assertion":"tabfolge Name"}"#).is_err());
        assert!(command(&rt, "Relief.assert", "{}").is_err());
        assert!(command(&rt, "Relief.gibtEsNicht", "").is_err());
        let dom = command(&rt, "Relief.assert", r#"{"assertion":"namen-wie-accname"}"#).unwrap();
        assert_eq!(dom["findings"][0]["outcome"], "untested", "{dom}");
    }
}
