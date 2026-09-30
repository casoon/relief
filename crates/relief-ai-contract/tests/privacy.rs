//! Privacy-Filter auf den Aufnahmen `spike/recordings/03-form` und
//! `04-iframe` sowie auf Login-, Zahlungs- und Checkout-Seiten.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use common::{at, field, node, page, typed};
use relief_ai_contract::{filter, FieldHint, FilteredInput, ModelNode, PrivacyContext, Redaction};
use relief_model::{perception, Role, SemanticGraph, TreeId};

fn recording(dir: &str) -> Vec<(String, SemanticGraph)> {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spike/recordings")
        .join(dir);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("Aufnahme fehlt")
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("snapshot-")
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "{}: keine Aufnahmen", dir.display());
    files
        .into_iter()
        .map(|p| {
            let snap: AXSnapshot =
                serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            let graph = perception::from_snapshot(&snap, &TreeId(snap.url.clone()));
            (p.file_name().unwrap().to_string_lossy().into_owned(), graph)
        })
        .collect()
}

fn json(input: &FilteredInput) -> String {
    serde_json::to_string(input).unwrap()
}

fn by_name<'a>(input: &'a FilteredInput, name: &str) -> &'a ModelNode {
    input
        .nodes()
        .iter()
        .find(|n| n.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("kein Knoten „{name}“"))
}

fn by_id<'a>(input: &'a FilteredInput, id: &str) -> &'a ModelNode {
    input.nodes().iter().find(|n| n.id == id).unwrap()
}

#[test]
fn formularwerte_der_aufnahme_gehen_nie_an_ein_modell() {
    for (file, graph) in recording("03-form/01-form") {
        let input = filter(&graph, &PrivacyContext::default());
        let text = json(&input);
        // In der Aufnahme eingetragen (snapshot-01 ff.).
        for value in ["Erika Muster", "erika@example.org"] {
            assert!(!text.contains(value), "{file}: „{value}“ im Modell-Input");
        }
        // Zustand der Checkbox ist Eingabe; Inline-Textboxen sind Doppel.
        assert!(!text.contains("checked"), "{file}");
        assert!(!text.contains("inlineTextBox"), "{file}");

        // Struktur bleibt: Beschriftungen, Button, Formularname.
        let fields: Vec<(&str, &str)> = input
            .nodes()
            .iter()
            .filter(|n| n.redacted.is_some())
            .map(|n| (n.role.as_str(), n.name.as_deref().unwrap_or("")))
            .collect();
        assert_eq!(
            fields,
            [
                ("textbox", "Name"),
                ("textbox", "E-Mail"),
                ("textbox", "Nachricht"),
                ("checkbox", "Datenschutzhinweise gelesen"),
            ],
            "{file}"
        );
        assert!(input
            .nodes()
            .iter()
            .filter(|n| n.redacted.is_some())
            .all(|n| n.redacted == Some(Redaction::Value)));
        assert_eq!(by_name(&input, "Nachricht senden").redacted, None);
        assert_eq!(by_name(&input, "Kontaktformular").role, "form");

        // Jede ID ist lokal und führt in der Runtime zum Knoten zurück.
        for n in input.nodes() {
            let at = input.node_ref(&n.id).expect("ID ohne Knoten");
            assert!(graph.node(at).is_some(), "{file}: {}", n.id);
            assert!(n.id.starts_with("t0:"), "{file}: {}", n.id);
        }
    }
}

#[test]
fn iframe_wird_eingehaengt_ohne_tree_ids_preiszugeben() {
    for (file, graph) in recording("04-iframe/01-with-iframe") {
        assert_eq!(graph.trees.len(), 2, "{file}");
        let input = filter(&graph, &PrivacyContext::default());
        let text = json(&input);

        let trees: BTreeSet<&str> = input
            .nodes()
            .iter()
            .map(|n| n.id.split(':').next().unwrap())
            .collect();
        assert_eq!(trees, BTreeSet::from(["t0", "t1"]), "{file}");
        // Tree-IDs des CDP-Hosts (URL + Frame-Präfix) bleiben in der Runtime.
        for tree in graph
            .trees
            .keys()
            .filter(|t| Some(*t) != graph.root.as_ref())
        {
            assert!(!text.contains(&tree.0), "{file}: {tree}");
        }

        // Der Button im Frame hängt über die Frame-Wurzel am iframe-Knoten.
        let mut current = by_name(&input, "Alle akzeptieren");
        assert!(current.id.starts_with("t1:"));
        let mut roles = Vec::new();
        while let Some(parent) = current.parent.as_deref() {
            current = by_id(&input, parent);
            roles.push(current.role.as_str());
        }
        assert!(roles.contains(&"iframe"), "{file}: {roles:?}");
        assert_eq!(roles.last(), Some(&"rootWebArea"), "{file}");
        assert_eq!(
            input.page().url.as_deref(),
            Some("file:///REPO/spike/fixtures/with-iframe.html")
        );
    }
}

