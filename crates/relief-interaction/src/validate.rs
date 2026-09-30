//! Aktion planen und validieren, bevor der Host sie ausführt
//! (→ `plan/spezifikation/05`).

use crate::graph::Control;
use crate::page::Page;
use relief_model::{Certainty, NodeRef, Role};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ActionKind {
    Focus,
    Activate,
    SetValue(String),
    /// Exakter Optionstext nach Validierung.
    Select(String),
    /// Hinbewegen und sichtbar machen, auch auf nicht fokussierbare Ziele
    /// (Überschrift, Bereich): Focus + ScrollToMakeVisible.
    NavigateTo,
    /// Schieberegler/Zahlenfeld um einen Schritt.
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Risk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionPlan {
    /// Ziel im Modell.
    pub target: NodeRef,
    /// DOM-Knoten des Ziels, über den der CDP-Host handelt.
    pub dom_node_id: i64,
    pub kind: ActionKind,
    pub risk: Risk,
    pub requires_confirmation: bool,
    /// Warum bestätigt werden muss bzw. was zu beachten ist.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Rejection {
    NoDomNode,
    Disabled,
    Unsupported {
        role: String,
        action: &'static str,
    },
    UnknownOption {
        available: Vec<String>,
    },
    /// Wert steht schon an der Grenze (`valuemin`/`valuemax`).
    AtLimit {
        limit: String,
        max: bool,
    },
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rejection::NoDomNode => {
                write!(f, "Element hat kein DOM-Gegenstück, Aktion nicht möglich")
            }
            Rejection::Disabled => write!(f, "Element ist deaktiviert"),
            Rejection::Unsupported { role, action } => {
                write!(f, "„{action}“ ist für {role} nicht möglich")
            }
            Rejection::UnknownOption { available } => {
                write!(
                    f,
                    "Option gibt es nicht. Verfügbar: {}",
                    available.join(", ")
                )
            }
            Rejection::AtLimit { limit, max } => write!(
                f,
                "Wert steht schon auf dem {} ({limit})",
                if *max { "Maximum" } else { "Minimum" }
            ),
        }
    }
}

fn is_activatable(role: &Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::Link
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Tab
            | Role::MenuItem
            | Role::MenuItemCheckbox
            | Role::MenuItemRadio
            | Role::TreeItem
            | Role::Combobox
    )
}

fn is_editable(role: &Role) -> bool {
    matches!(
        role,
        Role::Textbox | Role::SearchBox | Role::Combobox | Role::SpinButton
    )
}

fn is_selectable(role: &Role) -> bool {
    matches!(role, Role::Combobox | Role::Listbox)
}

fn is_steppable(role: &Role) -> bool {
    matches!(role, Role::Slider | Role::SpinButton)
}

/// Signalwörter für folgenreiche Aktionen. Konservativ: lieber einmal zu oft
/// nachfragen. Die Einstufung ist selbst eine Inferenz.
const HIGH_RISK_WORDS: &[&str] = &[
    "kaufen",
    "bestellen",
    "bestellung",
    "zahlen",
    "bezahlen",
    "zahlung",
    "senden",
    "absenden",
    "abschicken",
    "löschen",
    "entfernen",
    "kündigen",
    "bestätigen",
    "buy",
    "order",
    "pay",
    "checkout",
    "send",
    "submit",
    "delete",
    "remove",
    "confirm",
    "cancel subscription",
    "überweisen",
    "abonnieren",
];

