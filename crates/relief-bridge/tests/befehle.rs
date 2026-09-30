//! Befehle über die Runtime des Forks: dieselben Eingaben wie in
//! `spike/tasks/01-shop-clean.txt`, auf der Aufnahme als Modell, bis zu den
//! AX-Schritten, die der Fork als `AXActionData` sendet.

// Nur die Stände vorher werden gebraucht.
#[allow(dead_code)]
mod common;

use relief_bridge::{Key, Reply, Runtime, Step};
use relief_model::{Action, Role, SemanticGraph, TreeDelta};

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

fn schritte(runtime: &mut Runtime, eingabe: &str) -> Vec<(Role, Action, Option<String>)> {
    match runtime.command(eingabe) {
        Reply::Perform(steps) => steps
            .into_iter()
            .map(|s| match s {
                Step::Ax(s) => {
                    let role = runtime.graph().node(&s.target).unwrap().role.clone();
                    (role, s.action, s.value)
                }
                Step::Key(key) => panic!("„{eingabe}“: Taste {key:?}"),
            })
            .collect(),
        other => panic!("„{eingabe}“: {other:?}"),
    }
}

#[test]
fn aktionen_werden_ax_schritte() {
    let mut rt = runtime_shop();
    assert_eq!(
        schritte(&mut rt, "Wähle 43"),
        vec![(Role::Option, Action::DoDefault, None)]
    );
    assert_eq!(
        schritte(&mut rt, "fülle Suche mit Laufschuhe"),
        vec![
            (Role::SearchBox, Action::Focus, None),
            (Role::SearchBox, Action::SetValue, Some("Laufschuhe".into())),
        ]
    );
    assert_eq!(
        schritte(&mut rt, "Gehe zur Überschrift Technische Daten"),
        vec![
            (Role::Heading, Action::ScrollToMakeVisible, None),
            (
                Role::Heading,
                Action::SetSequentialFocusNavigationStartingPoint,
                None
            ),
        ]
    );
    // „!“ löst nur die eben gestellte Rückfrage ein.
    assert!(matches!(
        rt.command("klicke Jetzt kaufen"),
        Reply::Answer(_)
    ));
    assert_eq!(
        schritte(&mut rt, "!klicke Jetzt kaufen"),
        vec![(Role::Button, Action::DoDefault, None)]
    );
}

#[test]
fn bestaetigung_ohne_rueckfrage_und_zweimal_ergibt_keine_schritte() {
    let mut rt = runtime_shop();
    let Reply::Answer(text) = rt.command("!klicke Jetzt kaufen") else {
        panic!("„!“ ohne Rückfrage darf nichts auslösen");
    };
    assert!(text.contains("keine offene Rückfrage"), "{text}");
    assert!(matches!(
        rt.command("klicke Jetzt kaufen"),
        Reply::Answer(_)
    ));
    assert!(matches!(rt.command("Was ist hier?"), Reply::Answer(_)));
    assert!(matches!(
        rt.command("!klicke Jetzt kaufen"),
        Reply::Answer(_)
    ));
    assert!(matches!(
        rt.command("!klicke Jetzt kaufen"),
        Reply::Perform(_)
    ));
    assert!(matches!(
        rt.command("!klicke Jetzt kaufen"),
        Reply::Answer(_)
    ));

    // Das Security-Log trägt Entscheidung, Plan-ID und Grund, keine Namen.
    let log = serde_json::to_string(&rt.take_security_log()).unwrap();
    assert_eq!(
        log,
        r#"[{"decision":"reject","action":"activate","risk":"High","reason":"no_prompt"},{"decision":"ask_confirmation","plan":1,"action":"activate","risk":"High"},{"decision":"ask_confirmation","plan":2,"action":"activate","risk":"High"},{"decision":"reject","action":"activate","risk":"High","reason":"no_prompt"},{"decision":"ask_confirmation","plan":3,"action":"activate","risk":"High"},{"decision":"perform_confirmed","plan":3,"action":"activate","risk":"High"},{"decision":"reject","action":"activate","risk":"High","reason":"no_prompt"},{"decision":"ask_confirmation","plan":4,"action":"activate","risk":"High"}]"#
    );
    assert!(!log.contains("kaufen"));
    assert!(rt.take_security_log().is_empty());
}

