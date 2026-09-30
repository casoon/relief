//! Befehle in Sprache im Fork: Eingabe → [`relief_interaction::Session`] →
//! geprüfter Plan → Schritte über `AXActionData` ([`AxStep`]).
//!
//! Der Fork führt die Schritte aus, wartet auf Ruhe (keine AX-Pakete mehr)
//! und holt die Antwort mit [`Runtime::finish`]; bis dahin merkt sich die
//! Runtime den Stand vor der Aktion ([`Pending`]).

use relief_interaction::security::{FORM_ACTION, HTML_AUTOCOMPLETE};
use relief_interaction::{
    parse_input, ActionKind, ActionPlan as InteractionPlan, Graph, Outcome, ScrollDirection,
};
use relief_model::{Action, NodeId, NodeRef, Role, SemanticGraph};

use crate::runtime::Runtime;

/// Eine Aktion über `AXActionData` an einem Knoten.
#[derive(Debug, Clone, PartialEq)]
pub struct AxStep {
    pub target: NodeRef,
    pub action: Action,
    /// Nur bei [`Action::SetValue`].
    pub value: Option<String>,
}

/// Taste als echtes Tastaturereignis an das fokussierte Element: der
/// Ersatzweg, wo `AXActionData` nicht reicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Escape,
    ArrowUp,
    ArrowDown,
}

/// Ein Schritt einer Aktion.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Ax(AxStep),
    Key(Key),
}

/// Was der Fork nach einer Eingabe tun soll.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Answer(String),
    /// Schritte der Reihe nach senden, Ruhe abwarten, [`Runtime::finish`].
    Perform(Vec<Step>),
    /// [`Key::Escape`] an den Fokus (kein AX-Weg), Ruhe, [`Runtime::finish`].
    Escape,
    /// Dokument scrollen; Antwort über `respond::scrolled`.
    Scroll(ScrollDirection),
}

/// Stand vor einer Aktion, bis ihre Antwort feststeht.
#[derive(Debug, Clone)]
pub enum Pending {
    Plan {
        plan: InteractionPlan,
        label: String,
        before: SemanticGraph,
        before_graph: Box<Graph>,
    },
    Escape {
        target: String,
        reaches: Option<String>,
        before: SemanticGraph,
    },
}

impl Runtime {
    /// Eine Eingabe gegen den aktuellen Graphen. Der Fokus kommt aus dem
    /// Modell: Im Fork führt Chromium ihn in den Baumdaten nach.
    pub fn command(&mut self, input: &str) -> Reply {
        self.pending = None;
        let graph = Graph::build(&self.graph);
        // Antwort auf eine offene Rückfrage (Zahl, „ja“, „abbrechen“)?
        let input = match self.session.pending_reply(&graph, &self.graph, input) {
            relief_interaction::Pending::Done(outcome) => return self.reply(graph, outcome),
            relief_interaction::Pending::Command => input.to_string(),
        };
        let (confirmed, cmd) = match parse_input(&input) {
            Ok(c) => c,
            Err(msg) => return Reply::Answer(msg),
        };
        let focus = relief_interaction::focused(&self.graph);
        let outcome = self
            .session
            .handle(&graph, &self.graph, confirmed, cmd, focus.as_ref());
        self.reply(graph, outcome)
    }

    /// Ergebnis der Sitzung in einen Auftrag an den Fork übersetzen.
    fn reply(&mut self, graph: Graph, outcome: Outcome) -> Reply {
        match outcome {
            Outcome::Answer(text) => Reply::Answer(text),
            Outcome::Scroll(direction) => Reply::Scroll(direction),
            Outcome::Escape { target, reaches } => {
                self.pending = Some(Pending::Escape {
                    target,
                    reaches,
                    before: self.graph.clone(),
                });
                Reply::Escape
            }
            Outcome::Perform { plan, label } => match ax_steps(&self.graph, &plan) {
                Ok(steps) => {
                    self.pending = Some(Pending::Plan {
                        plan,
                        label,
                        before: self.graph.clone(),
                        before_graph: Box::new(graph),
                    });
                    Reply::Perform(steps)
                }
                Err(msg) => Reply::Answer(format!("Aktion fehlgeschlagen: {msg} ({label})")),
            },
        }
    }

    /// Antwort auf die zuletzt ausgeführte Aktion, gegen den jetzigen Stand.
    pub fn finish(&mut self) -> String {
        match self.pending.take() {
            None => "Keine Aktion ausstehend.".into(),
            Some(Pending::Plan {
                plan,
                label,
                before,
                before_graph,
            }) => {
                let after_graph = Graph::build(&self.graph);
                self.session.performed(
                    &plan,
                    &label,
                    (&before, &before_graph),
                    (&self.graph, &after_graph),
                )
            }
            Some(Pending::Escape {
                target,
                reaches,
                before,
            }) => self
                .session
                .escaped(&target, reaches.as_deref(), &before, &self.graph),
        }
    }

    /// Ziel der offenen Rückfrage (→ [`Runtime::apply_form_facts`]).
    pub fn confirmation_target(&self) -> Option<NodeRef> {
        self.session.confirmation_target().cloned()
    }