/// Login: Benutzername ohne Hinweis, Passwort mit `type=password`.
fn login() -> SemanticGraph {
    page(vec![
        (node(1, Role::RootWebArea, Some("Konto")), None),
        (node(2, Role::Form, Some("Anmeldung")), Some(1)),
        (field(3, "Benutzername"), Some(2)),
        (typed(4, "erika"), Some(3)),
        (field(5, "Passwort"), Some(2)),
        (typed(6, "geheim123"), Some(5)),
        (node(7, Role::Button, Some("Anmelden")), Some(2)),
        (node(8, Role::Link, Some("Zugang vergessen")), Some(1)),
    ])
}

fn password_hint() -> PrivacyContext {
    let mut context = PrivacyContext::default();
    context.fields.insert(
        at(5),
        FieldHint {
            input_type: Some("password".into()),
            autocomplete: Some("current-password".into()),
        },
    );
    context
}

#[test]
fn login_formular_ist_ganz_sensibel() {
    let input = filter(&login(), &password_hint());
    let text = json(&input);
    for secret in ["erika", "geheim123", "Benutzername", "Passwort"] {
        assert!(!text.contains(secret), "„{secret}“ im Modell-Input");
    }
    for id in ["t0:3", "t0:5"] {
        let f = by_id(&input, id);
        assert_eq!(f.redacted, Some(Redaction::Sensitive), "{id}");
        assert_eq!((f.name.as_deref(), f.states.len()), (None, 0), "{id}");
        assert_eq!(f.role, "textbox");
    }
    // Außerhalb der Felder bleibt die Struktur.
    assert_eq!(by_name(&input, "Anmelden").role, "button");
    assert_eq!(by_name(&input, "Zugang vergessen").role, "link");
}

#[test]
fn ohne_hinweis_bleiben_nur_die_werte_weg() {
    let input = filter(&login(), &PrivacyContext::default());
    let text = json(&input);
    for secret in ["erika", "geheim123"] {
        assert!(!text.contains(secret), "„{secret}“ im Modell-Input");
    }
    assert_eq!(by_name(&input, "Passwort").redacted, Some(Redaction::Value));
}

#[test]
fn zahlungsfeld_ohne_formular_und_sensible_seite() {
    let mut card = field(3, "Kartennummer");
    card.value = relief_model::Fact::known(Some("4111 1111 1111 1111".into()));
    let mut voucher = field(4, "Gutscheincode");
    voucher.value = relief_model::Fact::known(Some("RABATT10".into()));
    let graph = page(vec![
        (node(1, Role::RootWebArea, Some("Kasse")), None),
        (card, Some(1)),
        (voucher, Some(1)),
    ]);
    let mut context = PrivacyContext::default();
    context.fields.insert(
        at(3),
        FieldHint {
            input_type: None,
            autocomplete: Some("billing cc-number".into()),
        },
    );

    let input = filter(&graph, &context);
    let text = json(&input);
    assert!(!text.contains("4111") && !text.contains("RABATT10"));
    assert!(!text.contains("Kartennummer"));
    assert_eq!(by_id(&input, "t0:3").redacted, Some(Redaction::Sensitive));
    assert_eq!(
        by_name(&input, "Gutscheincode").redacted,
        Some(Redaction::Value)
    );

    // Seitentyp Checkout: jedes Feld sensibel.
    context.sensitive_page = true;
    let input = filter(&graph, &context);
    assert!(!json(&input).contains("Gutscheincode"));
    assert_eq!(by_id(&input, "t0:4").redacted, Some(Redaction::Sensitive));
}

#[test]
fn adressen_ohne_query_und_fragment() {
    let input = filter(&common::shop(), &PrivacyContext::default());
    let text = json(&input);
    assert!(!text.contains("sid=geheim"), "{text}");
    assert_eq!(
        input.page().url.as_deref(),
        Some("https://shop.example/produkt")
    );
    assert_eq!(
        by_name(&input, "Warenkorb").url.as_deref(),
        Some("https://shop.example/cart")
    );
    assert!(!text.contains("SOMMER"));
}
