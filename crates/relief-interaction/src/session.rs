//! Ein Befehl von der Eingabe bis zur Antwort, browserfrei.
//!
//! Beide Hosts (CDP-Spike, Fork) nehmen den aktuellen Stand auf, geben ihn
//! mit der Eingabe hierher und führen nur aus, was zurückkommt
//! ([`Outcome`]); die Antwort nach einer Aktion entsteht wieder hier
//! ([`Session::performed`], [`Session::escaped`]).
//!
//! **Position**: Relief steht dort, wo der Fokus steht, außer nach dem
//! Hinbewegen zu einer Überschrift oder einem Bereich
//! ([`ActionKind::NavigateTo`]). Im Fork lässt sich ein nicht fokussierbares
//! Ziel über den AX-Weg nicht fokussieren; Chromium setzt dort nur den
//! Startpunkt der Tab-Reihenfolge und nimmt den Fokus weg (wie ein
//! Screenreader mit eigenem Cursor). Diese Position gilt, bis der Fokus sich
//! bewegt.
//!
//! **Bestätigung**: Verlangt ein Plan eine Bestätigung, antwortet die Sitzung
//! mit einer Rückfrage und merkt sich ein Token, das an genau diesen Plan
//! gebunden ist (→ [`crate::security`]). „!“ vor dem Befehl löst es ein, und
//! nur dann, wenn es die unmittelbar folgende Eingabe ist, derselbe Plan
//! wieder entsteht und die Rückfrage nicht abgelaufen ist. „!“ ohne offene
//! Rückfrage bestätigt nichts; es gibt keine pauschale Zustimmung.

use std::time::Duration;

use relief_model::{NodeRef, SemanticGraph};

use crate::command::{self, parse, Command, ScrollDirection, Step};
use crate::graph::{focused, Control, Graph};
use crate::marks::{marks, Mark};
use crate::overlay::{self, ButtonKind};
use crate::resolve::{
    current_place, dismissal, resolve, resolve_inflected, resolve_place, step_field, step_heading,
    Dismissal, Place, PlaceResolution, Resolution,
};
use crate::respond;
use crate::security::{
    action_name, is_sensitive_field, Binding, Confirmation, Decision, PlanId, Reason,
    SecurityEvent, SecurityLog, CONFIRMATION_TTL, HTML_AUTOCOMPLETE, INPUT_TYPE,
};
use crate::validate::{plan_navigation, plan_on_page, ActionKind, ActionPlan};

/// Was der Host als Nächstes tun soll.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// Fertige Antwort, nichts auszuführen.
    Answer(String),
    /// Geprüften Plan ausführen, Ruhe abwarten, dann [`Session::performed`].
    Perform { plan: ActionPlan, label: String },
    /// Escape an das fokussierte Element, dann [`Session::escaped`].
    /// `reaches`: Der Fokus liegt nicht in `target`, Escape erreicht
    /// stattdessen dieses Element.
    Escape {
        target: String,
        reaches: Option<String>,
    },
    /// Dokument scrollen (kein Zielelement, LOW), Antwort über
    /// [`respond::scrolled`].
    Scroll(ScrollDirection),
}

/// Eingabe ohne Seitenstand zerlegen: `!` am Anfang bestätigt die eben
/// gestellte Rückfrage (→ Moduldoku). Fehler: Antworttext (nicht verstanden).
pub fn parse_input(input: &str) -> Result<(bool, Command), String> {
    let (confirmed, input) = match input.trim().strip_prefix('!') {
        Some(rest) => (true, rest.trim()),
        None => (false, input.trim()),
    };
    Ok((confirmed, parse(input)?))
}

/// Braucht der Befehl den aktuellen Fokus? Hosts, die ihn erst erfragen
/// müssen (CDP), tun das nur dann.
pub fn uses_focus(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::WhereAmI
            | Command::FieldStep(_)
            | Command::SectionStep(_)
            | Command::Read(None)
            | Command::Dismiss
    )
}

/// Position nach einem Hinbewegen: das Ziel und der Fokus, der danach galt.
#[derive(Debug, Clone)]
struct Position {
    at: NodeRef,
    focus: Option<NodeRef>,
}

/// Zustand zwischen Befehlen einer Seite (eines Tabs).
///
/// Nicht kopierbar: Eine Kopie trüge die offene Rückfrage mit, die nur
/// einmal gelten darf.
///
/// ```compile_fail
/// let session = relief_interaction::Session::new();
/// let copy = session.clone();
/// ```
#[derive(Debug)]
pub struct Session {
    position: Option<Position>,
    /// Offene Rückfrage; gilt nur für die nächste Eingabe.
    confirmation: Option<Confirmation>,
    confirmation_ttl: Duration,
    plans: u64,
    log: SecurityLog,
    /// Mehrdeutiges Ziel der letzten Eingabe; gilt nur für die nächste.
    choices: Option<Choices>,
    /// Ziel der offenen Rückfrage: „ja“ plant genau dieses neu und löst
    /// die Rückfrage ein.
    confirm_target: Option<(Control, ActionKind)>,
    /// Sprungmarken des zuletzt gezeigten Stands (→ `marks`).
    marks: Vec<Mark>,
    /// Letzte Eingabe als Befehl (ohne „!“): „ja“ bestätigt sie.
    last_input: Option<String>,
}

/// Kandidaten einer Rückfrage „Mehrdeutig“.
#[derive(Debug)]
enum Choices {
    Controls(Vec<Control>, ActionKind),
    Places(Vec<Place>),
}