pub fn plan(control: &Control, kind: ActionKind) -> Result<ActionPlan, Rejection> {
    let dom_node_id = control.dom_node_id.ok_or(Rejection::NoDomNode)?;
    if control.disabled {
        return Err(Rejection::Disabled);
    }
    let role = &control.role;
    let unsupported = |action| Rejection::Unsupported {
        role: role.to_string(),
        action,
    };

    let mut notes = Vec::new();
    let (kind, risk) = match kind {
        ActionKind::Focus => (ActionKind::Focus, Risk::Low),
        ActionKind::NavigateTo => (ActionKind::NavigateTo, Risk::Low),
        ActionKind::Increment | ActionKind::Decrement => {
            let up = kind == ActionKind::Increment;
            if !is_steppable(role) {
                return Err(unsupported(if up { "erhöhen" } else { "verringern" }));
            }
            if let Some(limit) = at_limit(control, up) {
                return Err(Rejection::AtLimit { limit, max: up });
            }
            (kind, Risk::Medium)
        }
        ActionKind::Activate => {
            // Ohne Bedienrolle nur, wenn Chromium selbst einen Klick meldet.
            if !is_activatable(role) && !control.clickable {
                return Err(unsupported("aktivieren"));
            }
            (ActionKind::Activate, activation_risk(control, &mut notes))
        }
        ActionKind::SetValue(v) => {
            if !is_editable(role) {
                return Err(unsupported("ausfüllen"));
            }
            (ActionKind::SetValue(v), Risk::Medium)
        }
        ActionKind::Select(wanted) => {
            if !is_selectable(role) {
                return Err(unsupported("auswählen"));
            }
            let exact = control
                .options
                .iter()
                .find(|o| o.eq_ignore_ascii_case(&wanted))
                .or_else(|| {
                    control
                        .options
                        .iter()
                        .find(|o| o.to_lowercase().starts_with(&wanted.to_lowercase()))
                })
                .cloned()
                .ok_or_else(|| Rejection::UnknownOption {
                    available: control.options.clone(),
                })?;
            (ActionKind::Select(exact), Risk::Medium)
        }
    };

    let mut requires_confirmation = risk == Risk::High;
    if control.name.certainty == Certainty::Uncertain
        && !matches!(kind, ActionKind::Focus | ActionKind::NavigateTo)
    {
        requires_confirmation = true;
        notes.push("Ziel hat keinen sicheren Namen".into());
    }

    Ok(ActionPlan {
        target: control.node.clone(),
        dom_node_id,
        kind,
        risk,
        requires_confirmation,
        notes,
    })
}

/// Wie [`plan`], mit dem Seitentyp der Seite: Auf einer Anmelde- oder
/// Kassenseite (auch nur unsicher erschlossen) wird jede Aktivierung ab
/// MEDIUM zu HIGH. Der Seitentyp erhöht nur, er senkt nie; Links, Tabs und
/// Baumeinträge (LOW) sowie Ausfüllen und Auswählen bleiben, wie sie sind.
///
/// Hosts mit einem [`Page`] rufen diese Funktion; [`plan`] allein kennt
/// keinen Seitentyp.
pub fn plan_on_page(
    page: &Page,
    control: &Control,
    kind: ActionKind,
) -> Result<ActionPlan, Rejection> {
    let mut plan = plan(control, kind)?;
    let page_type = &page.kind;
    if let Some(t) = page_type.value.filter(|t| t.is_sensitive()) {
        if plan.kind == ActionKind::Activate && plan.risk == Risk::Medium {
            let how = if page_type.certainty == Certainty::Uncertain {
                "möglicherweise"
            } else {
                "vermutlich"
            };
            plan.risk = Risk::High;
            plan.requires_confirmation = true;
            plan.notes.push(format!(
                "Seitentyp {how} {} ({})",
                t.label(),
                page_type.evidence.join(", ")
            ));
        }
    }
    Ok(plan)
}

/// Zu einer Überschrift oder einem Bereich navigieren: nur lesen, LOW.
pub fn plan_navigation(
    target: &NodeRef,
    dom_node_id: Option<i64>,
) -> Result<ActionPlan, Rejection> {
    Ok(ActionPlan {
        target: target.clone(),
        dom_node_id: dom_node_id.ok_or(Rejection::NoDomNode)?,
        kind: ActionKind::NavigateTo,
        risk: Risk::Low,
        requires_confirmation: false,
        notes: vec![],
    })
}

/// Grenze, an der der Wert schon steht. Ohne Zahlenwert oder Grenze keine
/// Aussage — dann entscheidet das Element selbst.
fn at_limit(control: &Control, up: bool) -> Option<String> {
    let key = if up { "valuemax" } else { "valuemin" };
    let limit = &control.states.iter().find(|(k, _)| k == key)?.1;
    let value: f64 = control.value.as_deref()?.parse().ok()?;
    let bound: f64 = limit.parse().ok()?;
    let reached = if up { value >= bound } else { value <= bound };
    reached.then(|| limit.clone())
}