    /// Angaben des Renderers zum Formular eines Ziels ins Modell (Paket 75):
    /// Formularziel am Ziel, HTML-`autocomplete` an den Feldern (Knoten im
    /// Baum des Ziels). Unter denselben Schlüsseln wie im CDP-Host; ein
    /// fehlendes Formularziel entfernt ein früheres. Gilt bis zur nächsten
    /// Delta des Knotens; der Host fragt vor jeder Eingabe neu an, solange
    /// eine Rückfrage offen ist.
    pub fn apply_form_facts(
        &mut self,
        target: &NodeRef,
        action: Option<String>,
        fields: &[(NodeId, String)],
    ) {
        let Some(tree) = self.graph.trees.get_mut(&target.tree) else {
            return;
        };
        if let Some(node) = tree.nodes.get_mut(&target.node) {
            match action {
                Some(a) => node.extra.insert(FORM_ACTION.into(), a),
                None => node.extra.remove(FORM_ACTION),
            };
        }
        for (id, autocomplete) in fields {
            if let Some(node) = tree.nodes.get_mut(id) {
                node.extra
                    .insert(HTML_AUTOCOMPLETE.into(), autocomplete.clone());
            }
        }
    }

    /// Offene Rückfrage mit dem jetzigen Modell neu stellen
    /// ([`relief_interaction::Session::reconfirm`]); ohne offene Rückfrage
    /// eine Antwort, die das sagt.
    pub fn reconfirm(&mut self) -> Reply {
        self.pending = None;
        let graph = Graph::build(&self.graph);
        match self.session.reconfirm(&graph, &self.graph) {
            Some(outcome) => self.reply(graph, outcome),
            None => Reply::Answer("Keine Rückfrage offen.".into()),
        }
    }

    /// Security-Log der Sitzung seit dem letzten Abholen (Entscheidung,
    /// Plan-ID, Aktionsart, Risiko, Grund; keine Werte, keine Namen). Der
    /// Fork schreibt es nach jeder Eingabe ins Protokoll (`--relief-log`).
    pub fn take_security_log(&mut self) -> Vec<relief_interaction::SecurityEvent> {
        self.session.take_security_log()
    }

    /// Sprungmarken des aktuellen Stands; die Sitzung merkt sie für
    /// „marke …“.
    pub fn show_marks(&mut self) -> Vec<relief_interaction::Mark> {
        let graph = Graph::build(&self.graph);
        self.session.show_marks(&graph, &self.graph).to_vec()
    }

    /// Seitenbeschreibung wie nach dem Laden im CDP-Host.
    pub fn describe_page(&self) -> String {
        relief_interaction::respond::describe(&Graph::build(&self.graph))
    }
}

/// Ein geprüfter Plan als Schritte (→ `plan/spezifikation/05`, Tabelle
/// „Aktion → AX-Weg“).
///
/// Fehler: Der Plan lässt sich im Modell nicht abbilden (Option fehlt).
pub fn ax_steps(model: &SemanticGraph, plan: &InteractionPlan) -> Result<Vec<Step>, String> {
    let ax = |action| AxStep {
        target: plan.target.clone(),
        action,
        value: None,
    };
    let at = |action| Step::Ax(ax(action));
    Ok(match &plan.kind {
        ActionKind::Focus => vec![at(Action::Focus)],
        ActionKind::Activate => vec![at(Action::DoDefault)],
        // Erst fokussieren wie eine Person, die ins Feld geht.
        ActionKind::SetValue(v) => vec![
            at(Action::Focus),
            Step::Ax(AxStep {
                value: Some(v.clone()),
                ..ax(Action::SetValue)
            }),
        ],
        // Die Option selbst auslösen: Blink wählt sie über
        // `HTMLOptionElement::AccessKeyAction` aus und feuert `input`/`change`.
        ActionKind::Select(label) => vec![Step::Ax(AxStep {
            target: option(model, &plan.target, label)
                .ok_or_else(|| format!("Option „{label}“ nicht im Baum"))?,
            action: Action::DoDefault,
            value: None,
        })],
        // Überschriften und Bereiche sind nicht fokussierbar: sichtbar machen
        // und die Tab-Reihenfolge dort beginnen lassen (→ `session`,
        // Position).
        ActionKind::NavigateTo => vec![
            at(Action::ScrollToMakeVisible),
            at(Action::SetSequentialFocusNavigationStartingPoint),
        ],
        // Ersatzweg: fokussieren, dann echte Pfeiltaste wie eine Person an
        // der Tastatur. `Increment` über AX erreicht ARIA-Widgets nur mit
        // dem experimentellen Blink-Feature
        // `SynthesizedKeyboardEventsForAccessibilityActions`, und mit ihm
        // schickt Blink einem Zahlenfeld (ohne Ausrichtung) „Pfeil rechts“,
        // das es übergeht (→ `plan/spezifikation/05`).
        ActionKind::Increment => vec![at(Action::Focus), Step::Key(Key::ArrowUp)],
        ActionKind::Decrement => vec![at(Action::Focus), Step::Key(Key::ArrowDown)],
    })
}

/// Optionsknoten mit genau diesem Namen unter dem Auswahlfeld (auch in
/// seinem Popup, `menuListPopup`).
fn option(model: &SemanticGraph, field: &NodeRef, label: &str) -> Option<NodeRef> {
    let tree = model.trees.get(&field.tree)?;
    let mut stack = vec![field.node];
    while let Some(id) = stack.pop() {
        let node = tree.nodes.get(&id)?;
        if matches!(node.role, Role::Option | Role::MenuListOption)
            && node.name.value.as_deref().map(str::trim) == Some(label)
        {
            return Some(NodeRef::new(field.tree.clone(), id));
        }
        stack.extend(node.children.iter().rev());
    }
    None
}
