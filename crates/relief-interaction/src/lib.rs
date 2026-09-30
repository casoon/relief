//! Relief: Interaction Graph über dem semantischen Modell.
//!
//! Browserfrei: nimmt einen [`relief_model::SemanticGraph`] entgegen und
//! beantwortet, was ein Mensch auf der Seite tun kann. Ziele sind
//! [`relief_model::NodeRef`]s; die DOM-ID steht daneben für Hosts, die über
//! das DOM handeln (CDP). Aktionen werden hier nur *geplant und validiert* —
//! ausgeführt werden sie vom Host.
//!
//! | Modul | Aufgabe |
//! |---|---|
//! | [`graph`] | Bereiche, Überschriften, Bedienelemente mit Herkunft ihres Namens |
//! | [`page`] | Seitentyp, funktionale Gruppen, primäre Aktion (erschlossen) |
//! | [`command`] | Texteingabe → [`command::Command`] (deterministisch, kein LLM) |
//! | [`resolve`] | Zielbeschreibung → Bedienelement, Mehrdeutigkeit wird gemeldet |
//! | [`validate`] | Bedienelement + Aktion → [`validate::ActionPlan`] mit Risikoklasse |
//! | [`respond`] | Antworttexte für Abfragen und Aktionsergebnisse |
//! | [`session`] | Eingabe → Antwort oder auszuführender Plan; Position; Rückfragen; Aufgabendateien |
//! | [`security`] | Bestätigungstoken, Security-Log, Grenzen (Namen) |
//! | `assertions` | Formular-Zusicherungen → Befunde (`a11y-report`); nur mit Feature `assertions` |

#![forbid(unsafe_code)]

// Braucht barrierlab-Crates, die der Fork-Build nicht hat.
#[cfg(feature = "assertions")]
pub mod assertions;
pub mod command;
pub mod graph;
pub mod page;
pub mod resolve;
pub mod respond;
pub mod security;
pub mod session;
pub mod validate;

pub use command::{parse, Command, ScrollDirection, Step};
pub use graph::{focused, Anchor, Control, Graph, Heading, Region, Text};
pub use page::{Group, GroupKind, Page, PageType};
pub use resolve::{
    current_place, dismissal, resolve, resolve_inflected, resolve_place, step_field, step_heading,
    Dismissal, Place, PlaceResolution, Resolution,
};
pub use security::{Decision, Limit, PlanId, Reason, SecurityEvent};
pub use session::{
    expectation_met, parse_input, parse_tasks, uses_focus, Outcome, Session, TaskLine,
};
pub use validate::{plan, plan_navigation, plan_on_page, ActionKind, ActionPlan, Rejection, Risk};
