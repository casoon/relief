//! Sicherheitsgrenzen zwischen Befehl und Browseraktion
//! (→ `plan/spezifikation/07`, „Sicherheits-Regressionsmatrix“).
//!
//! - **Bestätigung** ([`Binding`], `Confirmation`): Eine Rückfrage erzeugt ein
//!   Token, das an den angezeigten Plan gebunden ist: Aktion mit Wert,
//!   Zielknoten, Graph-Version, Zieladresse, Seite und den relevanten
//!   Ausschnitt (Bedienelement, Seitentyp). Es gilt nur für die nächste
//!   Eingabe, höchstens [`CONFIRMATION_TTL`] lang, und wird beim Einlösen
//!   verbraucht. Das Token ist weder `Clone` noch serialisierbar: Aus Cache,
//!   Profil oder einer Kopie der Sitzung entsteht keine Freigabe.
//! - **Security-Log** ([`SecurityEvent`]): Entscheidung, Plan-ID, Aktion
//!   (nur die Art), Risikoklasse, Grund. Keine Feldwerte, keine Namen, keine
//!   Eingabe.
//! - **Grenzen** ([`Limit`]): welche Grenze eine Aufgabe beendet hat; gezählt
//!   wird im Modellvertrag (`relief_ai_contract::Budget`).
//! - **Angaben der Hosts** ([`FORM_ACTION`], [`INPUT_TYPE`],
//!   [`HTML_AUTOCOMPLETE`]): was der Accessibility-Tree nicht trägt, legt der
//!   Host in `SemanticNode::extra` ab; [`is_sensitive_field`] entscheidet
//!   daraus, ob ein Wert in der Rückfrage verdeckt wird.

use std::collections::VecDeque;
use std::fmt;
use std::mem::discriminant;
use std::time::{Duration, Instant};

use relief_model::{Certainty, GraphVersion, NodeRef, SemanticGraph, Toggle};
use serde::Serialize;

use crate::graph::Control;
use crate::page::{Page, PageType};
use crate::validate::{ActionKind, ActionPlan, Risk};

/// So lange gilt eine Rückfrage [Annahme: genug zum Lesen oder Anhören der
/// Rückfrage, kurz genug, dass die Seite sich kaum unbemerkt ändert].
pub const CONFIRMATION_TTL: Duration = Duration::from_secs(60);

/// So viele Einträge hält das Security-Log, bis der Host sie abholt; ältere
/// fallen weg.
pub const LOG_CAPACITY: usize = 256;

/// Schlüssel in `SemanticNode::extra`: Formularziel eines Absenden-Buttons,
/// vollständige Adresse (`formaction` des Buttons, sonst `action` des
/// Formulars, aufgelöst gegen die Basisadresse). Füllt der CDP-Host aus dem
/// DOM; Chromium serialisiert es nicht in `AXNodeData`.
pub const FORM_ACTION: &str = "formAction";
/// Schlüssel in `SemanticNode::extra`: HTML-`type` eines `<input>`
/// (Chromium: `kInputType`, CDP-Host: DOM).
pub const INPUT_TYPE: &str = "inputType";
/// Schlüssel in `SemanticNode::extra`: HTML-`autocomplete` (nicht
/// `aria-autocomplete`, das unter `autocomplete` steht). Nur der CDP-Host
/// kennt es.
pub const HTML_AUTOCOMPLETE: &str = "htmlAutocomplete";

/// `autocomplete`-Token für Zugangs- und Identitätsdaten; dazu alle `cc-*`.
const SENSITIVE_AUTOCOMPLETE: &[&str] = &[
    "current-password",
    "new-password",
    "one-time-code",
    "username",
    "webauthn",
    "bday",
    "bday-day",
    "bday-month",
    "bday-year",
    "sex",
];

