//! Formular-Zusicherungen: den Bedienablauf eines Formulars aus Sicht von
//! Screenreader- und Tastaturnutzenden prüfen (Linie B, →
//! `plan/spezifikation/12`).
//!
//! Browserfrei: Der Host übergibt das Modell ([`SemanticGraph`]), bei Bedarf
//! DOM-Fakten ([`DomFacts`]) und eine beobachtete Tab-Folge ([`Observed`]);
//! hier wird nur ausgewertet. Ergebnis sind Befunde im Format von
//! `a11y-report` ([`Finding`]). `Fail` heißt: aus den Daten belegt;
//! `Review` heißt: Heuristik oder Abweichung ohne sichere Ursache, ein Mensch
//! urteilt; `Untested`: die nötigen Daten fehlen.
//!
//! | Zusicherung (Aufgabenzeile `assert: …`) | Regel | Daten |
//! |---|---|---|
//! | `feldnamen` | `form/field-name` | Modell |
//! | `namen-wie-accname` | `form/name-accname` | Modell + DOM-Fakten |
//! | `fehler-verknüpft [Feld]` | `form/error-linked` | Modell |
//! | `fokus-auf-erstem-fehler` | `form/focus-first-error` | Modell |
//! | `bestätigungsdialog` | `form/confirm-dialog*` | Modell |
//! | `statusmeldung <Text>` | `form/status-message` | Modell + Modell vor dem letzten `do:` |
//! | `tabfolge <Feld>, <Feld>, …` | `form/tab-order` | beobachtete Tab-Folge |
//!
//! Zusicherungen prüfen den **aktuellen** Stand; der Ablauf davor (Absenden,
//! Dialog öffnen) steht als `do:`-Zeilen in der Aufgabendatei. Nur
//! `statusmeldung` vergleicht zusätzlich mit dem Stand vor der letzten
//! `do:`-Zeile ([`Observed::before`]).

use std::collections::HashMap;

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document};
use a11y_report::{Evidence, Finding, Location, Severity};
use accname::IdIndex;
use relief_model::{NodeRef, Role, SemanticGraph, SemanticNode, TreeDelta};

/// Eine Zusicherung aus der Aufgabendatei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assertion {
    /// Jedes exponierte Feld hat einen nichtleeren zugänglichen Namen.
    FieldNames,
    /// Chromiums Name gleich der Berechnung durch `accname` auf den
    /// DOM-Fakten; jede Abweichung ist ein eigener Befund.
    NamesMatchDom,
    /// Ungültige Felder (oder das genannte Feld) haben eine verknüpfte,
    /// wahrnehmbare Fehlermeldung.
    ErrorsLinked(Option<String>),
    /// Der Fokus liegt auf dem ersten ungültigen Feld.
    FocusFirstError,
    /// Der offene Dialog hat Namen, Text zum Bestätigten, Fokus und
    /// Abbruchweg.
    ConfirmDialog,
    /// Der Text steht vollständig in einer Live-Region, und die letzte
    /// Aktion hat diese Region geändert (nicht mitsamt Text neu eingefügt).
    StatusMessage(String),
    /// Tab erreicht die Felder in dieser Reihenfolge.
    TabOrder(Vec<String>),
}

/// Kurzhilfe für falsch geschriebene Zeilen.
pub const HELP: &str = "Zusicherungen: feldnamen · namen-wie-accname · fehler-verknüpft [Feld] · \
fokus-auf-erstem-fehler · bestätigungsdialog · statusmeldung <Text> · tabfolge <Feld>, <Feld>, …";

impl Assertion {
    /// Text nach `assert:` → Zusicherung.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        let rest = rest.trim();
        let arg = (!rest.is_empty()).then(|| rest.to_string());
        match word.to_lowercase().as_str() {
            "feldnamen" => Ok(Self::FieldNames),
            "namen-wie-accname" => Ok(Self::NamesMatchDom),
            "fehler-verknüpft" | "fehler-verknuepft" => Ok(Self::ErrorsLinked(arg)),
            "fokus-auf-erstem-fehler" => Ok(Self::FocusFirstError),
            "bestätigungsdialog" | "bestaetigungsdialog" => Ok(Self::ConfirmDialog),
            "statusmeldung" => arg
                .map(Self::StatusMessage)
                .ok_or_else(|| "statusmeldung braucht den erwarteten Text.".into()),
            "tabfolge" => {
                let fields: Vec<String> = rest
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                if fields.is_empty() {
                    Err("tabfolge braucht die erwarteten Felder, durch Komma getrennt.".into())
                } else {
                    Ok(Self::TabOrder(fields))
                }
            }
            _ => Err(format!("Unbekannte Zusicherung „{text}“. {HELP}")),
        }
    }

    /// Braucht die Prüfung DOM-Fakten vom Host?
    pub fn needs_dom(&self) -> bool {
        matches!(self, Self::NamesMatchDom)
    }

    /// Muss der Host die Tab-Folge beobachten?
    pub fn needs_tab_walk(&self) -> bool {
        matches!(self, Self::TabOrder(_))
    }
}