#[test]
fn erhoehen_ist_fokus_und_pfeiltaste() {
    let mut rt = runtime_shop();
    // Kein Schieberegler auf der Seite: abgelehnt, bevor Schritte entstehen.
    assert!(matches!(rt.command("erhöhe Größe"), Reply::Answer(_)));
    let steps = match relief_bridge::ax_steps(
        rt.graph(),
        &relief_interaction::ActionPlan {
            target: relief_model::NodeRef::new(
                rt.graph().root.clone().unwrap(),
                relief_model::NodeId(1),
            ),
            dom_node_id: 1,
            kind: relief_interaction::ActionKind::Increment,
            risk: relief_interaction::Risk::Medium,
            requires_confirmation: false,
            notes: Vec::new(),
        },
    ) {
        Ok(steps) => steps,
        Err(e) => panic!("{e}"),
    };
    assert!(matches!(
        steps.as_slice(),
        [Step::Ax(_), Step::Key(Key::ArrowUp)]
    ));
}

#[test]
fn antworten_ohne_aktion_und_rueckfrage() {
    let mut rt = runtime_shop();
    let Reply::Answer(text) = rt.command("klicke Jetzt kaufen") else {
        panic!("Rückfrage erwartet");
    };
    assert!(text.starts_with("Bestätigung nötig"), "{text}");
    assert_eq!(rt.finish(), "Keine Aktion ausstehend.");
    assert!(rt.describe_page().contains("Nike Air Max"));
    assert_eq!(
        rt.command("scrolle nach unten"),
        Reply::Scroll(relief_interaction::ScrollDirection::Down)
    );
}

#[test]
fn nach_der_aktion_antwortet_die_runtime_mit_der_wirkung() {
    let mut rt = runtime_shop();
    assert!(matches!(
        rt.command("Gehe zur Überschrift Technische Daten"),
        Reply::Perform(_)
    ));
    // Ohne neue Delta: keine Änderung, aber die Position steht auf der Überschrift.
    let text = rt.finish();
    assert!(
        text.contains("Fokus jetzt auf heading „Technische Daten“"),
        "{text}"
    );
    let Reply::Answer(wo) = rt.command("Wo bin ich?") else {
        panic!()
    };
    assert!(
        wo.starts_with("Fokus auf Überschrift H2 „Technische Daten“"),
        "{wo}"
    );
}

#[test]
fn mehrdeutig_nummeriert_und_per_zahl_oder_name_waehlbar() {
    let mut rt = runtime_shop();
    let Reply::Answer(text) = rt.command("Öffne den Warenkorb") else {
        panic!("Rückfrage erwartet")
    };
    assert!(text.contains("  1. [link] Warenkorb (0)"), "{text}");
    assert!(text.contains("  2. [button] In den Warenkorb"), "{text}");
    assert_eq!(
        schritte(&mut rt, "2"),
        vec![(Role::Button, Action::DoDefault, None)]
    );

    assert!(matches!(
        rt.command("Öffne den Warenkorb"),
        Reply::Answer(_)
    ));
    assert_eq!(
        schritte(&mut rt, "link"),
        vec![(Role::Link, Action::DoDefault, None)]
    );

    // Die Auswahl gilt nur für die nächste Eingabe.
    assert!(matches!(
        rt.command("Öffne den Warenkorb"),
        Reply::Answer(_)
    ));
    assert!(matches!(rt.command("Was ist hier?"), Reply::Answer(_)));
    assert!(matches!(rt.command("2"), Reply::Answer(t) if t.starts_with("Nicht verstanden")));
}

