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