/// DOM-Fakten eines Dokuments in browserneutraler Form: der Baum als
/// `a11y-dom`-Arena mit Tag, Text und den Attributen, die die Prüfungen
/// brauchen ([`dom_attribute_needed`]), dazu die Zuordnung DOM-ID →
/// Arena-Knoten. Der Host erhebt sie; hier werden sie nur gelesen.
#[derive(Debug)]
pub struct DomFacts {
    arena: Arena,
    by_dom_node: HashMap<i64, a11y_dom::NodeId>,
}

impl DomFacts {
    /// Aufbau in Dokumentreihenfolge; das erste Element ist die Wurzel.
    pub fn builder() -> DomFactsBuilder {
        DomFactsBuilder {
            arena: Arena::builder(),
            next: 0,
            by_dom_node: HashMap::new(),
        }
    }

    pub fn document(&self) -> &Arena {
        &self.arena
    }

    /// Element zur DOM-ID des Modells ([`SemanticNode::dom_node_id`]).
    pub fn node(&self, dom_node_id: i64) -> Option<ArenaNode<'_>> {
        self.arena.get(*self.by_dom_node.get(&dom_node_id)?)
    }
}

/// Baut [`DomFacts`] auf wie [`ArenaBuilder`], merkt sich dabei die DOM-ID
/// jedes Elements.
pub struct DomFactsBuilder {
    arena: ArenaBuilder,
    /// Index des nächsten Arena-Knotens (die Arena zählt in Einfügefolge).
    next: u32,
    by_dom_node: HashMap<i64, a11y_dom::NodeId>,
}

impl DomFactsBuilder {
    /// Element öffnen; `dom_node_id` wie im Modell.
    pub fn open(mut self, local_name: &str, dom_node_id: i64) -> Self {
        self.by_dom_node
            .insert(dom_node_id, a11y_dom::NodeId(self.next));
        self.next += 1;
        self.arena = self.arena.open(local_name);
        self
    }

    /// Attribut am offenen Element; nur, was [`dom_attribute_needed`] nennt.
    pub fn attr(mut self, name: &str, value: &str) -> Self {
        self.arena = self.arena.attr(name, value);
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.next += 1;
        self.arena = self.arena.text(text);
        self
    }

    pub fn close(mut self) -> Self {
        self.arena = self.arena.close();
        self
    }

    pub fn build(self) -> DomFacts {
        DomFacts {
            arena: self.arena.build(),
            by_dom_node: self.by_dom_node,
        }
    }
}

/// Attribute, die die Namensberechnung und die Prüfungen brauchen. Der Host
/// überträgt nur diese (keine Formularinhalte außer `value`, das `accname`
/// für Buttons aus `<input>` braucht).
pub fn dom_attribute_needed(name: &str) -> bool {
    name.starts_with("aria-")
        || matches!(
            name,
            "id" | "for"
                | "role"
                | "tabindex"
                | "hidden"
                | "type"
                | "title"
                | "alt"
                | "placeholder"
                | "value"
        )
}

/// Was der Host für eine Prüfung übergibt.
pub struct Observed<'a> {
    pub model: &'a SemanticGraph,
    /// Nur für [`Assertion::needs_dom`].
    pub dom: Option<&'a DomFacts>,
    /// Aktueller Fokus. Ein Fokuswechsel ist keine DOM-Mutation; der Host
    /// fragt ihn deshalb live ab, statt ihn der letzten Aufnahme zu entnehmen.
    pub focus: Option<NodeRef>,
    /// Modell vor der letzten Aktion (`do:`-Zeile); nur für
    /// [`Assertion::StatusMessage`]. `None`: nur der Endzustand wird geprüft.
    pub before: Option<&'a SemanticGraph>,
    /// Nur für [`Assertion::needs_tab_walk`]: Fokus nach jedem Tab, in
    /// Reihenfolge; `None` = Fokus auf einem Element ohne Knoten im Modell.
    pub tab_sequence: Option<&'a [Option<NodeRef>]>,
}

/// Zusicherung auswerten. Leere Liste: keine Befunde.
pub fn check(assertion: &Assertion, observed: &Observed) -> Vec<Finding> {
    let model = observed.model;
    match assertion {
        Assertion::FieldNames => field_names(model),
        Assertion::NamesMatchDom => match observed.dom {
            Some(dom) => names_match_dom(model, dom),
            None => vec![Finding::untested(
                "form/name-accname",
                "Keine DOM-Fakten übergeben; Namen nicht mit accname verglichen.",
            )],
        },
        Assertion::ErrorsLinked(field) => errors_linked(model, field.as_deref()),
        Assertion::FocusFirstError => focus_first_error(model, observed.focus.as_ref()),
        Assertion::ConfirmDialog => confirm_dialog(model, observed.focus.as_ref()),
        Assertion::StatusMessage(text) => status_message(model, observed.before, text),
        Assertion::TabOrder(expected) => match observed.tab_sequence {
            Some(seq) => tab_order(model, expected, seq),
            None => vec![Finding::untested(
                "form/tab-order",
                "Keine beobachtete Tab-Folge übergeben.",
            )],
        },
    }
}