#[test]
fn ja_bestaetigt_einmal_abbrechen_verwirft() {
    let mut rt = runtime_shop();
    assert!(
        matches!(rt.command("klicke Jetzt kaufen"), Reply::Answer(t) if t.starts_with("Bestätigung nötig"))
    );
    assert_eq!(
        schritte(&mut rt, "ja"),
        vec![(Role::Button, Action::DoDefault, None)]
    );
    // Ein zweites „ja“ löst nichts mehr aus.
    assert!(matches!(rt.command("ja"), Reply::Answer(t) if t.starts_with("Nicht verstanden")));

    assert!(matches!(
        rt.command("klicke Jetzt kaufen"),
        Reply::Answer(_)
    ));
    assert_eq!(
        rt.command("abbrechen"),
        Reply::Answer("Abgebrochen. Nichts ausgeführt.".into())
    );
    assert!(matches!(rt.command("ja"), Reply::Answer(t) if t.starts_with("Nicht verstanden")));
    assert!(
        matches!(rt.command("!klicke Jetzt kaufen"), Reply::Answer(t) if t.starts_with("Bestätigung nötig"))
    );
    assert_eq!(
        rt.command("abbrechen"),
        Reply::Answer("Abgebrochen. Nichts ausgeführt.".into())
    );
    assert_eq!(
        rt.command("abbrechen"),
        Reply::Answer("Nichts offen, das sich abbrechen ließe.".into())
    );
}

/// Wie `runtime_shop`, jeder Knoten mit einer Position (die CDP-Aufnahmen
/// tragen keine), in Dokumentreihenfolge untereinander.
fn runtime_shop_mit_positionen() -> Runtime {
    let pair = common::pairs()
        .into_iter()
        .find(|p| p.name.contains("01-shop-clean"))
        .expect("Aufnahme 01-shop-clean");
    let mut model = pair.before;
    let order = model.document_order();
    for (i, at) in order.iter().enumerate() {
        let tree = model.trees.get_mut(&at.tree).unwrap();
        tree.nodes.get_mut(&at.node).unwrap().bounds = Some(relief_model::Rect {
            x: 10.0,
            y: 20.0 * i as f32,
            width: 80.0,
            height: 16.0,
        });
    }
    let mut runtime = Runtime::new();
    runtime
        .apply(&TreeDelta::between(&SemanticGraph::default(), &model))
        .unwrap();
    runtime
}

#[test]
fn sprungmarken_listen_und_waehlen_mit_rueckfrage() {
    let mut rt = runtime_shop_mit_positionen();
    let Reply::Answer(liste) = rt.command("sprungmarken") else {
        panic!("Liste erwartet")
    };
    assert!(liste.starts_with("12 Sprungmarken:"), "{liste}");
    let zeile = liste
        .lines()
        .find(|l| l.contains("[button] Jetzt kaufen"))
        .unwrap();
    let marke = zeile.trim().split(':').next().unwrap().to_string();
    assert_eq!(marke.len(), 2);

    assert!(
        matches!(rt.command(&format!("marke {marke}")), Reply::Answer(t) if t.starts_with("Bestätigung nötig"))
    );
    assert_eq!(
        schritte(&mut rt, "ja"),
        vec![(Role::Button, Action::DoDefault, None)]
    );
    // Felder werden fokussiert, nicht ausgelöst.
    let suche = liste
        .lines()
        .find(|l| l.contains("[searchbox] Suche"))
        .unwrap()
        .trim()
        .split(':')
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        schritte(&mut rt, &format!("marke {suche}")),
        vec![(Role::SearchBox, Action::Focus, None)]
    );
    assert!(
        matches!(rt.command("marke zz"), Reply::Answer(t) if t.starts_with("Keine Sprungmarke"))
    );
}