/// Passwortfeld (`type=password`) oder `autocomplete` für Zahlungs- und
/// Identitätsdaten (→ `plan/spezifikation/07`). Gilt für die Rückfrage wie
/// für den Privacy-Filter.
pub fn is_sensitive_field(input_type: Option<&str>, autocomplete: Option<&str>) -> bool {
    let password = input_type.is_some_and(|t| t.eq_ignore_ascii_case("password"));
    let autocomplete = autocomplete.is_some_and(|a| {
        a.split_ascii_whitespace().any(|token| {
            let token = token.to_ascii_lowercase();
            token.starts_with("cc-") || SENSITIVE_AUTOCOMPLETE.contains(&token.as_str())
        })
    });
    password || autocomplete
}

/// Nummer eines Plans in einer Sitzung; verbindet Rückfrage, Bestätigung
/// und Ausführung im Log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PlanId(pub u64);

/// Wofür eine Bestätigung gilt.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// Aktion samt Wert (`SetValue`, `Select`).
    pub kind: ActionKind,
    pub target: NodeRef,
    pub dom_node_id: i64,
    pub risk: Risk,
    /// Stand, auf dem die Rückfrage entstand.
    pub version: GraphVersion,
    /// Zieladresse des Ziels, vollständig: Link-Ziel oder Formularziel
    /// eines Absenden-Buttons ([`FORM_ACTION`]).
    pub destination: Option<String>,
    /// Adresse des Hauptdokuments.
    pub page: Option<String>,
    /// Relevanter Ausschnitt: gilt, wenn die Graph-Version sich geändert hat.
    pub section: Section,
    /// Werte der Felder im Formular des Ziels (Knoten, Wert, angehakt); nur
    /// im Speicher, nie im Log.
    pub form: Vec<(NodeRef, Option<String>, Option<Toggle>)>,
}

/// Was an Ziel und Seite für die Entscheidung zählt. Indizes in den Graphen
/// (Bereich, Abschnitt) fehlen bewusst: Sie verschieben sich, wenn anderswo
/// etwas dazukommt.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    role: String,
    name: Option<String>,
    name_certainty: Certainty,
    value: Option<String>,
    options: Vec<String>,
    selected_option: Option<String>,
    disabled: bool,
    states: Vec<(String, String)>,
    page_type: Option<PageType>,
    page_certainty: Certainty,
}

/// Was sich zwischen Rückfrage und Bestätigung geändert hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Changed {
    /// Andere Seite (Navigation).
    Page,
    Target,
    Action,
    Value,
    Destination,
    Risk,
    /// Graph-Version neu und Ziel oder Seitentyp verändert.
    Section,
    /// Ein Feldwert im Formular des Ziels hat sich geändert.
    Form,
}

impl Binding {
    pub fn new(plan: &ActionPlan, control: &Control, page: &Page, model: &SemanticGraph) -> Self {
        Binding {
            kind: plan.kind.clone(),
            target: plan.target.clone(),
            dom_node_id: plan.dom_node_id,
            risk: plan.risk,
            version: model.version,
            destination: model
                .node(&plan.target)
                .and_then(|n| n.url.clone().or_else(|| n.extra.get(FORM_ACTION).cloned())),
            page: model
                .root
                .as_ref()
                .and_then(|t| model.trees.get(t))
                .and_then(|t| t.data.url.clone()),
            section: Section {
                role: control.role.to_string(),
                name: control.name.value.clone(),
                name_certainty: control.name.certainty,
                value: control.value.clone(),
                options: control.options.clone(),
                selected_option: control.selected_option.clone(),
                disabled: control.disabled,
                states: control.states.clone(),
                page_type: page.kind.value,
                page_certainty: page.kind.certainty,
            },
            form: Vec::new(),
        }
    }

    /// Werte der Formularfelder dazunehmen (→ [`Binding::form`]).
    pub fn with_form(mut self, form: Vec<(NodeRef, Option<String>, Option<Toggle>)>) -> Self {
        self.form = form;
        self
    }