/// Befunde als Text für die Ausgabe des Hosts.
pub fn render(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "Keine Befunde.".into();
    }
    let mut out = format!(
        "{} {}:",
        findings.len(),
        if findings.len() == 1 {
            "Befund"
        } else {
            "Befunde"
        }
    );
    for f in findings {
        out.push_str(&format!(
            "\n- {} {} [{}]: {}",
            f.outcome.as_str(),
            f.rule_id,
            f.severity.as_str(),
            f.message
        ));
        if let Some(help) = &f.help {
            out.push_str(&format!(" — {help}"));
        }
    }
    out
}

/// Rollen, die als Formularfeld zählen.
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

/// Wahrnehmbare Felder in Dokumentreihenfolge.
fn fields(model: &SemanticGraph) -> Vec<(NodeRef, &SemanticNode)> {
    model
        .document_order()
        .into_iter()
        .filter_map(|at| {
            let node = model.node(&at)?;
            (!node.ignored && is_field(&node.role)).then_some((at, node))
        })
        .collect()
}

/// Leerraum zusammenfassen, Ränder weg.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn name_of(node: &SemanticNode) -> String {
    norm(node.name.value.as_deref().unwrap_or(""))
}

/// `textbox „E-Mail“` bzw. `textbox ohne Namen`.
fn label(node: &SemanticNode) -> String {
    match name_of(node) {
        n if n.is_empty() => format!("{} ohne Namen", node.role),
        n => format!("{} „{n}“", node.role),
    }
}

/// Befund am Element: Verortung über die DOM-ID (sonst den Modellknoten),
/// Rolle und Name dazu.
fn at_node(finding: Finding, at: &NodeRef, node: &SemanticNode) -> Finding {
    let location = match node.dom_node_id {
        Some(dom) => Location::node(dom.to_string()),
        None => Location::node(at.to_string()),
    };
    let name = Some(name_of(node)).filter(|n| !n.is_empty());
    finding
        .at(location)
        .with_element(Some(node.role.to_string()), name)
}

/// Liegt `at` in `ancestor` (oder ist es)? Nur innerhalb eines Baums.
fn within(model: &SemanticGraph, at: &NodeRef, ancestor: &NodeRef) -> bool {
    if at.tree != ancestor.tree {
        return false;
    }
    let Some(tree) = model.trees.get(&at.tree) else {
        return false;
    };
    let mut current = Some(at.node);
    while let Some(id) = current {
        if id == ancestor.node {
            return true;
        }
        current = tree.nodes.get(&id).and_then(|n| n.parent);
    }
    false
}

/// Wahrnehmbare Texte unter `at` (eingeschlossen), in Dokumentreihenfolge;
/// `skip` lässt Teilbäume aus.
fn texts(model: &SemanticGraph, at: &NodeRef, skip: &dyn Fn(&SemanticNode) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    let Some(tree) = model.trees.get(&at.tree) else {
        return out;
    };
    let mut stack = vec![at.node];
    while let Some(id) = stack.pop() {
        let Some(node) = tree.nodes.get(&id) else {
            continue;
        };
        if id != at.node && skip(node) {
            continue;
        }
        // Durch ignorierte Container (Chromium ignoriert z. B. schlichte
        // `<span>`) hindurch; wahrnehmbar ist, was als Text nicht ignoriert ist.
        if node.role == Role::StaticText && !node.ignored {
            let t = name_of(node);
            if !t.is_empty() {
                out.push(t);
            }
        }
        stack.extend(node.children.iter().rev());
    }
    out
}