fn activation_risk(control: &Control, notes: &mut Vec<String>) -> Risk {
    let name = control.name.value.as_deref().unwrap_or("").to_lowercase();
    if let Some(word) = HIGH_RISK_WORDS.iter().find(|w| name.contains(*w)) {
        notes.push(format!("Name enthält „{word}“"));
        return Risk::High;
    }
    match control.role {
        Role::Link | Role::Tab | Role::TreeItem => Risk::Low,
        // Ein unbenannter Button kann alles auslösen.
        Role::Button if control.name.certainty == Certainty::Uncertain => {
            notes.push("unbenannter Button, Wirkung unbekannt".into());
            Risk::High
        }
        _ => Risk::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::at;
    use relief_model::Fact;

    fn button(name: Option<&str>) -> Control {
        Control {
            node: at(1),
            dom_node_id: Some(1),
            role: Role::Button,
            name: Fact {
                certainty: if name.is_some() {
                    Certainty::Known
                } else {
                    Certainty::Uncertain
                },
                ..Fact::known(name.map(String::from))
            },
            region: None,
            value: None,
            options: vec![],
            selected_option: None,
            disabled: false,
            focusable: true,
            clickable: false,
            states: vec![],
            heading: None,
        }
    }

    #[test]
    fn high_risk_needs_confirmation() {
        let p = plan(&button(Some("Jetzt kaufen")), ActionKind::Activate).unwrap();
        assert_eq!(p.risk, Risk::High);
        assert!(p.requires_confirmation);
        let p = plan(&button(Some("Größentabelle")), ActionKind::Activate).unwrap();
        assert_eq!(p.risk, Risk::Medium);
        assert!(!p.requires_confirmation);
        let p = plan(&button(None), ActionKind::Activate).unwrap();
        assert_eq!(p.risk, Risk::High);
    }

    #[test]
    fn increment_only_on_range_controls_within_limits() {
        let mut c = button(Some("Menge"));
        assert!(matches!(
            plan(&c, ActionKind::Increment),
            Err(Rejection::Unsupported { .. })
        ));
        c.role = Role::SpinButton;
        c.value = Some("1".into());
        c.states = vec![
            ("valuemin".into(), "1".into()),
            ("valuemax".into(), "5".into()),
        ];
        let p = plan(&c, ActionKind::Increment).unwrap();
        assert_eq!(p.risk, Risk::Medium);
        assert!(!p.requires_confirmation);
        assert_eq!(
            plan(&c, ActionKind::Decrement).unwrap_err(),
            Rejection::AtLimit {
                limit: "1".into(),
                max: false
            }
        );
        c.value = Some("5".into());
        assert!(matches!(
            plan(&c, ActionKind::Increment),
            Err(Rejection::AtLimit { max: true, .. })
        ));
        // Ohne Grenzen entscheidet das Element.
        c.states.clear();
        assert!(plan(&c, ActionKind::Increment).is_ok());
    }

    #[test]
    fn navigation_is_low_risk_without_confirmation() {
        let p = plan_navigation(&at(7), Some(7)).unwrap();
        assert_eq!(
            (p.kind, p.risk, p.requires_confirmation),
            (ActionKind::NavigateTo, Risk::Low, false)
        );
        assert_eq!(
            plan_navigation(&at(7), None).unwrap_err(),
            Rejection::NoDomNode
        );
        let p = plan(&button(None), ActionKind::NavigateTo).unwrap();
        assert!(!p.requires_confirmation);
    }

    #[test]
    fn select_needs_existing_option() {
        let mut c = button(Some("Größe"));
        c.role = Role::Combobox;
        c.options = vec!["39".into(), "43".into()];
        assert_eq!(
            plan(&c, ActionKind::Select("43".into())).unwrap().kind,
            ActionKind::Select("43".into())
        );
        assert!(matches!(
            plan(&c, ActionKind::Select("44".into())),
            Err(Rejection::UnknownOption { .. })
        ));
    }
}
