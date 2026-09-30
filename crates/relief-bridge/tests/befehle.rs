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
