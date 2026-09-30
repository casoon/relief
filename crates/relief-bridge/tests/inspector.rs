//! Inspector-Daten (Paket 20) auf der Aufnahme der sauberen Produktseite:
//! Einträge mit Herkunft, Auswahl über Schlüssel, „im Dokument zeigen“
//! ohne Auslösen.

// Nur die Stände vorher werden gebraucht.
#[allow(dead_code)]
mod common;

use relief_bridge::{inspector_json, Reply, Runtime, Step};
use relief_model::{Action, SemanticGraph, TreeDelta};
use serde_json::Value;

fn runtime_shop() -> Runtime {
    let pair = common::pairs()
        .into_iter()
        .find(|p| p.name.contains("01-shop-clean"))
        .expect("Aufnahme 01-shop-clean");
    let mut runtime = Runtime::new();
    runtime
        .apply(&TreeDelta::between(&SemanticGraph::default(), &pair.before))
        .unwrap();
    runtime
}

fn eintrag<'a>(view: &'a Value, liste: &str, label: &str) -> &'a Value {
    view[liste]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["label"].as_str().unwrap().starts_with(label))
        .unwrap_or_else(|| panic!("{liste}: {label} fehlt"))
}

#[test]
fn eintraege_mit_herkunft_zustaenden_und_aktionen() {
    let rt = runtime_shop();
    let view: Value = serde_json::from_str(&inspector_json(&rt)).unwrap();
    assert_eq!(view["title"], "Nike Air Max – Testshop");
    assert!(view["page"].as_str().unwrap().contains("Produktseite"));

    let groesse = eintrag(&view, "controls", "[combobox] Größe");
    assert_eq!(groesse["certainty"], "known");
    assert_eq!(groesse["origin"], "Chromium");
    assert_eq!(groesse["region"], "main");
    assert!(groesse["states"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s[0] == "hasPopup"));

    let h2 = eintrag(&view, "headings", "H2 Technische Daten");
    assert_eq!(h2["level"], 2);
    assert!(
        eintrag(&view, "regions", "navigation „Hauptmenü“")["reachable"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn zeigen_loest_nichts_aus() {
    let mut rt = runtime_shop();
    let view: Value = serde_json::from_str(&inspector_json(&rt)).unwrap();
    let kaufen = eintrag(&view, "controls", "[button] Jetzt kaufen")["key"]
        .as_str()
        .unwrap()
        .to_string();
    // Ein riskanter Button wird nur fokussiert, nie ausgelöst.
    let Reply::Perform(steps) = rt.show(&kaufen) else {
        panic!("Schritte erwartet")
    };
    assert!(matches!(steps.as_slice(), [Step::Ax(s)] if s.action == Action::Focus));

    let h2 = eintrag(&view, "headings", "H2 Technische Daten")["key"]
        .as_str()
        .unwrap()
        .to_string();
    let Reply::Perform(steps) = rt.show(&h2) else {
        panic!("Schritte erwartet")
    };
    assert_eq!(steps.len(), 2);
    assert!(rt
        .finish()
        .contains("Fokus jetzt auf heading „Technische Daten“"));

    assert_eq!(
        rt.show("unbekannt#1"),
        Reply::Answer("Eintrag ist nicht mehr auf der Seite.".into())
    );
}

/// Paket 100: Der Inspector zeigt bei einem sensiblen Feld nur, dass es einen
/// Wert gibt, weder in der Kurzzeile noch unter „Wert“.
#[test]
fn sensibler_wert_ist_verdeckt() {
    let pair = common::pairs()
        .into_iter()
        .find(|p| p.name.contains("01-shop-clean"))
        .expect("Aufnahme 01-shop-clean");
    let mut graph = pair.before;
    let suche = graph
        .document_order()
        .into_iter()
        .find(|at| graph.node(at).unwrap().role == relief_model::Role::SearchBox)
        .unwrap();
    let feld = graph
        .trees
        .get_mut(&suche.tree)
        .unwrap()
        .nodes
        .get_mut(&suche.node)
        .unwrap();
    feld.value = relief_model::Fact::known(Some("4111111111111111".into()));
    feld.extra.insert(
        relief_interaction::security::HTML_AUTOCOMPLETE.into(),
        "cc-number".into(),
    );
    let mut rt = Runtime::new();
    rt.apply(&TreeDelta::between(&SemanticGraph::default(), &graph))
        .unwrap();

    let json = inspector_json(&rt);
    assert!(!json.contains("4111111111111111"), "{json}");
    let view: Value = serde_json::from_str(&json).unwrap();
    let feld = eintrag(&view, "controls", "[searchbox]");
    assert_eq!(feld["value"], "(verdeckt)");
    let label = feld["label"].as_str().unwrap();
    assert!(label.ends_with("= (verdeckt)"), "{label}");
}

/// Semantic View (Paket 29): Einträge in Dokumentreihenfolge mit Bereich;
/// Bedienung läuft als validierte Aktion, riskant nur nach Rückfrage.
#[test]
fn semantische_ansicht_bedient_ueber_validierte_aktionen() {
    let mut rt = runtime_shop();
    let view: Value = serde_json::from_str(&inspector_json(&rt)).unwrap();
    let eintraege = view["semantic"].as_array().unwrap();
    let text = |e: &Value| e["text"].as_str().unwrap().to_string();
    let pos = |t: &str| {
        eintraege
            .iter()
            .position(|e| text(e).starts_with(t))
            .unwrap_or_else(|| panic!("{t} fehlt"))
    };
    // Reihenfolge wie im Dokument: Überschrift vor Warenkorb-Button.
    assert!(pos("Nike Air Max") < pos("In den Warenkorb"));
    let h1 = &eintraege[pos("Nike Air Max")];
    assert_eq!(h1["kind"], "heading");
    assert_eq!(h1["level"], 1);
    // Linktext nur einmal: als Link, nicht zusätzlich als Text.
    assert_eq!(
        eintraege.iter().filter(|e| text(e) == "Impressum").count(),
        1
    );
    let kaufen = &eintraege[pos("Jetzt kaufen")];
    assert_eq!(kaufen["control"], "button");
    assert!(kaufen["region"].as_str().is_some());

    // Riskant: erst Rückfrage, keine Schritte; „ja“ führt aus.
    let key = kaufen["key"].as_str().unwrap();
    match rt.view_act(key, "activate", "") {
        Reply::Answer(t) => assert!(t.starts_with("Bestätigung nötig"), "{t}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(rt.command("ja"), Reply::Perform(_)));

    // Unbekannter Schlüssel: nichts ausgeführt.
    assert!(matches!(
        rt.view_act("x#1", "activate", ""),
        Reply::Answer(_)
    ));
}