/// Was eine Eingabe vor der Befehlszerlegung bewirkt
/// (→ [`Session::pending_reply`]).
#[derive(Debug)]
pub enum Pending {
    /// Die Eingabe beantwortet eine Rückfrage; das ist das Ergebnis.
    Done(Outcome),
    /// Ein neuer Befehl; offene Rückfragen sind verworfen bzw. gelten nur
    /// noch für „!“ in dieser Eingabe.
    Command,
}

/// Nicht gefunden (Antwort) oder mehrdeutig (Antwort mit Kandidaten).
enum Miss<T> {
    Text(String),
    Many(String, Vec<T>),
}

impl<T> From<String> for Miss<T> {
    fn from(text: String) -> Self {
        Miss::Text(text)
    }
}

impl<T> Miss<T> {
    fn text(self) -> String {
        match self {
            Miss::Text(text) | Miss::Many(text, _) => text,
        }
    }
}

/// Wörter, die eine offene Rückfrage verwerfen, bzw. sie bestätigen.
const CANCEL: &[&str] = &["abbrechen", "abbruch", "nein", "stopp", "stop", "cancel"];
const CONFIRM: &[&str] = &["ja", "bestätigen", "bestätige", "yes"];

impl Default for Session {
    fn default() -> Self {
        Session {
            position: None,
            choices: None,
            last_input: None,
            confirm_target: None,
            marks: Vec::new(),
            confirmation: None,
            confirmation_ttl: CONFIRMATION_TTL,
            plans: 0,
            log: SecurityLog::default(),
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wie [`Session::new`], mit anderer Gültigkeit der Rückfrage.
    pub fn with_confirmation_ttl(ttl: Duration) -> Self {
        Session {
            confirmation_ttl: ttl,
            ..Self::default()
        }
    }

    /// Security-Log seit dem letzten Abholen (→ [`SecurityEvent`]).
    pub fn take_security_log(&mut self) -> Vec<SecurityEvent> {
        self.log.take()
    }

    /// Offene Rückfrage verwerfen, ohne einen Befehl zu verarbeiten (die
    /// Nutzerin lehnt ab, → `spezifikation/05`, „Abbrechen“).
    pub fn discard_confirmation(&mut self) {
        self.confirmation = None;
    }

    fn next_plan(&mut self) -> PlanId {
        self.plans += 1;
        PlanId(self.plans)
    }

    /// Wo Relief steht, gegeben den Fokus der Seite (→ Moduldoku).
    pub fn position(&self, focus: Option<&NodeRef>) -> Option<NodeRef> {
        match &self.position {
            Some(p) if p.focus.as_ref() == focus => Some(p.at.clone()),
            _ => focus.cloned(),
        }
    }

    /// Eingabe vor der Befehlszerlegung gegen offene Rückfragen prüfen: Zahl
    /// oder Name wählt einen Kandidaten, „ja“ bestätigt die gezeigte
    /// Rückfrage (wie „!“ vor derselben Eingabe), „abbrechen“ verwirft
    /// Auswahl und Rückfrage (→ `spezifikation/05`). Hosts rufen das vor
    /// [`parse_input`] und [`Session::handle`].
    pub fn pending_reply(&mut self, graph: &Graph, model: &SemanticGraph, input: &str) -> Pending {
        let text = input
            .trim()
            .trim_end_matches(['.', '!'])
            .trim()
            .to_lowercase();
        let choices = self.choices.take();
        if CANCEL.contains(&text.as_str()) {
            let open = choices.is_some() || self.confirmation.is_some();
            self.confirmation = None;
            self.confirm_target = None;
            self.last_input = None;
            return Pending::Done(Outcome::Answer(if open {
                "Abgebrochen. Nichts ausgeführt.".into()
            } else {
                "Nichts offen, das sich abbrechen ließe.".into()
            }));
        }
        if CONFIRM.contains(&text.as_str()) && self.confirmation.is_some() {
            if let Some((control, kind)) = self.confirm_target.take() {
                // Gegen den aktuellen Stand: dasselbe Element, falls noch da.
                let current = graph
                    .controls
                    .iter()
                    .find(|c| c.node == control.node)
                    .cloned()
                    .or_else(|| {
                        model.node(&control.node).map(|node| {
                            crate::graph::control(model, &control.node, node, control.region)
                        })
                    });
                let offered = self.confirmation.take();
                return Pending::Done(match current {
                    Some(c) => self.plan_control(graph, model, c, kind, true, offered),
                    None => Outcome::Answer(
                        "Das Ziel ist nicht mehr auf der Seite; nichts ausgeführt.".into(),
                    ),
                });
            }
        }
        if let Some(label) = text
            .strip_prefix("marke ")
            .or_else(|| text.strip_prefix("sprungmarke "))
        {
            self.confirmation = None;
            let label = label.trim();
            return Pending::Done(
                match self.marks.iter().find(|m| m.label == label).cloned() {
                    Some(mark) => {
                        self.plan_control(graph, model, mark.control, mark.kind, false, None)
                    }
                    None => Outcome::Answer(format!(
                        "Keine Sprungmarke „{label}“. „sprungmarken“ zeigt sie."
                    )),
                },
            );
        }
        if matches!(
            text.as_str(),
            "sprungmarken" | "zeige sprungmarken" | "marken"
        ) {
            self.confirmation = None;
            return Pending::Done(Outcome::Answer(self.list_marks(graph, model)));
        }
        match choices {
            Some(Choices::Controls(controls, kind)) => {
                let labels: Vec<String> = controls.iter().map(respond::control_line).collect();
                if let Some(i) = choose(&text, &labels) {
                    self.confirmation = None;
                    return Pending::Done(self.plan_control(
                        graph,
                        model,
                        controls[i].clone(),
                        kind,
                        false,
                        None,
                    ));
                }
            }
            Some(Choices::Places(places)) => {
                let labels: Vec<String> = places
                    .iter()
                    .map(|p| respond::place_label(graph, *p))
                    .collect();
                if let Some(i) = choose(&text, &labels) {
                    self.confirmation = None;
                    return Pending::Done(self.navigate(graph, places[i]));
                }
            }
            None => {}
        }
        self.last_input = Some(input.trim().trim_start_matches('!').trim().to_string());
        Pending::Command
    }

    /// Sprungmarken für den aktuellen Stand berechnen und merken.
    pub fn show_marks(&mut self, graph: &Graph, model: &SemanticGraph) -> &[Mark] {
        self.marks = marks(model, graph);
        &self.marks
    }

    fn list_marks(&mut self, graph: &Graph, model: &SemanticGraph) -> String {
        let marks = self.show_marks(graph, model);
        if marks.is_empty() {
            return "Keine Sprungmarken: kein Element mit Aktion und Position.".into();
        }
        let lines: Vec<String> = marks
            .iter()
            .map(|m| {
                format!(
                    "  {}: {}{}",
                    m.label,
                    respond::control_line(&m.control),
                    // Ohne Namen sagt `control_line` es schon.
                    if m.uncertain() && m.name().value.is_some() {
                        " (Name nicht gesichert)"
                    } else {
                        ""
                    }
                )
            })
            .collect();
        format!(
            "{} Sprungmarken:\n{}\n„marke <Buchstaben>“ wählt eine aus.",
            marks.len(),
            lines.join("\n")
        )
    }

    /// Nicht gefunden: Antwort. Mehrdeutig: Kandidaten merken, nummeriert
    /// antworten.
    fn ask<T>(&mut self, miss: Miss<T>, keep: impl FnOnce(Vec<T>) -> Choices) -> Outcome {
        match miss {
            Miss::Text(text) => Outcome::Answer(text),
            Miss::Many(text, choices) => {
                self.choices = Some(keep(choices));
                Outcome::Answer(text)
            }
        }
    }

    /// Einen zerlegten Befehl gegen den aktuellen Stand beantworten oder in
    /// einen geprüften Plan übersetzen. `focus`: Fokus der Seite, nur nötig,
    /// wenn [`uses_focus`] zutrifft. Vorher [`Session::pending_reply`].
    pub fn handle(
        &mut self,
        graph: &Graph,
        model: &SemanticGraph,
        confirmed: bool,
        cmd: Command,
        focus: Option<&NodeRef>,
    ) -> Outcome {
        // Jede Eingabe verbraucht die offene Rückfrage (einlösen oder verwerfen).
        let offered = self.confirmation.take();
        self.confirm_target = None;
        let here = self.position(focus);
        let (target, kind) = match cmd {
            Command::Help => return Outcome::Answer(command::HELP.into()),
            Command::Describe => return Outcome::Answer(respond::describe(graph)),
            Command::ListActions => return Outcome::Answer(respond::list_actions(graph)),
            Command::ListHeadings => return Outcome::Answer(respond::list_headings(graph)),
            Command::Overlays => return Outcome::Answer(respond::overlays(graph)),
            // Nur Auskunft: keine Auswahl merken, damit keine Nummer und kein
            // Name danach ein gesperrtes Element trifft.
            Command::Background => return Outcome::Answer(respond::background(graph)),
            Command::RejectConsent => (reject_consent(graph), ActionKind::Activate),
            Command::Focus(q) => (pick(graph, &q, |_| true), ActionKind::Focus),
            Command::Activate(q) => (pick(graph, &q, |_| true), ActionKind::Activate),
            Command::SetValue(q, v) => (pick(graph, &q, is_editable), ActionKind::SetValue(v)),
            Command::Select(Some(q), v) => (
                pick(graph, &q, |c| !c.options.is_empty()),
                ActionKind::Select(v),
            ),
            Command::Select(None, v) => (pick_by_option(graph, &v), ActionKind::Select(v)),
            Command::Increment(q) => (pick(graph, &q, is_steppable), ActionKind::Increment),
            Command::Decrement(q) => (pick(graph, &q, is_steppable), ActionKind::Decrement),
            Command::FieldStep(step) => {
                let field = step_field(graph, here.as_ref(), step)
                    .cloned()
                    .ok_or_else(|| {
                        Miss::Text(format!(
                            "Kein {} Formularfeld.",
                            match step {
                                Step::Next => "weiteres",
                                Step::Previous => "vorheriges",
                            }
                        ))
                    });
                (field, ActionKind::Focus)
            }
            Command::WhereAmI => return Outcome::Answer(respond::where_am_i(graph, here.as_ref())),
            Command::Scroll(direction) => return Outcome::Scroll(direction),
            Command::SectionStep(step) => {
                return match step_heading(graph, here.as_ref(), step) {
                    Some(h) => self.navigate(graph, Place::Heading(h)),
                    None => Outcome::Answer(format!(
                        "Keine {} Überschrift.",
                        match step {
                            Step::Next => "weitere",
                            Step::Previous => "vorherige",
                        }
                    )),
                };
            }
            Command::GoToPlace(q) => {
                return match pick_place(graph, &q) {
                    Ok(place) => self.navigate(graph, place),
                    Err(miss) => self.ask(miss, Choices::Places),
                }
            }
            Command::Read(q) => {
                let place = match q {
                    Some(q) => pick_place(graph, &q).map_err(Miss::text),
                    None => current_place(graph, here.as_ref()).ok_or_else(|| {
                        "Kein Abschnitt am Fokus. „lies den Abschnitt <Name>“ nennt einen.".into()
                    }),
                };
                return Outcome::Answer(match place {
                    Ok(place) => respond::read_place(graph, place),
                    Err(msg) => msg,
                });
            }
            Command::Inspect(q) => {
                let found = resolve_inflected(graph, &q, |_| true);
                return Outcome::Answer(match pick_from(graph, &q, found) {
                    Ok(c) => respond::inspect(&c),
                    Err(miss) => miss.text(),
                });
            }
            // Escape geht an den echten Fokus, nicht an die Position.
            Command::Dismiss => match dismissal(graph, model, focus) {
                Dismissal::Button(c) => (Ok(c.clone()), ActionKind::Activate),
                Dismissal::NothingOpen => {
                    return Outcome::Answer("Kein Dialog und kein aufgeklapptes Menü offen.".into())
                }
                Dismissal::Escape { target, reaches } => {
                    return Outcome::Escape { target, reaches }
                }
            },
        };
        match target {
            Ok(control) => self.plan_control(graph, model, control, kind, confirmed, offered),
            Err(miss) => self.ask(miss, |controls| Choices::Controls(controls, kind)),
        }
    }

    /// Plan für ein Bedienelement; riskant ohne eingelöste Rückfrage →
    /// Rückfrage mit Einmal-Bestätigung.
    fn plan_control(
        &mut self,
        graph: &Graph,
        model: &SemanticGraph,
        control: Control,
        kind: ActionKind,
        confirmed: bool,
        offered: Option<Confirmation>,
    ) -> Outcome {
        let label = respond::control_line(&control);

        let action = action_name(&kind);
        let kind_for_confirm = kind.clone();
        let plan = match plan_on_page(&graph.page, &control, kind) {
            Ok(p) => p,
            Err(rejection) => {
                self.log.push(SecurityEvent::rejected(action));
                return Outcome::Answer(format!("Abgelehnt: {rejection} ({label})"));
            }
        };
        if !plan.requires_confirmation {
            let id = self.next_plan();
            self.log
                .push(SecurityEvent::plan(Decision::Perform, Some(id), &plan));
            return Outcome::Perform { plan, label };
        }

        let binding = Binding::new(&plan, &control, &graph.page, model);
        let mut refused = None;
        if confirmed {
            let (id, redeemed) = match offered {
                Some(token) => (
                    Some(token.id),
                    token.redeem(&binding, self.confirmation_ttl),
                ),
                None => (None, Err(Reason::NoPrompt)),
            };
            match redeemed {
                Ok(id) => {
                    self.log.push(SecurityEvent::plan(
                        Decision::PerformConfirmed,
                        Some(id),
                        &plan,
                    ));
                    return Outcome::Perform { plan, label };
                }
                Err(reason) => {
                    self.log
                        .push(SecurityEvent::plan(Decision::Reject, id, &plan).because(reason));
                    refused = Some(reason);
                }
            }
        }

        let id = self.next_plan();
        self.log.push(SecurityEvent::plan(
            Decision::AskConfirmation,
            Some(id),
            &plan,
        ));
        let answer = confirmation_prompt(&plan, &control, model, &binding, refused);
        self.confirmation = Some(Confirmation::new(id, binding));
        self.confirm_target = Some((control, kind_for_confirm));
        Outcome::Answer(answer)
    }

    /// Antwort nach einem ausgeführten Plan: Stand vorher und nachher (nach
    /// der Ruhe). Merkt sich bei [`ActionKind::NavigateTo`] das Ziel als
    /// Position.
    pub fn performed(
        &mut self,
        plan: &ActionPlan,
        label: &str,
        before: (&SemanticGraph, &Graph),
        after: (&SemanticGraph, &Graph),
    ) -> String {
        let focus_before = self.position(focused(before.0).as_ref());
        let focus_after = focused(after.0);
        let position_after = if plan.kind == ActionKind::NavigateTo {
            self.position = Some(Position {
                at: plan.target.clone(),
                focus: focus_after.clone(),
            });
            Some(plan.target.clone())
        } else {
            self.position(focus_after.as_ref())
        };
        let target = respond::target_change(before.1, after.1, &plan.target)
            .map(|t| format!("{t}. "))
            .unwrap_or_default();
        format!(
            "{:?} auf {label}. {target}{}",
            plan.kind,
            respond::describe_diff_at(
                before.0,
                after.0,
                focus_before.as_ref(),
                position_after.as_ref()
            )
        )
    }

    /// Antwort nach Escape (→ [`Outcome::Escape`]).
    pub fn escaped(
        &self,
        target: &str,
        reaches: Option<&str>,
        before: &SemanticGraph,
        after: &SemanticGraph,
    ) -> String {
        let focus = match reaches {
            None => String::new(),
            Some(r) => format!(" Der Fokus liegt nicht darin, Escape ging an {r}."),
        };
        format!(
            "Escape für {target} (kein Schließen-Button gefunden).{focus} {}",
            respond::describe_diff_at(
                before,
                after,
                self.position(focused(before).as_ref()).as_ref(),
                self.position(focused(after).as_ref()).as_ref()
            )
        )
    }
}

/// Rückfrage aus dem validierten Plan und den lokalen Daten, nicht aus einer
/// freien Zusammenfassung: Risiko, Gründe, Aktion mit Wert, Ziel und
/// Zieladresse bzw. Formularziel (ohne Query und Fragment). Bei einem
/// sensiblen Feld (Passwort, `autocomplete` für Zahlungs- und
/// Identitätsdaten, → [`is_sensitive_field`]) stehen weder der neue noch der
/// bisherige Wert darin.
fn confirmation_prompt(
    plan: &ActionPlan,
    control: &Control,
    model: &SemanticGraph,
    binding: &Binding,
    refused: Option<Reason>,
) -> String {
    let node = model.node(&plan.target);
    let sensitive = node.is_some_and(|n| {
        is_sensitive_field(
            n.extra.get(INPUT_TYPE).map(String::as_str),
            n.extra.get(HTML_AUTOCOMPLETE).map(String::as_str),
        )
    });
    let (kind, label) = if sensitive {
        let kind = match &plan.kind {
            ActionKind::SetValue(_) => "SetValue(verdeckt)".to_string(),
            ActionKind::Select(_) => "Select(verdeckt)".to_string(),
            other => format!("{other:?}"),
        };
        let hidden = Control {
            value: None,
            ..control.clone()
        };
        (kind, respond::control_line(&hidden))
    } else {
        (format!("{:?}", plan.kind), respond::control_line(control))
    };
    let what = if node.is_some_and(|n| n.url.is_some()) {
        "Adresse"
    } else {
        "Formularziel"
    };
    let destination = binding
        .destination
        .as_deref()
        .map(|d| format!(", {what}: {}", d.split(['?', '#']).next().unwrap_or(d)))
        .unwrap_or_default();
    let refused = refused
        .map(|r| format!(" Die Bestätigung galt nicht: {r}."))
        .unwrap_or_default();
    format!(
        "Bestätigung nötig ({:?}): {} — Aktion: {kind}, Ziel: {label}{destination}. \
         „ja“ (oder „!“ vor denselben Befehl) bestätigt einmal und nur genau diese \
         Aktion, „abbrechen“ verwirft sie.{refused}",
        plan.risk,
        plan.notes.join("; "),
    )
}

impl Session {
    /// Zu einer Überschrift oder einem Bereich (LOW, ohne Rückfrage).
    fn navigate(&mut self, graph: &Graph, place: Place) -> Outcome {
        let (node, dom_node_id) = match place {
            Place::Heading(h) => {
                let h = &graph.headings[h];
                (h.node.clone(), h.dom_node_id)
            }
            Place::Region(r) => {
                let r = &graph.regions[r];
                (r.node.clone(), r.dom_node_id)
            }
        };
        let label = respond::place_label(graph, place);
        match plan_navigation(&node, dom_node_id) {
            Ok(plan) => {
                let id = self.next_plan();
                self.log
                    .push(SecurityEvent::plan(Decision::Perform, Some(id), &plan));
                Outcome::Perform { plan, label }
            }
            Err(rejection) => Outcome::Answer(format!("Abgelehnt: {rejection} ({label})")),
        }
    }
}

/// Nummerierte Kandidatenliste mit Hinweis, wie gewählt wird.
fn numbered(head: String, labels: impl Iterator<Item = String>) -> String {
    let list = labels
        .enumerate()
        .map(|(i, l)| format!("  {}. {l}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}\n{list}\nZahl oder Name wählt aus, „abbrechen“ verwirft.")
}

/// Kandidat zu einer Antwort: Zahl (1-basiert) oder ein Text, der genau
/// einen Kandidaten trifft.
fn choose(text: &str, labels: &[String]) -> Option<usize> {
    if let Ok(n) = text.parse::<usize>() {
        return (1..=labels.len()).contains(&n).then(|| n - 1);
    }
    if text.is_empty() {
        return None;
    }
    let hits: Vec<usize> = labels
        .iter()
        .enumerate()
        .filter(|(_, l)| l.to_lowercase().contains(text))
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn pick_place(graph: &Graph, query: &str) -> Result<Place, Miss<Place>> {
    match resolve_place(graph, query) {
        PlaceResolution::One(place) => Ok(place),
        PlaceResolution::None => Err(Miss::Text(format!(
            "Keine Überschrift und kein Bereich „{query}“ gefunden."
        ))),
        PlaceResolution::Many(places) => Err(Miss::Many(
            numbered(
                format!("Mehrdeutig, „{query}“ passt auf:"),
                places.iter().map(|p| respond::place_label(graph, *p)),
            ),
            places,
        )),
    }
}

fn pick(
    graph: &Graph,
    query: &str,
    accept: impl Fn(&Control) -> bool,
) -> Result<Control, Miss<Control>> {
    pick_from(graph, query, resolve(graph, query, accept))
}

fn pick_from(graph: &Graph, query: &str, found: Resolution) -> Result<Control, Miss<Control>> {
    match found {
        Resolution::One(c) => Ok(c.clone()),
        Resolution::None => Err(Miss::Text(match graph.active_modal() {
            Some(m)
                if graph.controls.iter().any(|c| {
                    !graph.is_reachable(c.region, &c.node)
                        && c.name
                            .value
                            .as_deref()
                            .is_some_and(|n| n.to_lowercase().contains(&query.to_lowercase()))
                }) =>
            {
                format!(
                    "„{query}“ ist gesperrt, solange „{}“ offen ist. Erst den Dialog schließen.",
                    graph.regions[m].name.as_deref().unwrap_or("der Dialog")
                )
            }
            _ => format!("Nichts gefunden für „{query}“."),
        })),
        Resolution::Many(cs) => Err(Miss::Many(
            numbered(
                format!("Mehrdeutig, „{query}“ passt auf:"),
                cs.iter().map(|c| {
                    format!(
                        "{} in {}",
                        respond::control_line(c),
                        graph.region_label(c.region)
                    )
                }),
            ),
            cs.into_iter().cloned().collect(),
        )),
    }
}

/// Der Button im Cookie-Dialog, der ablehnt, ohne zu bezahlen (→
/// [`crate::overlay`]). Keiner: Das wird angesagt, nicht umgangen; Relief
/// wählt dann weder Zustimmen noch Einstellungen noch Abo.
fn reject_consent(graph: &Graph) -> Result<Control, Miss<Control>> {
    let Some(consent) = overlay::consent(graph) else {
        return Err(Miss::Text(
            "Kein Cookie-Dialog erkannt; nichts abgelehnt. „welcher Dialog ist offen“ zeigt, \
             was offen ist."
                .into(),
        ));
    };
    let mut rejects: Vec<&Control> = consent
        .of_kind(ButtonKind::Reject)
        .map(|i| &graph.controls[i])
        .collect();
    rejects.dedup_by_key(|c| c.dom_node_id);
    match rejects.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(Miss::Text(format!(
            "Nicht abgelehnt: {}",
            respond::overlay(graph, &consent)
        ))),
        many => Err(Miss::Many(
            numbered(
                "Mehrere Buttons lehnen vermutlich ab:".into(),
                many.iter().map(|c| respond::control_line(c)),
            ),
            many.iter().map(|c| (*c).clone()).collect(),
        )),
    }
}

fn pick_by_option(graph: &Graph, option: &str) -> Result<Control, Miss<Control>> {
    // Wie `resolve`: bei offenem modalem Dialog nur dessen Inhalt.
    let hits: Vec<&Control> = graph
        .reachable_controls()
        .filter(|c| c.options.iter().any(|o| o.eq_ignore_ascii_case(option)))
        .collect();
    match hits.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(Miss::Text(format!(
            "Kein Auswahlfeld mit Option „{option}“."
        ))),
        many => Err(Miss::Many(
            numbered(
                format!("Mehrere Auswahlfelder haben „{option}“:"),
                many.iter().map(|c| c.display_name()),
            ),
            many.iter().map(|c| (*c).clone()).collect(),
        )),
    }
}

fn is_editable(c: &Control) -> bool {
    matches!(
        c.role.as_str(),
        "textbox" | "searchbox" | "combobox" | "spinbutton"
    )
}

fn is_steppable(c: &Control) -> bool {
    matches!(c.role.as_str(), "slider" | "spinbutton")
}

/// Aufgabendatei (`spike/tasks/*.txt`) Zeile für Zeile: `url:`, `do:`,
/// `assert:` (Formular-Zusicherung), `expect:`; `#` und Leerzeilen fallen
/// weg, Unbekanntes auch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskLine {
    Url(String),
    Do(String),
    Assert(String),
    Expect(String),
}

pub fn parse_tasks(text: &str) -> Vec<TaskLine> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|line| {
            if let Some(url) = line.strip_prefix("url:") {
                Some(TaskLine::Url(url.trim().to_string()))
            } else if let Some(input) = line.strip_prefix("do:") {
                Some(TaskLine::Do(input.trim().to_string()))
            } else if let Some(text) = line.strip_prefix("assert:") {
                Some(TaskLine::Assert(text.trim().to_string()))
            } else {
                line.strip_prefix("expect:")
                    .map(|e| TaskLine::Expect(e.trim().to_string()))
            }
        })
        .collect()
}

