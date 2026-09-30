//! Resolver-Eingabe: Ausschnitt um einen Knoten, auf Testseiten und auf der
//! Wikipedia-Aufnahme.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use a11y_perception::AXSnapshot;
use common::shop;
use relief_ai_contract::{
    filter, validate_hypotheses, FilteredInput, ModelId, ModelNode, PrivacyContext, Tier,
    ValidationError,
};
use relief_model::{perception, TreeId};

fn wikipedia() -> FilteredInput {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../spike/recordings/10-real/02-en-wikipedia-org-wiki-accessibility/snapshot-00.json",
    );
    let snap: AXSnapshot = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let graph = perception::from_snapshot(&snap, &TreeId(snap.url.clone()));
    filter(&graph, &PrivacyContext::default())
}

/// Jeder Knoten des Ausschnitts steht unverändert in der Eingabe, bis auf
/// `parent`, der ein Vorfahr in der Eingabe sein muss.
fn assert_only_removes(full: &FilteredInput, part: &FilteredInput) {
    let by_id: BTreeMap<&str, &ModelNode> =
        full.nodes().iter().map(|n| (n.id.as_str(), n)).collect();
    let ancestors = |id: &str| {
        let mut out = Vec::new();
        let mut up = by_id[id].parent.clone();
        while let Some(p) = up {
            up = by_id[p.as_str()].parent.clone();
            out.push(p);
        }
        out
    };
    for node in part.nodes() {
        let original = by_id
            .get(node.id.as_str())
            .unwrap_or_else(|| panic!("{} nicht in der Eingabe", node.id));
        assert_eq!(
            ModelNode {
                parent: None,
                ..node.clone()
            },
            ModelNode {
                parent: None,
                ..(*original).clone()
            }
        );
        if let Some(p) = &node.parent {
            assert!(ancestors(&node.id).contains(p), "{}: {p}", node.id);
        }
        assert_eq!(part.node_ref(&node.id), full.node_ref(&node.id));
    }
    assert_eq!(part.graph_version(), full.graph_version());
    assert_eq!(part.page(), full.page());
}

#[test]
fn ausschnitt_enthaelt_fokus_vorfahren_und_nachbarn() {
    let full = filter(&shop(), &PrivacyContext::default());
    let part = full.excerpt("t0:7", 3).unwrap();
    assert_eq!(part.focus(), Some("t0:7"));
    let ids: Vec<&str> = part.nodes().iter().map(|n| n.id.as_str()).collect();
    // Vorfahren (Wurzel, main, Formular), Fokus und Nachbarn in
    // Dokumentreihenfolge; das Fenster liegt im Formular.
    assert_eq!(ids, ["t0:1", "t0:2", "t0:5", "t0:6", "t0:7", "t0:10"]);
    assert_only_removes(&full, &part);
    assert!(full.excerpt("t0:99", 10).is_none());
}

#[test]
fn ausschnitt_ist_begrenzt_und_nimmt_nur_weg() {
    let full = wikipedia();
    // Rückverweis „b“ in den Einzelnachweisen, Bild-Link in einer Abbildung.
    for id in ["t0:4062", "t0:1690"] {
        let part = full.excerpt(id, 40).unwrap();
        assert_only_removes(&full, &part);
        let ancestors = {
            let mut n = 0;
            let mut up = full
                .nodes()
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .parent
                .clone();
            while let Some(p) = up {
                n += 1;
                up = full
                    .nodes()
                    .iter()
                    .find(|m| m.id == p)
                    .unwrap()
                    .parent
                    .clone();
            }
            n
        };
        assert!(part.nodes().len() <= 40 + ancestors, "{id}");
        assert!(part.nodes().len() * 20 < full.nodes().len(), "{id}");
        assert!(part.nodes().iter().any(|n| n.id == id));
    }
}

#[test]
fn modellausgaben_nur_zu_knoten_im_ausschnitt() {
    let full = filter(&shop(), &PrivacyContext::default());
    let part = full.excerpt("t0:7", 3).unwrap();
    let model = ModelId {
        tier: Tier::Api,
        name: "test".into(),
        version: "1".into(),
    };
    let reply = |node: &str| {
        format!(
            r#"{{"hypotheses":[{{"node":"{node}","property":"name","value":"x","confidence":0.5,"evidence":["e"]}}]}}"#
        )
    };
    // t0:9 (unbenannter Link) gibt es in der ganzen Eingabe, nicht im Ausschnitt.
    assert!(validate_hypotheses(&reply("t0:9"), &full, &model).is_ok());
    assert_eq!(
        validate_hypotheses(&reply("t0:9"), &part, &model),
        Err(ValidationError::UnknownNode("t0:9".into()))
    );
    assert!(validate_hypotheses(&reply("t0:7"), &part, &model).is_ok());
}

#[test]
fn fokus_steht_in_der_anfrage() {
    let full = filter(&shop(), &PrivacyContext::default());
    let json = serde_json::to_string(&full.excerpt("t0:7", 3).unwrap()).unwrap();
    assert!(json.contains(r#""focus":"t0:7""#));
    assert!(!serde_json::to_string(&full)
        .unwrap()
        .contains(r#""focus":"#));
}