fn field_names(model: &SemanticGraph) -> Vec<Finding> {
    fields(model)
        .into_iter()
        .filter(|(_, node)| name_of(node).is_empty())
        .map(|(at, node)| {
            at_node(
                Finding::fail(
                    "form/field-name",
                    format!("{} ohne zugänglichen Namen.", node.role),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1", "4.1.2"])
                .with_evidence([Evidence::ax_tree("name leer")])
                .with_help("<label for>, aria-label oder aria-labelledby setzen."),
                &at,
                node,
            )
        })
        .collect()
}

fn names_match_dom(model: &SemanticGraph, dom: &DomFacts) -> Vec<Finding> {
    let doc = dom.document();
    let ids = IdIndex::build(doc.root());
    let mut out = Vec::new();
    for (at, node) in fields(model) {
        let Some(element) = node.dom_node_id.and_then(|d| dom.node(d)) else {
            out.push(at_node(
                Finding::untested(
                    "form/name-accname",
                    format!("{}: kein DOM-Knoten in den DOM-Fakten.", label(node)),
                ),
                &at,
                node,
            ));
            continue;
        };
        let chromium = name_of(node);
        let computed = norm(&accname::name(element, &ids).unwrap_or_default());
        if chromium == computed {
            continue;
        }
        let why = if !chromium.is_empty() && computed.contains(&chromium) {
            "accname zählt Inhalt mit, den Chromium auslässt — meist per CSS verborgen \
             (die DOM-Fakten tragen kein Rendering, display: none sieht accname nicht)."
        } else if chromium.is_empty() {
            "Chromium findet keinen Namen, die DOM-Berechnung schon; Markup prüfen."
        } else if computed.is_empty() {
            "Chromium bildet einen Namen, den die DOM-Berechnung nicht findet \
             (z. B. aus CSS-Inhalt oder Shadow DOM)."
        } else {
            "Ursache aus den Daten nicht bestimmbar; Markup prüfen."
        };
        out.push(at_node(
            Finding::review(
                "form/name-accname",
                format!("Name laut Chromium „{chromium}“, laut accname „{computed}“."),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["4.1.2"])
            .with_evidence([
                Evidence::ax_tree(chromium),
                Evidence::computed("accname", computed),
            ])
            .with_help(why),
            &at,
            node,
        ));
    }
    out
}

fn invalid_fields(model: &SemanticGraph) -> Vec<(NodeRef, &SemanticNode)> {
    fields(model)
        .into_iter()
        .filter(|(_, n)| n.states.invalid.is_some())
        .collect()
}

fn no_invalid_field(rule: &str) -> Finding {
    Finding::review(
        rule,
        "Kein Feld ist als ungültig gekennzeichnet (aria-invalid); ein Fehler ist für \
         Screenreader nicht erkennbar oder das Absenden war erfolgreich.",
    )
    .with_severity(Severity::High)
    .with_wcag(["3.3.1"])
}

fn errors_linked(model: &SemanticGraph, field: Option<&str>) -> Vec<Finding> {
    const RULE: &str = "form/error-linked";
    let targets: Vec<(NodeRef, &SemanticNode)> = match field {
        Some(wanted) => {
            let found: Vec<_> = fields(model)
                .into_iter()
                .filter(|(_, n)| name_of(n).eq_ignore_ascii_case(&norm(wanted)))
                .collect();
            if found.is_empty() {
                return vec![
                    Finding::fail(RULE, format!("Kein Feld „{wanted}“ gefunden."))
                        .with_severity(Severity::High),
                ];
            }
            let mut out = Vec::new();
            let mut invalid = Vec::new();
            for (at, node) in found {
                if node.states.invalid.is_some() {
                    invalid.push((at, node));
                } else {
                    out.push(at_node(
                        Finding::fail(
                            RULE,
                            format!("{} ist nicht als ungültig gekennzeichnet.", label(node)),
                        )
                        .with_severity(Severity::High)
                        .with_wcag(["3.3.1"])
                        .with_help("aria-invalid=\"true\" setzen, solange der Fehler besteht."),
                        &at,
                        node,
                    ));
                }
            }
            if !out.is_empty() {
                return out;
            }
            invalid
        }
        None => {
            let invalid = invalid_fields(model);
            if invalid.is_empty() {
                return vec![no_invalid_field(RULE)];
            }
            invalid
        }
    };

    let mut out = Vec::new();
    for (at, node) in targets {
        let links: Vec<NodeRef> = node
            .relations
            .error_message
            .iter()
            .chain(&node.relations.described_by)
            .map(|id| NodeRef::new(at.tree.clone(), *id))
            .collect();
        let messages: Vec<String> = links
            .iter()
            .map(|l| texts(model, l, &|_| false).join(" "))
            .filter(|t| t.chars().any(char::is_alphanumeric))
            .collect();
        if !messages.is_empty() {
            continue;
        }
        let message = if links.is_empty() {
            format!(
                "{} ist ungültig, aber keine Fehlermeldung ist verknüpft.",
                label(node)
            )
        } else {
            format!(
                "{} ist ungültig; die verknüpfte Fehlermeldung ist leer oder nicht wahrnehmbar.",
                label(node)
            )
        };
        out.push(at_node(
            Finding::fail(RULE, message)
                .with_severity(Severity::High)
                .with_wcag(["1.3.1", "3.3.1"])
                .with_evidence([Evidence::ax_tree(format!(
                    "errormessage/describedby: {} Ziel(e)",
                    links.len()
                ))])
                .with_help("Fehlertext über aria-describedby oder aria-errormessage verknüpfen."),
            &at,
            node,
        ));
    }
    out
}

fn focus_first_error(model: &SemanticGraph, focus: Option<&NodeRef>) -> Vec<Finding> {
    const RULE: &str = "form/focus-first-error";
    let invalid = invalid_fields(model);
    let Some((first, first_node)) = invalid.first() else {
        return vec![no_invalid_field(RULE)];
    };
    if focus == Some(first) {
        return Vec::new();
    }
    let now = focus
        .and_then(|at| model.node(at))
        .map(label)
        .unwrap_or_else(|| "keinem Element".into());
    vec![at_node(
        Finding::fail(
            RULE,
            format!("Fokus auf {now}, erster Fehler ist {}.", label(first_node)),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["2.4.3", "3.3.1"])
        .with_help("Nach fehlerhaftem Absenden das erste ungültige Feld fokussieren."),
        first,
        first_node,
    )]
}

/// Wörter, an denen ein Abbrechen-Button erkannt wird.
const CANCEL_WORDS: &[&str] = &[
    "abbrechen",
    "schließen",
    "nein",
    "zurück",
    "cancel",
    "close",
    "no",
];

fn confirm_dialog(model: &SemanticGraph, focus: Option<&NodeRef>) -> Vec<Finding> {
    let dialogs: Vec<(NodeRef, &SemanticNode)> = model
        .document_order()
        .into_iter()
        .filter_map(|at| {
            let n = model.node(&at)?;
            (!n.ignored && matches!(n.role, Role::Dialog | Role::AlertDialog)).then_some((at, n))
        })
        .collect();
    let chosen = dialogs
        .iter()
        .find(|(at, _)| focus.is_some_and(|f| within(model, f, at)))
        .or(dialogs.last());
    let Some((at, dialog)) = chosen else {
        return vec![Finding::fail(
            "form/confirm-dialog",
            "Kein Dialog wahrnehmbar (Rolle dialog oder alertdialog).",
        )
        .with_severity(Severity::High)
        .with_wcag(["4.1.2"])];
    };
    let mut out = Vec::new();
    let name = name_of(dialog);
    if name.is_empty() {
        out.push(at_node(
            Finding::fail(
                "form/confirm-dialog-name",
                "Dialog ohne zugänglichen Namen.",
            )
            .with_severity(Severity::High)
            .with_wcag(["4.1.2"])
            .with_help("aria-labelledby auf die Überschrift des Dialogs setzen."),
            at,
            dialog,
        ));
    }
    // Text außer Buttons und dem Namen: nennt der Dialog, was bestätigt wird?
    // Ob der Text verständlich ist, urteilt ein Mensch (Review).
    let rest: Vec<String> = texts(model, at, &|n| matches!(n.role, Role::Button | Role::Link))
        .into_iter()
        .filter(|t| *t != name && t.chars().any(char::is_alphanumeric))
        .collect();
    let description = norm(dialog.description.value.as_deref().unwrap_or(""));
    if rest.is_empty() && description.is_empty() {
        out.push(at_node(
            Finding::review(
                "form/confirm-dialog-text",
                "Dialog nennt außer Name und Buttons nicht, was bestätigt wird.",
            )
            .with_severity(Severity::Medium)
            .with_tags(["best-practice"])
            .with_help("Im Dialog sagen, was mit welchen Angaben geschieht."),
            at,
            dialog,
        ));
    }
    if !focus.is_some_and(|f| within(model, f, at)) {
        let now = focus
            .and_then(|f| model.node(f))
            .map(label)
            .unwrap_or_else(|| "keinem Element".into());
        out.push(at_node(
            Finding::fail(
                "form/confirm-dialog-focus",
                format!("Fokus liegt nicht im Dialog, sondern auf {now}."),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.3"])
            .with_help("Beim Öffnen den Fokus in den Dialog setzen (showModal() tut das)."),
            at,
            dialog,
        ));
    }
    let buttons: Vec<String> = model
        .trees
        .get(&at.tree)
        .map(|t| {
            t.nodes
                .iter()
                .filter(|(id, n)| {
                    !n.ignored
                        && n.role == Role::Button
                        && within(model, &NodeRef::new(at.tree.clone(), **id), at)
                })
                .map(|(_, n)| name_of(n).to_lowercase())
                .collect()
        })
        .unwrap_or_default();
    let cancel = buttons.iter().any(|b| {
        b.split(|c: char| !c.is_alphanumeric())
            .any(|w| CANCEL_WORDS.contains(&w))
    });
    if !cancel {
        out.push(at_node(
            Finding::review(
                "form/confirm-dialog-cancel",
                format!(
                    "Kein Abbrechen-Button erkannt (Buttons: {}); Escape nicht geprüft.",
                    if buttons.is_empty() {
                        "keine".into()
                    } else {
                        buttons.join(", ")
                    }
                ),
            )
            .with_severity(Severity::Medium)
            .with_tags(["best-practice"])
            .with_help("Einen Button „Abbrechen“ anbieten."),
            at,
            dialog,
        ));
    }
    out
}

/// Live-Region: Rolle mit implizitem `aria-live` oder ausdrücklich
/// `aria-live` polite/assertive.
fn is_live(node: &SemanticNode) -> bool {
    matches!(
        node.role,
        Role::Status | Role::Alert | Role::Log | Role::Timer | Role::Marquee
    ) || matches!(
        node.extra.get("live").map(String::as_str),
        Some("polite" | "assertive")
    )
}

fn status_message(
    model: &SemanticGraph,
    before: Option<&SemanticGraph>,
    expected: &str,
) -> Vec<Finding> {
    const RULE: &str = "form/status-message";
    let wanted = norm(expected).to_lowercase();
    let order = model.document_order();
    let region = order.iter().find(|at| {
        model.node(at).is_some_and(|n| !n.ignored && is_live(n))
            && texts(model, at, &|_| false)
                .join(" ")
                .to_lowercase()
                .contains(&wanted)
    });
    if let Some(region) = region {
        return match before {
            Some(before) => status_changed(before, model, region, expected),
            None => Vec::new(),
        };
    }
    // Irgendwo auf der Seite, aber nicht in einer Live-Region?
    let page = order
        .iter()
        .filter_map(|at| model.node(at))
        .filter(|n| !n.ignored && n.role == Role::StaticText)
        .map(name_of)
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let message = if page.contains(&wanted) {
        format!(
            "„{expected}“ steht auf der Seite, aber in keiner Live-Region; Screenreader sagen es \
             nicht an."
        )
    } else {
        format!("„{expected}“ ist nicht wahrnehmbar.")
    };
    vec![Finding::fail(RULE, message)
        .with_severity(Severity::High)
        .with_wcag(["4.1.3"])
        .with_help("Ergebnis als einen Text in role=\"status\" (oder aria-live) ausgeben.")]
}

/// Hat die letzte Aktion die Live-Region `region` geändert? Screenreader
/// sagen Änderungen an einer vorhandenen Region an; eine mitsamt Text neu
/// eingefügte Region oft nicht. Grundlage ist die Delta des Modells.
fn status_changed(
    before: &SemanticGraph,
    model: &SemanticGraph,
    region: &NodeRef,
    expected: &str,
) -> Vec<Finding> {
    const RULE: &str = "form/status-message";
    let delta = TreeDelta::between(before, model);
    let update = delta.trees.iter().find(|u| u.tree == region.tree);
    let created = |id| update.is_some_and(|u| u.created.iter().any(|n| n.id == id));
    // Neu eingefügt nur, wenn der Elternknoten schon vorher wahrnehmbar war.
    // Ist er es auch nicht (Seite hinter einem modalen Dialog, der jetzt zu
    // ist; Navigation), trennt das Modell „eingefügt“ nicht von „wieder
    // wahrnehmbar“: dann kein Befund.
    let parent = model.node(region).and_then(|n| n.parent);
    if created(region.node) && parent.is_some_and(|p| !created(p)) {
        return vec![Finding::fail(
            RULE,
            format!(
                "„{expected}“ steht in einer Live-Region, die die letzte Aktion mitsamt Text \
                 neu eingefügt hat; Screenreader sagen sie oft nicht an."
            ),
        )
        .with_severity(Severity::High)
        .with_wcag(["4.1.3"])
        .with_evidence([Evidence::ax_tree("Live-Region neu angelegt")])
        .with_help("Die Live-Region leer mit der Seite ausliefern und nur ihren Text ändern.")];
    }
    let touched = update.is_some_and(|u| {
        u.created
            .iter()
            .chain(&u.changed)
            .any(|n| within(model, &NodeRef::new(region.tree.clone(), n.id), region))
    });
    if touched {
        return Vec::new();
    }
    vec![Finding::fail(
        RULE,
        format!(
            "„{expected}“ stand schon vor der letzten Aktion in der Live-Region; die Aktion hat \
             sie nicht geändert, Screenreader sagen nichts an."
        ),
    )
    .with_severity(Severity::Medium)
    .with_wcag(["4.1.3"])
    .with_evidence([Evidence::ax_tree("Live-Region unverändert")])
    .with_help("Die Meldung als Änderung der Live-Region ausgeben.")]
}

fn tab_order(model: &SemanticGraph, expected: &[String], seq: &[Option<NodeRef>]) -> Vec<Finding> {
    const RULE: &str = "form/tab-order";
    let mut observed: Vec<String> = Vec::new();
    let mut last: Option<&Option<NodeRef>> = None;
    for step in seq {
        if last == Some(step) {
            continue;
        }
        last = Some(step);
        observed.push(
            step.as_ref()
                .and_then(|at| model.node(at))
                .map(name_of)
                .unwrap_or_else(|| "?".into()),
        );
    }
    let evidence = Evidence::computed("tab-folge", observed.join(" → "));
    let matches = |o: &String, e: &String| o.eq_ignore_ascii_case(&norm(e));
    let mut out = Vec::new();
    let mut pos = 0;
    let mut previous: Option<&String> = None;
    for wanted in expected {
        if let Some(i) = observed[pos..].iter().position(|o| matches(o, wanted)) {
            pos += i + 1;
            previous = Some(wanted);
            continue;
        }
        let finding = if observed[..pos].iter().any(|o| matches(o, wanted)) {
            Finding::fail(
                RULE,
                format!(
                    "„{wanted}“ wird per Tab erreicht, aber vor „{}“.",
                    previous.map(String::as_str).unwrap_or("")
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.4.3"])
        } else {
            Finding::fail(RULE, format!("„{wanted}“ wird per Tab nicht erreicht."))
                .with_severity(Severity::High)
                .with_wcag(["2.1.1"])
        };
        out.push(finding.with_evidence([evidence.clone()]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use relief_model::{Invalid, NodeId, SemanticTree, TreeId};

    /// Ein Baum `t` aus (ID, Rolle, Name, Kinder); DOM-ID = Knoten-ID.
    fn model(nodes: &[(i32, Role, &str, &[i32])]) -> SemanticGraph {
        let tree_id = TreeId("t".into());
        let mut tree = SemanticTree::new(tree_id.clone());
        tree.root = Some(NodeId(nodes[0].0));
        for (id, role, name, children) in nodes {
            let mut n = SemanticNode::new(NodeId(*id), role.clone());
            n.name.value = (!name.is_empty()).then(|| name.to_string());
            n.children = children.iter().map(|c| NodeId(*c)).collect();
            n.dom_node_id = Some(*id as i64);
            tree.nodes.insert(NodeId(*id), n);
        }
        let parents: Vec<(NodeId, NodeId)> = tree
            .nodes
            .values()
            .flat_map(|n| n.children.iter().map(move |c| (*c, n.id)))
            .collect();
        for (child, parent) in parents {
            tree.nodes.get_mut(&child).unwrap().parent = Some(parent);
        }
        SemanticGraph {
            root: Some(tree_id.clone()),
            trees: [(tree_id, tree)].into(),
            ..Default::default()
        }
    }

    fn node(g: &mut SemanticGraph, id: i32) -> &mut SemanticNode {
        g.trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .get_mut(&NodeId(id))
            .unwrap()
    }

    fn at(id: i32) -> NodeRef {
        NodeRef::new(TreeId("t".into()), NodeId(id))
    }

    fn observed(g: &SemanticGraph) -> Observed<'_> {
        Observed {
            model: g,
            dom: None,
            focus: None,
            before: None,
            tab_sequence: None,
        }
    }

    fn form() -> SemanticGraph {
        model(&[
            (1, Role::RootWebArea, "Seite", &[2]),
            (2, Role::Form, "", &[3, 4, 5, 7]),
            (3, Role::Textbox, "Name", &[]),
            (4, Role::Textbox, "E-Mail", &[]),
            (5, Role::Generic, "", &[6]),
            (6, Role::StaticText, "Bitte E-Mail angeben.", &[]),
            (7, Role::Button, "Senden", &[]),
        ])
    }

    #[test]
    fn zeilen_werden_zu_zusicherungen() {
        assert_eq!(Assertion::parse("feldnamen"), Ok(Assertion::FieldNames));
        assert_eq!(
            Assertion::parse("fehler-verknüpft E-Mail"),
            Ok(Assertion::ErrorsLinked(Some("E-Mail".into())))
        );
        assert_eq!(
            Assertion::parse("tabfolge Name, E-Mail ,Senden"),
            Ok(Assertion::TabOrder(vec![
                "Name".into(),
                "E-Mail".into(),
                "Senden".into()
            ]))
        );
        assert!(Assertion::parse("statusmeldung").is_err());
        assert!(Assertion::parse("irgendwas").is_err());
    }

    #[test]
    fn feld_ohne_namen_ist_ein_befund() {
        let mut g = form();
        assert!(check(&Assertion::FieldNames, &observed(&g)).is_empty());
        node(&mut g, 3).name.value = None;
        let f = check(&Assertion::FieldNames, &observed(&g));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "form/field-name");
        assert_eq!(f[0].location.node.as_deref(), Some("3"));
    }

    #[test]
    fn fehlermeldung_muss_verknuepft_und_wahrnehmbar_sein() {
        let mut g = form();
        node(&mut g, 4).states.invalid = Some(Invalid::True);
        let a = Assertion::ErrorsLinked(Some("E-Mail".into()));
        assert_eq!(check(&a, &observed(&g))[0].rule_id, "form/error-linked");

        node(&mut g, 4).relations.described_by = vec![NodeId(5)];
        assert!(check(&a, &observed(&g)).is_empty());

        // Die Meldung selbst ist verborgen (Chromium: Text ignoriert).
        node(&mut g, 6).ignored = true;
        let f = check(&a, &observed(&g));
        assert!(f[0].message.contains("nicht wahrnehmbar"), "{f:?}");
    }

    #[test]
    fn fokus_muss_auf_dem_ersten_fehler_stehen() {
        let mut g = form();
        node(&mut g, 3).states.invalid = Some(Invalid::True);
        node(&mut g, 4).states.invalid = Some(Invalid::True);
        let on = |id| Observed {
            focus: Some(at(id)),
            ..observed(&g)
        };
        let f = check(&Assertion::FocusFirstError, &on(4));
        assert!(
            f[0].message.contains("erster Fehler ist textbox „Name“"),
            "{f:?}"
        );
        assert!(check(&Assertion::FocusFirstError, &on(3)).is_empty());
    }

    #[test]
    fn statusmeldung_nur_in_live_region() {
        let mut g = form();
        let a = Assertion::StatusMessage("bitte e-mail angeben".into());
        assert!(check(&a, &observed(&g))[0]
            .message
            .contains("keiner Live-Region"));
        node(&mut g, 5).role = Role::Status;
        assert!(check(&a, &observed(&g)).is_empty());
    }

    #[test]
    fn statusmeldung_muss_eine_aenderung_sein() {
        let a = Assertion::StatusMessage("bitte e-mail angeben".into());
        let mut after = form();
        node(&mut after, 5).role = Role::Status;
        let with_before = |before: &SemanticGraph| {
            check(
                &a,
                &Observed {
                    before: Some(before),
                    ..observed(&after)
                },
            )
        };

        // Region vorher leer, jetzt mit Text: Änderung, kein Befund.
        let mut before = after.clone();
        node(&mut before, 5).children.clear();
        before
            .trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .remove(&NodeId(6));
        assert!(with_before(&before).is_empty());

        // Region samt Text neu eingefügt.
        let mut before = after.clone();
        node(&mut before, 2).children.retain(|c| *c != NodeId(5));
        let nodes = &mut before.trees.values_mut().next().unwrap().nodes;
        nodes.remove(&NodeId(5));
        nodes.remove(&NodeId(6));
        let f = with_before(&before);
        assert_eq!(f[0].rule_id, "form/status-message");
        assert!(f[0].message.contains("neu eingefügt"), "{f:?}");

        // Mitsamt Elternknoten neu wahrnehmbar (Dialog geschlossen): kein
        // Befund, das Modell trennt es nicht vom Einfügen.
        let mut before = after.clone();
        node(&mut before, 1).children.clear();
        let nodes = &mut before.trees.values_mut().next().unwrap().nodes;
        nodes.retain(|id, _| *id == NodeId(1));
        assert!(with_before(&before).is_empty());

        // Text stand schon vorher darin.
        let f = with_before(&after.clone());
        assert!(f[0].message.contains("nicht geändert"), "{f:?}");
    }

    #[test]
    fn tabfolge_meldet_fehlende_und_vertauschte_felder() {
        let g = form();
        let seq = [Some(at(4)), Some(at(3)), Some(at(7))];
        let o = Observed {
            tab_sequence: Some(&seq),
            ..observed(&g)
        };
        let a = Assertion::TabOrder(vec!["Name".into(), "E-Mail".into(), "Senden".into()]);
        let f = check(&a, &o);
        assert_eq!(f.len(), 1);
        assert!(f[0]
            .message
            .contains("„E-Mail“ wird per Tab erreicht, aber vor"));

        let seq = [Some(at(3)), Some(at(7))];
        let o = Observed {
            tab_sequence: Some(&seq),
            ..observed(&g)
        };
        let f = check(&a, &o);
        assert_eq!(f[0].message, "„E-Mail“ wird per Tab nicht erreicht.");
    }

    #[test]
    fn abweichung_zu_accname_ist_ein_eigener_befund() {
        let g = form();
        // <form><label for=name>Name <span>intern</span></label><input id=name>
        //       <label for=mail>E-Mail</label><input id=mail>…
        let dom = DomFacts::builder()
            .open("form", 2)
            .open("label", 10)
            .attr("for", "name")
            .text("Name ")
            .open("span", 11)
            .text("intern")
            .close()
            .close()
            .open("input", 3)
            .attr("id", "name")
            .close()
            .open("label", 12)
            .attr("for", "mail")
            .text("E-Mail")
            .close()
            .open("input", 4)
            .attr("id", "mail")
            .close()
            .close()
            .build();
        let o = Observed {
            dom: Some(&dom),
            ..observed(&g)
        };
        let f = check(&Assertion::NamesMatchDom, &o);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].rule_id, "form/name-accname");
        assert!(f[0].message.contains("laut accname „Name intern“"));
        assert!(f[0].help.as_deref().unwrap().contains("CSS"));
    }
}
