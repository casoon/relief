//! Formular-Assistent (Paket 39): Felder eines Formulars mit Pflicht,
//! Fehlerzustand und verknüpfter Fehlermeldung, für „was fehlt noch“,
//! „Fehler vorlesen“, den Sprung zum ersten Fehler und die Zusammenfassung
//! vor dem Absenden.
//!
//! Grundlage ist die Gruppe [`GroupKind::Form`] aus `page` (Felder eines
//! `<form>`). Pflicht und Fehler kommen aus dem AXTree (`required`,
//! `invalid`); die Meldung aus `aria-errormessage`, sonst aus
//! `aria-describedby`, solange das Feld als fehlerhaft gilt. Werte
//! sensibler Felder (Passwort, Zahlungs- und Identitätsdaten, Anmelde- und
//! Kassenseite) erscheinen nie im Klartext. Relief speichert keine Werte;
//! alles hier liest nur den aktuellen Stand.

use relief_model::{NodeId, NodeRef, Role, SemanticGraph, Toggle};

use crate::graph::{Control, Graph};
use crate::page::{Group, GroupKind};
use crate::respond;
use crate::security::{is_sensitive_field, HTML_AUTOCOMPLETE, INPUT_TYPE};

/// Ein Feld eines Formulars.
#[derive(Debug, Clone)]
pub struct Field {
    /// Index in [`Graph::controls`].
    pub control: usize,
    pub label: String,
    pub required: bool,
    pub invalid: bool,
    /// Wert zur Anzeige: Text, „angehakt“/„nicht angehakt“, gewählte Option
    /// oder „leer“; sensible Werte als „(verdeckt)“.
    pub shown: String,
    /// Leer im Sinne von „Pflicht nicht erfüllt“.
    pub empty: bool,
    /// Verknüpfte Fehlermeldung, nur bei `invalid`.
    pub error: Option<String>,
}

/// Rollen, die ein Formularfeld ausmachen (kein Button).
fn is_field(role: &Role) -> bool {
    matches!(
        role,
        Role::Textbox
            | Role::SearchBox
            | Role::Combobox
            | Role::Listbox
            | Role::SpinButton
            | Role::Slider
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
    )
}

/// Das Formular, in dem ein Bedienelement liegt.
pub fn form_of(graph: &Graph, control: usize) -> Option<&Group> {
    graph
        .page
        .groups
        .iter()
        .find(|g| g.kind == GroupKind::Form && g.controls.contains(&control))
}