    /// Deckt eine Bestätigung für `self` auch `now`? `None`: ja. Die
    /// Graph-Version darf sich ändern, wenn der Ausschnitt gleich blieb
    /// (→ `spezifikation/05`, Validierung Schritt 2).
    pub fn changed(&self, now: &Binding) -> Option<Changed> {
        if self.page != now.page {
            Some(Changed::Page)
        } else if self.target != now.target || self.dom_node_id != now.dom_node_id {
            Some(Changed::Target)
        } else if discriminant(&self.kind) != discriminant(&now.kind) {
            Some(Changed::Action)
        } else if self.kind != now.kind {
            Some(Changed::Value)
        } else if self.destination != now.destination {
            Some(Changed::Destination)
        } else if self.risk != now.risk {
            Some(Changed::Risk)
        } else if self.form != now.form {
            Some(Changed::Form)
        } else if self.version != now.version && self.section != now.section {
            Some(Changed::Section)
        } else {
            None
        }
    }
}

/// Offene Rückfrage. Bewusst weder `Clone` noch serialisierbar; Einlösen
/// verbraucht sie.
#[derive(Debug)]
pub(crate) struct Confirmation {
    pub(crate) id: PlanId,
    binding: Binding,
    issued: Instant,
}

impl Confirmation {
    pub(crate) fn new(id: PlanId, binding: Binding) -> Self {
        Confirmation {
            id,
            binding,
            issued: Instant::now(),
        }
    }

    /// Einlösen für den Plan, der jetzt entstanden ist.
    pub(crate) fn redeem(self, now: &Binding, ttl: Duration) -> Result<PlanId, Reason> {
        if self.issued.elapsed() >= ttl {
            return Err(Reason::Expired);
        }
        match self.binding.changed(now) {
            None => Ok(self.id),
            Some(c) => Err(Reason::Changed(c)),
        }
    }
}

/// Grenze je Seite oder Aufgabe (→ `spezifikation/07`, Ressourcenmissbrauch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Limit {
    /// Knoten in einer Modelleingabe.
    TreeSize,
    ModelCalls,
    /// Dieselbe Anfrage noch einmal (Schleife).
    Repeats,
    Time,
    /// Tokens, wie der Anbieter sie meldet.
    Cost,
}

impl Limit {
    pub fn label(self) -> &'static str {
        match self {
            Limit::TreeSize => "Knoten in der Modelleingabe",
            Limit::ModelCalls => "Modellaufrufe",
            Limit::Repeats => "Wiederholungen derselben Anfrage",
            Limit::Time => "Sekunden",
            Limit::Cost => "Tokens",
        }
    }
}

/// Warum etwas nicht ausgeführt oder eine Aufgabe beendet wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// „!“ ohne offene Rückfrage (nie gestellt, schon verbraucht oder
    /// durch eine andere Eingabe verworfen).
    NoPrompt,
    Expired,
    Changed(Changed),
    /// Die Validierung hat den Plan abgelehnt.
    Invalid,
    Limit(Limit),
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::NoPrompt => f.write_str("keine offene Rückfrage zu dieser Aktion"),
            Reason::Expired => f.write_str("Rückfrage abgelaufen"),
            Reason::Changed(c) => f.write_str(match c {
                Changed::Page => "andere Seite",
                Changed::Target => "anderes Ziel",
                Changed::Action => "andere Aktion",
                Changed::Value => "anderer Wert",
                Changed::Destination => "andere Zieladresse",
                Changed::Risk => "andere Risikoklasse",
                Changed::Section => "Ziel oder Seite hat sich geändert",
                Changed::Form => "ein Feldwert im Formular hat sich geändert",
            }),
            Reason::Invalid => f.write_str("Aktion abgelehnt"),
            Reason::Limit(l) => write!(f, "Grenze für {} erreicht", l.label()),
        }
    }
}

/// Was die Runtime entschieden hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Ausgeführt ohne Rückfrage (Risiko erlaubt es).
    Perform,
    /// Rückfrage gestellt, Token ausgegeben.
    AskConfirmation,
    /// Ausgeführt nach eingelöster Rückfrage.
    PerformConfirmed,
    /// Nicht ausgeführt.
    Reject,
    /// Aufgabe beendet (Grenze).
    Abort,
}