/// Paket 76: Ein Passwortfeld ausfüllen. Der Wert geht als AX-Schritt an den
/// Fork, steht aber weder in der Antwort noch in der Eingabe fürs Protokoll.
#[test]
fn passwort_steht_nicht_in_antwort_und_protokoll() {
    let pair = common::pairs()
        .into_iter()
        .find(|p| p.name.contains("01-shop-clean"))
        .expect("Aufnahme 01-shop-clean");
    // Das Suchfeld als Passwortfeld, wie der Fork `type` meldet.
    let mut vorher = pair.before;
    let suche = vorher
        .document_order()
        .into_iter()
        .find(|at| vorher.node(at).unwrap().role == Role::SearchBox)
        .unwrap();
    let mut feld = vorher.node(&suche).unwrap().clone();
    feld.extra.insert(
        relief_interaction::security::INPUT_TYPE.into(),
        "password".into(),
    );
    let setze = |g: &mut SemanticGraph, n: relief_model::SemanticNode| {
        g.trees
            .get_mut(&suche.tree)
            .unwrap()
            .nodes
            .insert(suche.node, n);
    };
    setze(&mut vorher, feld.clone());
    let mut rt = Runtime::new();
    rt.apply(&TreeDelta::between(&SemanticGraph::default(), &vorher))
        .unwrap();

    let eingabe = "fülle Suche mit geheim123";
    assert_eq!(
        schritte(&mut rt, eingabe),
        vec![
            (Role::SearchBox, Action::Focus, None),
            (Role::SearchBox, Action::SetValue, Some("geheim123".into())),
        ]
    );
    let mut nachher = rt.graph().clone();
    feld.value = relief_model::Fact::known(Some("geheim123".into()));
    setze(&mut nachher, feld);
    rt.apply(&TreeDelta::between(rt.graph(), &nachher)).unwrap();
    let antwort = rt.finish();
    assert!(antwort.starts_with("SetValue(verdeckt) auf"), "{antwort}");
    assert!(antwort.contains("Wert geändert"), "{antwort}");
    assert!(!antwort.contains("geheim123"), "{antwort}");
    assert_eq!(
        relief_interaction::redact_input(eingabe),
        "fülle Suche mit (verdeckt)"
    );
    let security = serde_json::to_string(&rt.take_security_log()).unwrap();
    assert!(!security.contains("geheim123"), "{security}");
}

/// Paket 75: Der Fork trägt das Formularziel erst bei einer Rückfrage nach
/// (Renderer-Anfrage); die neu gestellte Rückfrage nennt es, ein anderes
/// Ziel vor „ja“ verlangt eine neue Bestätigung.
#[test]
fn formularziel_nachgetragen_und_gebunden() {
    let mut rt = runtime_shop();
    assert!(rt.confirmation_target().is_none());
    assert!(
        matches!(rt.command("klicke Jetzt kaufen"), Reply::Answer(t) if t.starts_with("Bestätigung nötig"))
    );
    let ziel = rt.confirmation_target().expect("Rückfrage offen");
    rt.apply_form_facts(&ziel, Some("https://shop.test/kasse".into()), &[]);
    let frage = match rt.reconfirm() {
        Reply::Answer(t) => t,
        other => panic!("{other:?}"),
    };
    assert!(frage.starts_with("Bestätigung nötig"), "{frage}");
    assert!(frage.contains("https://shop.test/kasse"), "{frage}");
    assert_eq!(rt.confirmation_target(), Some(ziel.clone()));

    // Vor „ja“ meldet der Renderer ein anderes Ziel: keine Schritte.
    rt.apply_form_facts(&ziel, Some("https://fremd.test/".into()), &[]);
    match rt.command("ja") {
        Reply::Answer(t) => {
            assert!(t.contains("https://fremd.test/"), "{t}");
            assert!(t.contains("Bestätigung nötig"), "{t}");
        }
        other => panic!("{other:?}"),
    }
    // Die neue Rückfrage gilt mit demselben Ziel.
    assert_eq!(
        schritte(&mut rt, "ja"),
        vec![(Role::Button, Action::DoDefault, None)]
    );
}

/// Neu gestellt nennt die Rückfrage weiter, warum „!“ nicht galt.
#[test]
fn neu_gestellte_rueckfrage_behaelt_den_grund() {
    let mut rt = runtime_shop();
    let erste = match rt.command("!klicke Jetzt kaufen") {
        Reply::Answer(t) => t,
        other => panic!("{other:?}"),
    };
    assert!(erste.contains("keine offene Rückfrage"), "{erste}");
    match rt.reconfirm() {
        Reply::Answer(t) => assert!(t.contains("keine offene Rückfrage"), "{t}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        rt.command("abbrechen"),
        Reply::Answer("Abgebrochen. Nichts ausgeführt.".into())
    );
    match rt.command("klicke Jetzt kaufen") {
        Reply::Answer(t) => assert!(!t.contains("keine offene Rückfrage"), "{t}"),
        other => panic!("{other:?}"),
    }
}
