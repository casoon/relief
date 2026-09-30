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

use relief_model::{NodeRef, SemanticGraph};

use crate::command::{self, parse, Command, ScrollDirection, Step};
use crate::graph::{focused, Control, Graph};
use crate::resolve::{
    current_place, dismissal, resolve, resolve_inflected, resolve_place, step_field, step_heading,
    Dismissal, Place, PlaceResolution, Resolution,
};
use crate::respond;
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

/// Eingabe ohne Seitenstand zerlegen: `!` am Anfang bestätigt riskante
/// Aktionen. Fehler: Antworttext (nicht verstanden).
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
#[derive(Debug, Clone, Default)]
pub struct Session {
    position: Option<Position>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wo Relief steht, gegeben den Fokus der Seite (→ Moduldoku).
    pub fn position(&self, focus: Option<&NodeRef>) -> Option<NodeRef> {
        match &self.position {
            Some(p) if p.focus.as_ref() == focus => Some(p.at.clone()),
            _ => focus.cloned(),
        }
    }

    /// Einen zerlegten Befehl gegen den aktuellen Stand beantworten oder in
    /// einen geprüften Plan übersetzen. `focus`: Fokus der Seite, nur nötig,
    /// wenn [`uses_focus`] zutrifft.
    pub fn handle(
        &self,
        graph: &Graph,
        model: &SemanticGraph,
        confirmed: bool,
        cmd: Command,
        focus: Option<&NodeRef>,
    ) -> Outcome {
        let here = self.position(focus);
        let (target, kind) = match cmd {
            Command::Help => return Outcome::Answer(command::HELP.into()),
            Command::Describe => return Outcome::Answer(respond::describe(graph)),
            Command::ListActions => return Outcome::Answer(respond::list_actions(graph)),
            Command::ListHeadings => return Outcome::Answer(respond::list_headings(graph)),
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
                        format!(
                            "Kein {} Formularfeld.",
                            match step {
                                Step::Next => "weiteres",
                                Step::Previous => "vorheriges",
                            }
                        )
                    });
                (field, ActionKind::Focus)
            }
            Command::WhereAmI => return Outcome::Answer(respond::where_am_i(graph, here.as_ref())),
            Command::Scroll(direction) => return Outcome::Scroll(direction),
            Command::SectionStep(step) => {
                return match step_heading(graph, here.as_ref(), step) {
                    Some(h) => navigate(graph, Place::Heading(h)),
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
                    Ok(place) => navigate(graph, place),
                    Err(msg) => Outcome::Answer(msg),
                }
            }
            Command::Read(q) => {
                let place = match q {
                    Some(q) => pick_place(graph, &q),
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
                    Err(msg) => msg,
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
        let control = match target {
            Ok(c) => c,
            Err(msg) => return Outcome::Answer(msg),
        };
        let label = respond::control_line(&control);

        let plan = match plan_on_page(&graph.page, &control, kind) {
            Ok(p) => p,
            Err(rejection) => return Outcome::Answer(format!("Abgelehnt: {rejection} ({label})")),
        };
        if plan.requires_confirmation && !confirmed {
            return Outcome::Answer(format!(
                "Bestätigung nötig ({:?}): {} — Ziel: {label}. Mit „!“ davor bestätigen.",
                plan.risk,
                plan.notes.join("; ")
            ));
        }
        Outcome::Perform { plan, label }
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

/// Zu einer Überschrift oder einem Bereich (LOW, ohne Rückfrage).
fn navigate(graph: &Graph, place: Place) -> Outcome {
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
        Ok(plan) => Outcome::Perform { plan, label },
        Err(rejection) => Outcome::Answer(format!("Abgelehnt: {rejection} ({label})")),
    }
}

fn pick_place(graph: &Graph, query: &str) -> Result<Place, String> {
    match resolve_place(graph, query) {
        PlaceResolution::One(place) => Ok(place),
        PlaceResolution::None => Err(format!(
            "Keine Überschrift und kein Bereich „{query}“ gefunden."
        )),
        PlaceResolution::Many(places) => Err(format!(
            "Mehrdeutig, „{query}“ passt auf:\n{}",
            places
                .iter()
                .map(|p| format!("  - {}", respond::place_label(graph, *p)))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

fn pick(graph: &Graph, query: &str, accept: impl Fn(&Control) -> bool) -> Result<Control, String> {
    pick_from(graph, query, resolve(graph, query, accept))
}

fn pick_from(graph: &Graph, query: &str, found: Resolution) -> Result<Control, String> {
    match found {
        Resolution::One(c) => Ok(c.clone()),
        Resolution::None => Err(match graph.active_modal() {
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
        }),
        Resolution::Many(cs) => Err(format!(
            "Mehrdeutig, „{query}“ passt auf:\n{}",
            cs.iter()
                .map(|c| format!(
                    "  - {} in {}",
                    respond::control_line(c),
                    graph.region_label(c.region)
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

fn pick_by_option(graph: &Graph, option: &str) -> Result<Control, String> {
    // Wie `resolve`: bei offenem modalem Dialog nur dessen Inhalt.
    let hits: Vec<&Control> = graph
        .reachable_controls()
        .filter(|c| c.options.iter().any(|o| o.eq_ignore_ascii_case(option)))
        .collect();
    match hits.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!("Kein Auswahlfeld mit Option „{option}“.")),
        many => Err(format!(
            "Mehrere Auswahlfelder haben „{option}“: {}",
            many.iter()
                .map(|c| c.display_name())
                .collect::<Vec<_>>()
                .join(", ")
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
}