/// Ein Eintrag im Security-Log. Enthält nie Feldwerte, Namen oder Eingaben.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SecurityEvent {
    pub decision: Decision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanId>,
    /// Art der Aktion ohne Wert, z. B. `set_value`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

impl SecurityEvent {
    /// Eintrag zu einem Plan.
    pub fn plan(decision: Decision, id: Option<PlanId>, plan: &ActionPlan) -> Self {
        SecurityEvent {
            decision,
            plan: id,
            action: Some(action_name(&plan.kind)),
            risk: Some(plan.risk),
            reason: None,
        }
    }

    /// Die Validierung hat die Aktion abgelehnt; es gibt keinen Plan.
    pub fn rejected(action: &'static str) -> Self {
        SecurityEvent {
            decision: Decision::Reject,
            plan: None,
            action: Some(action),
            risk: None,
            reason: Some(Reason::Invalid),
        }
    }

    /// Aufgabe an einer Grenze beendet.
    pub fn abort(limit: Limit) -> Self {
        SecurityEvent {
            decision: Decision::Abort,
            plan: None,
            action: None,
            risk: None,
            reason: Some(Reason::Limit(limit)),
        }
    }

    pub fn because(mut self, reason: Reason) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Name der Aktion fürs Log, ohne Wert.
pub fn action_name(kind: &ActionKind) -> &'static str {
    match kind {
        ActionKind::Focus => "focus",
        ActionKind::Activate => "activate",
        ActionKind::SetValue(_) => "set_value",
        ActionKind::Select(_) => "select",
        ActionKind::NavigateTo => "navigate_to",
        ActionKind::Increment => "increment",
        ActionKind::Decrement => "decrement",
    }
}

/// Security-Log einer Sitzung, höchstens [`LOG_CAPACITY`] Einträge.
#[derive(Debug, Default)]
pub(crate) struct SecurityLog(VecDeque<SecurityEvent>);

impl SecurityLog {
    pub(crate) fn push(&mut self, event: SecurityEvent) {
        if self.0.len() == LOG_CAPACITY {
            self.0.pop_front();
        }
        self.0.push_back(event);
    }