/// Das gemeinte Formular: das am Fokus bzw. an der Position, sonst das
/// einzige der Seite. Fehler: Antworttext.
pub fn current_form<'g>(graph: &'g Graph, here: Option<&NodeRef>) -> Result<&'g Group, String> {
    let forms: Vec<&Group> = graph
        .page
        .groups
        .iter()
        .filter(|g| g.kind == GroupKind::Form)
        .collect();
    if let Some(at) = here {
        if let Some(i) = graph.controls.iter().position(|c| c.node == *at) {
            if let Some(g) = form_of(graph, i) {
                return Ok(g);
            }
        }
    }
    match forms.as_slice() {
        [one] => Ok(one),
        [] => Err("Kein Formular auf der Seite.".into()),
        many => Err(format!(
            "{} Formulare auf der Seite: {}. Erst in eines gehen („nächstes Formularfeld“ oder „gehe zu …“).",
            many.len(),
            many.iter()
                .map(|g| label(g))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn label(g: &Group) -> String {
    match &g.name {
        Some(n) => format!("„{n}“"),
        None => "ohne Namen".into(),
    }
}

/// Text eines Knotens: Name, sonst der Text seiner Nachfahren.
fn text_of(model: &SemanticGraph, at: &NodeRef) -> String {
    let Some(node) = model.node(at) else {
        return String::new();
    };
    if let Some(name) = node.name.value.as_deref().filter(|n| !n.trim().is_empty()) {
        return name.trim().to_string();
    }
    let mut parts = Vec::new();
    let mut stack: Vec<NodeId> = node.children.iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        let child_at = NodeRef::new(at.tree.clone(), id);
        let Some(child) = model.node(&child_at) else {
            continue;
        };
        if child.role == Role::StaticText {
            if let Some(t) = child.name.value.as_deref().filter(|t| !t.trim().is_empty()) {
                parts.push(t.trim().to_string());
            }
            continue;
        }
        stack.extend(child.children.iter().rev().copied());
    }
    parts.join(" ")
}

/// Sensibel: Feldangaben des Hosts oder Anmelde-/Kassenseite (→ 07).
fn sensitive(graph: &Graph, model: &SemanticGraph, c: &Control) -> bool {
    let node = model.node(&c.node);
    let extra = |key: &str| node.and_then(|n| n.extra.get(key)).map(String::as_str);
    is_sensitive_field(extra(INPUT_TYPE), extra(HTML_AUTOCOMPLETE))
        || graph.page.kind.value.is_some_and(|t| t.is_sensitive())
}

/// Felder eines Formulars in Dokumentreihenfolge.
pub fn fields(graph: &Graph, model: &SemanticGraph, group: &Group) -> Vec<Field> {
    group
        .controls
        .iter()
        .filter(|&&i| is_field(&graph.controls[i].role))
        .map(|&i| {
            let c = &graph.controls[i];
            let node = model.node(&c.node);
            let required = node.is_some_and(|n| n.states.required);
            let invalid = node.is_some_and(|n| n.states.invalid.is_some());
            let checked = node.and_then(|n| n.states.checked);
            let (shown, empty) = match c.role {
                Role::Checkbox | Role::Radio | Role::Switch => match checked {
                    Some(Toggle::True) => ("angehakt".to_string(), false),
                    Some(Toggle::Mixed) => ("teilweise".to_string(), false),
                    _ => ("nicht angehakt".to_string(), true),
                },
                _ => match c.selected_option.as_ref().or(c.value.as_ref()) {
                    Some(v) if !v.trim().is_empty() => {
                        if sensitive(graph, model, c) {
                            ("(verdeckt)".to_string(), false)
                        } else {
                            (format!("„{}“", v.trim()), false)
                        }
                    }
                    _ => ("leer".to_string(), true),
                },
            };
            let error = invalid
                .then(|| {
                    let relations = &node?.relations;
                    let ids = if relations.error_message.is_empty() {
                        &relations.described_by
                    } else {
                        &relations.error_message
                    };
                    let text = ids
                        .iter()
                        .map(|id| text_of(model, &NodeRef::new(c.node.tree.clone(), *id)))
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    (!text.is_empty()).then_some(text)
                })
                .flatten();
            Field {
                control: i,
                label: c.display_name(),
                required,
                invalid,
                shown,
                empty,
                error,
            }
        })
        .collect()
}

/// „Was fehlt noch?“: leere Pflichtfelder und als fehlerhaft gemeldete.
pub fn missing(graph: &Graph, model: &SemanticGraph, group: &Group) -> String {
    let fields = fields(graph, model, group);
    let open: Vec<String> = fields
        .iter()
        .filter(|f| (f.required && f.empty) || f.invalid)
        .map(|f| {
            let mut s = format!(
                "{} ({})",
                f.label,
                if f.invalid {
                    "fehlerhaft"
                } else {
                    "Pflicht, leer"
                }
            );
            if let Some(e) = &f.error {
                s.push_str(&format!(": „{e}“"));
            }
            s
        })
        .collect();
    if open.is_empty() {
        format!(
            "Formular {}: alle Pflichtfelder ausgefüllt, keine Fehler gemeldet ({} Felder, {} Pflicht).",
            label(group),
            fields.len(),
            fields.iter().filter(|f| f.required).count()
        )
    } else {
        format!(
            "Formular {}: noch offen: {}.",
            label(group),
            open.join("; ")
        )
    }
}

/// „Fehler vorlesen“: als fehlerhaft gemeldete Felder mit Meldung.
pub fn errors(graph: &Graph, model: &SemanticGraph, group: &Group) -> String {
    let wrong: Vec<String> = fields(graph, model, group)
        .into_iter()
        .filter(|f| f.invalid)
        .map(|f| match f.error {
            Some(e) => format!("{}: „{e}“", f.label),
            None => format!("{}: fehlerhaft, keine Meldung verknüpft", f.label),
        })
        .collect();
    if wrong.is_empty() {
        format!(
            "Formular {}: kein Feld als fehlerhaft gemeldet.",
            label(group)
        )
    } else {
        format!(
            "Formular {}: {} Fehler: {}. „zum ersten Fehler“ führt hin.",
            label(group),
            wrong.len(),
            wrong.join("; ")
        )
    }
}

/// Erstes fehlerhaftes Feld.
pub fn first_error(graph: &Graph, model: &SemanticGraph, group: &Group) -> Option<usize> {
    fields(graph, model, group)
        .into_iter()
        .find(|f| f.invalid)
        .map(|f| f.control)
}

/// Werte aller Felder, für die Rückfrage vor dem Absenden (sensible
/// verdeckt).
pub fn summary(graph: &Graph, model: &SemanticGraph, group: &Group) -> String {
    let fields = fields(graph, model, group);
    let list = fields
        .iter()
        .map(|f| {
            format!(
                "{} = {}{}",
                f.label,
                f.shown,
                if f.required && f.empty {
                    " (Pflicht)"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("Formular {}: {list}", label(group))
}

/// Werte der Felder für die Bindung einer Bestätigung (im Speicher, nie im
/// Log): (Knoten, Wert, angehakt).
pub fn values(
    graph: &Graph,
    model: &SemanticGraph,
    group: &Group,
) -> Vec<(NodeRef, Option<String>, Option<Toggle>)> {
    group
        .controls
        .iter()
        .filter(|&&i| is_field(&graph.controls[i].role))
        .map(|&i| {
            let c = &graph.controls[i];
            (
                c.node.clone(),
                c.selected_option.clone().or_else(|| c.value.clone()),
                model.node(&c.node).and_then(|n| n.states.checked),
            )
        })
        .collect()
}

/// Nach einer Aktion: fehlerhafte Felder im Formular des Ziels, als Hinweis
/// an die Antwort.
pub fn errors_after(graph: &Graph, model: &SemanticGraph, target: &NodeRef) -> Option<String> {
    let i = graph.controls.iter().position(|c| c.node == *target)?;
    let group = form_of(graph, i)?;
    let wrong: Vec<Field> = fields(graph, model, group)
        .into_iter()
        .filter(|f| f.invalid)
        .collect();
    if wrong.is_empty() {
        return None;
    }
    Some(format!(
        "{} {} im Formular: {}. „zum ersten Fehler“ führt hin, „fehler vorlesen“ liest sie.",
        wrong.len(),
        if wrong.len() == 1 {
            "Feld fehlerhaft"
        } else {
            "Felder fehlerhaft"
        },
        wrong
            .iter()
            .map(|f| match &f.error {
                Some(e) => format!("{} („{e}“)", f.label),
                None => f.label.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Kurzform eines Bedienelements für Antworten.
pub fn line(graph: &Graph, i: usize) -> String {
    respond::control_line(&graph.controls[i])
}
