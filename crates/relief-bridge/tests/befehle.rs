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
    assert_eq!(
        schritte(&mut rt, "!klicke Jetzt kaufen"),
        vec![(Role::Button, Action::DoDefault, None)]
    );
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