    pub(crate) fn take(&mut self) -> Vec<SecurityEvent> {
        self.0.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{at, sample_tree};
    use crate::validate::plan;
    use crate::Graph;

    fn binding(model: &SemanticGraph, name: &str, kind: ActionKind) -> Binding {
        let graph = Graph::build(model);
        let control = graph
            .controls
            .iter()
            .find(|c| c.name.value.as_deref() == Some(name))
            .unwrap();
        let p = plan(control, kind).unwrap();
        Binding::new(&p, control, &graph.page, model)
    }

    #[test]
    fn gleicher_plan_gilt_auch_bei_neuer_version_mit_gleichem_ausschnitt() {
        let model = sample_tree();
        let before = binding(&model, "Bestellen", ActionKind::Activate);
        let mut later = model.clone();
        later.version = GraphVersion(model.version.0 + 3);
        let now = binding(&later, "Bestellen", ActionKind::Activate);
        assert_eq!(before.changed(&now), None);
    }

    #[test]
    fn jedes_gebundene_feld_verlangt_neue_bestaetigung() {
        let model = sample_tree();
        let fill = |v: &str| binding(&model, "Menge", ActionKind::SetValue(v.into()));
        assert_eq!(fill("2").changed(&fill("3")), Some(Changed::Value));
        assert_eq!(
            fill("2").changed(&binding(&model, "Menge", ActionKind::Focus)),
            Some(Changed::Action)
        );
        let order = binding(&model, "Bestellen", ActionKind::Activate);
        assert_eq!(
            order.changed(&binding(&model, "Pflegehinweisen", ActionKind::Activate)),
            Some(Changed::Target)
        );

        // Zieladresse: derselbe Link zeigt jetzt woandershin.
        let mut a = model.clone();
        a.trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .get_mut(&at(7).node)
            .unwrap()
            .url = Some("https://shop.example/pflege".into());
        let mut b = a.clone();
        b.trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .get_mut(&at(7).node)
            .unwrap()
            .url = Some("https://evil.example/pflege".into());
        assert_eq!(
            binding(&a, "Pflegehinweisen", ActionKind::Activate).changed(&binding(
                &b,
                "Pflegehinweisen",
                ActionKind::Activate
            )),
            Some(Changed::Destination)
        );

        // Formularziel: derselbe Absenden-Button schickt jetzt woandershin.
        let with_action = |url: &str| {
            let mut m = model.clone();
            m.trees
                .values_mut()
                .next()
                .unwrap()
                .nodes
                .get_mut(&at(21).node)
                .unwrap()
                .extra
                .insert(FORM_ACTION.into(), url.into());
            m
        };
        let shop = binding(
            &with_action("https://shop.example/bestellung"),
            "Bestellen",
            ActionKind::Activate,
        );
        assert_eq!(
            shop.destination.as_deref(),
            Some("https://shop.example/bestellung")
        );
        assert_eq!(
            shop.changed(&binding(
                &with_action("https://evil.example/bestellung"),
                "Bestellen",
                ActionKind::Activate
            )),
            Some(Changed::Destination)
        );

        // Andere Seite.
        let mut c = model.clone();
        c.trees.values_mut().next().unwrap().data.url = Some("https://shop.example/kasse".into());
        assert_eq!(
            order.changed(&binding(&c, "Bestellen", ActionKind::Activate)),
            Some(Changed::Page)
        );

        // Neue Version, Ziel verändert (jetzt deaktiviert → anderer Ausschnitt).
        let mut d = model.clone();
        d.version = GraphVersion(model.version.0 + 1);
        d.trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .get_mut(&at(21).node)
            .unwrap()
            .states
            .expanded = Some(true);
        assert_eq!(
            order.changed(&binding(&d, "Bestellen", ActionKind::Activate)),
            Some(Changed::Section)
        );
    }

    #[test]
    fn sensible_felder() {
        assert!(is_sensitive_field(Some("Password"), None));
        assert!(is_sensitive_field(None, Some("shipping cc-number")));
        assert!(is_sensitive_field(Some("text"), Some("one-time-code")));
        assert!(!is_sensitive_field(Some("text"), Some("name")));
        assert!(!is_sensitive_field(None, None));
    }

    #[test]
    fn token_laeuft_ab() {
        let model = sample_tree();
        let b = binding(&model, "Bestellen", ActionKind::Activate);
        let token = Confirmation::new(PlanId(1), b.clone());
        assert_eq!(token.redeem(&b, Duration::ZERO), Err(Reason::Expired));
        let token = Confirmation::new(PlanId(2), b.clone());
        assert_eq!(token.redeem(&b, CONFIRMATION_TTL), Ok(PlanId(2)));
    }

    #[test]
    fn log_ist_begrenzt() {
        let mut log = SecurityLog::default();
        for _ in 0..LOG_CAPACITY + 5 {
            log.push(SecurityEvent::abort(Limit::Time));
        }
        assert_eq!(log.take().len(), LOG_CAPACITY);
        assert!(log.take().is_empty());
    }

    #[test]
    fn geaenderter_feldwert_im_formular_macht_die_bestaetigung_ungueltig() {
        let model = crate::graph::sample_tree();
        let graph = Graph::build(&model);
        let control = graph.controls[0].clone();
        let plan = plan(&control, ActionKind::Activate).unwrap();
        let field = crate::graph::at(20);
        let before = Binding::new(&plan, &control, &graph.page, &model)
            .with_form(vec![(field.clone(), Some("1".into()), None)]);
        let after = Binding::new(&plan, &control, &graph.page, &model)
            .with_form(vec![(field, Some("2".into()), None)]);
        assert_eq!(before.changed(&before.clone()), None);
        assert_eq!(before.changed(&after), Some(Changed::Form));
    }

}