/// Erfüllt die letzte Antwort die Erwartung? Teilstring ohne Groß- und
/// Kleinschreibung.
pub fn expectation_met(answer: &str, expected: &str) -> bool {
    answer.to_lowercase().contains(&expected.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aufgabenzeilen() {
        let lines = parse_tasks("# Kommentar\nurl: a.html\n\ndo: !klicke X\nexpect: Äpfel\nfoo");
        assert_eq!(
            lines,
            vec![
                TaskLine::Url("a.html".into()),
                TaskLine::Do("!klicke X".into()),
                TaskLine::Expect("Äpfel".into()),
            ]
        );
        assert!(expectation_met("Größe ÄPFEL gewählt", "äpfel"));
    }

    #[test]
    fn eingabe_mit_bestaetigung() {
        assert_eq!(
            parse_input(" !klicke Senden").unwrap(),
            (true, Command::Activate("Senden".into()))
        );
        assert!(!parse_input("klicke Senden").unwrap().0);
    }

    // Bestätigung (→ `crate::security`), auf der Testseite: „Bestellen“ ist
    // HIGH, „Menge“ ein Zahlenfeld (Ausfüllen MEDIUM).

    fn order() -> Command {
        Command::Activate("Bestellen".into())
    }

    /// Eingabe gegen `model`; Antworttext oder `None` bei einem Plan.
    fn run(
        session: &mut Session,
        model: &SemanticGraph,
        confirmed: bool,
        cmd: Command,
    ) -> Option<String> {
        match session.handle(&Graph::build(model), model, confirmed, cmd, None) {
            Outcome::Answer(text) => Some(text),
            Outcome::Perform { .. } => None,
            other => panic!("{other:?}"),
        }
    }

    fn decisions(session: &mut Session) -> Vec<Decision> {
        session
            .take_security_log()
            .into_iter()
            .map(|e| e.decision)
            .collect()
    }

    #[test]
    fn ausrufezeichen_ohne_rueckfrage_bestaetigt_nichts() {
        let model = crate::graph::sample_tree();
        let mut s = Session::new();
        let text = run(&mut s, &model, true, order()).unwrap();
        assert!(text.starts_with("Bestätigung nötig (High)"), "{text}");
        assert!(
            text.contains("Aktion: Activate, Ziel: [button] Bestellen"),
            "{text}"
        );
        assert!(text.contains("keine offene Rückfrage"), "{text}");
        assert_eq!(
            decisions(&mut s),
            vec![Decision::Reject, Decision::AskConfirmation]
        );
    }

    #[test]
    fn rueckfrage_gilt_genau_einmal() {
        let model = crate::graph::sample_tree();
        let mut s = Session::new();
        assert!(run(&mut s, &model, false, order()).is_some());
        assert_eq!(run(&mut s, &model, true, order()), None);
        let again = run(&mut s, &model, true, order()).unwrap();
        assert!(again.contains("keine offene Rückfrage"), "{again}");
    }

    #[test]
    fn andere_eingabe_verwirft_die_rueckfrage() {
        let model = crate::graph::sample_tree();
        let mut s = Session::new();
        assert!(run(&mut s, &model, false, order()).is_some());
        assert!(run(&mut s, &model, false, Command::Describe).is_some());
        let text = run(&mut s, &model, true, order()).unwrap();
        assert!(text.contains("keine offene Rückfrage"), "{text}");

        // Ablehnen („nein“ in der Befehlsleiste) verwirft sie ebenso.
        assert!(run(&mut s, &model, false, order()).is_some());
        s.discard_confirmation();
        let text = run(&mut s, &model, true, order()).unwrap();
        assert!(text.contains("keine offene Rückfrage"), "{text}");
    }

    #[test]
    fn abgelaufene_rueckfrage_gilt_nicht() {
        let model = crate::graph::sample_tree();
        let mut s = Session::with_confirmation_ttl(Duration::ZERO);
        assert!(run(&mut s, &model, false, order()).is_some());
        let text = run(&mut s, &model, true, order()).unwrap();
        assert!(text.contains("Rückfrage abgelaufen"), "{text}");
    }

    #[test]
    fn veraenderter_ausschnitt_verlangt_neue_bestaetigung() {
        let model = crate::graph::sample_tree();
        let mut s = Session::new();
        assert!(run(&mut s, &model, false, order()).is_some());

        // Nur die Version ist neu: Bestätigung gilt.
        let mut same = model.clone();
        same.version.0 += 1;
        assert_eq!(run(&mut s, &same, true, order()), None);

        // Zwischen Rückfrage und „!“ klappt der Button etwas auf.
        assert!(run(&mut s, &model, false, order()).is_some());
        let mut changed = model.clone();
        changed.version.0 += 1;
        let button = crate::graph::at(21);
        changed
            .trees
            .get_mut(&button.tree)
            .unwrap()
            .nodes
            .get_mut(&button.node)
            .unwrap()
            .states
            .expanded = Some(true);
        let text = run(&mut s, &changed, true, order()).unwrap();
        assert!(text.contains("Ziel oder Seite hat sich geändert"), "{text}");
        // Die neue Rückfrage gilt für den neuen Stand.
        assert_eq!(run(&mut s, &changed, true, order()), None);
    }

    /// Knoten der Testseite verändern.
    fn with_node(
        model: &SemanticGraph,
        id: i32,
        change: impl FnOnce(&mut relief_model::SemanticNode),
    ) -> SemanticGraph {
        let mut m = model.clone();
        let at = crate::graph::at(id);
        change(
            m.trees
                .get_mut(&at.tree)
                .unwrap()
                .nodes
                .get_mut(&at.node)
                .unwrap(),
        );
        m
    }

    #[test]
    fn rueckfrage_nennt_das_formularziel_und_bindet_es() {
        let target = |url: &str| {
            let url = url.to_string();
            with_node(&crate::graph::sample_tree(), 21, move |n| {
                n.extra.insert(crate::security::FORM_ACTION.into(), url);
            })
        };
        let shop = target("https://shop.example/bestellung?id=7");
        let mut s = Session::new();
        let text = run(&mut s, &shop, false, order()).unwrap();
        assert!(
            text.contains("Formularziel: https://shop.example/bestellung."),
            "{text}"
        );
        // Zwischen Rückfrage und „!“ zeigt das Formular woandershin.
        let evil = target("https://evil.example/bestellung");
        let text = run(&mut s, &evil, true, order()).unwrap();
        assert!(text.contains("andere Zieladresse"), "{text}");
        assert!(
            text.contains("Formularziel: https://evil.example/bestellung"),
            "{text}"
        );
        assert_eq!(run(&mut s, &evil, true, order()), None);
    }

    #[test]
    fn sensible_werte_stehen_nicht_in_der_rueckfrage() {
        // „Menge“ ohne sicheren Namen: Ausfüllen verlangt eine Rückfrage.
        let base = with_node(&crate::graph::sample_tree(), 20, |n| {
            n.name.certainty = relief_model::Certainty::Uncertain;
            n.value = relief_model::Fact::known(Some("alt-geheim".into()));
        });
        let fill = || Command::SetValue("Menge".into(), "geheim123".into());

        let mut s = Session::new();
        let plain = run(&mut s, &base, false, fill()).unwrap();
        assert!(plain.contains("SetValue(\"geheim123\")"), "{plain}");

        for (key, value) in [
            (crate::security::INPUT_TYPE, "password"),
            (crate::security::HTML_AUTOCOMPLETE, "cc-number"),
        ] {
            let model = with_node(&base, 20, |n| {
                n.extra.insert(key.into(), value.into());
            });
            let mut s = Session::new();
            let text = run(&mut s, &model, false, fill()).unwrap();
            assert!(text.starts_with("Bestätigung nötig"), "{text}");
            assert!(text.contains("SetValue(verdeckt)"), "{text}");
            for secret in ["geheim123", "alt-geheim"] {
                assert!(!text.contains(secret), "„{secret}“ in: {text}");
            }
            // Gebunden bleibt der Wert: „!“ mit anderem Wert gilt nicht.
            let other = Command::SetValue("Menge".into(), "anders".into());
            let refused = run(&mut s, &model, true, other).unwrap();
            assert!(refused.contains("anderer Wert"), "{refused}");
        }
    }

    // Overlay- und Consent-Dialoge (→ `crate::overlay`): Consent-iframe im
    // modalen Dialog, Buttons ab Knoten 20.

    fn target(outcome: Outcome) -> NodeRef {
        match outcome {
            Outcome::Perform { plan, .. } => plan.target,
            other => panic!("{other:?}"),
        }
    }

    fn frame_node(id: i32) -> NodeRef {
        NodeRef::new(
            relief_model::TreeId("frame".into()),
            relief_model::NodeId(id),
        )
    }

    #[test]
    fn cookies_ablehnen_nur_ueber_den_ablehnen_button() {
        let model = crate::overlay::consent_page(&["Alle akzeptieren", "Alle ablehnen"]);
        let g = Graph::build(&model);
        let mut s = Session::new();
        let out = s.handle(&g, &model, false, Command::RejectConsent, None);
        assert_eq!(target(out), frame_node(21));
    }

    #[test]
    fn ohne_ablehnen_wird_nichts_geklickt() {
        let model = crate::overlay::consent_page(&[
            "Einwilligen und weiter",
            "Jetzt abonnieren",
            "Einstellungen",
        ]);
        let mut s = Session::new();
        let text = run(&mut s, &model, false, Command::RejectConsent).unwrap();
        assert!(text.starts_with("Nicht abgelehnt:"), "{text}");
        assert!(text.contains("vermutlich Cookie-Dialog"), "{text}");
        assert!(text.contains("Kein Ablehnen ohne Bezahlung"), "{text}");
        assert!(s.take_security_log().is_empty());
    }

    #[test]
    fn schliessen_waehlt_keinen_zustimmen_button() {
        let model = crate::overlay::consent_page(&["Akzeptieren und schließen"]);
        let g = Graph::build(&model);
        let mut s = Session::new();
        let out = s.handle(&g, &model, false, Command::Dismiss, None);
        assert!(matches!(out, Outcome::Escape { .. }), "{out:?}");
        // Ein echter Schließen-Button bleibt einer.
        let model = crate::overlay::consent_page(&["Akzeptieren", "Schließen"]);
        let g = Graph::build(&model);
        let out = s.handle(&g, &model, false, Command::Dismiss, None);
        assert_eq!(target(out), frame_node(21));
    }

    #[test]
    fn hintergrund_ist_nur_auskunft() {
        let model = crate::overlay::consent_page(&["Alle ablehnen"]);
        let g = Graph::build(&model);
        let mut s = Session::new();
        let text = run(&mut s, &model, false, Command::Background).unwrap();
        assert!(text.contains("H2 „Nachrichten“"), "{text}");
        assert!(text.contains("[link] Zum Artikel"), "{text}");
        // Keine Auswahl entsteht: „1“ ist danach ein neuer Befehl.
        assert!(matches!(s.pending_reply(&g, &model, "1"), Pending::Command));
        let text = run(
            &mut s,
            &model,
            false,
            Command::Activate("Zum Artikel".into()),
        )
        .unwrap();
        assert!(text.contains("gesperrt"), "{text}");
    }

    #[test]
    fn security_log_ohne_werte_und_namen() {
        let model = crate::graph::sample_tree();
        let mut s = Session::new();
        let fill = Command::SetValue("Menge".into(), "geheim123".into());
        assert_eq!(run(&mut s, &model, false, fill), None);
        assert!(run(&mut s, &model, false, order()).is_some());
        assert_eq!(run(&mut s, &model, true, order()), None);
        assert!(run(&mut s, &model, false, Command::Activate("Menge".into())).is_some());

        let log = s.take_security_log();
        let json = serde_json::to_string(&log).unwrap();
        for secret in ["geheim123", "Menge", "Bestellen", "klicke"] {
            assert!(!json.contains(secret), "„{secret}“ im Log: {json}");
        }
        assert_eq!(
            json,
            r#"[{"decision":"perform","plan":1,"action":"set_value","risk":"Medium"},{"decision":"ask_confirmation","plan":2,"action":"activate","risk":"High"},{"decision":"perform_confirmed","plan":2,"action":"activate","risk":"High"},{"decision":"reject","action":"activate","reason":"invalid"}]"#
        );
    }
}
