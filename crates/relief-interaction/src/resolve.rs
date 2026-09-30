//! Zielauflösung: „Warenkorb“ → Bedienelement.
//!
//! Mehrdeutigkeit wird gemeldet, nie durch Raten aufgelöst
//! (→ `plan/spezifikation/05`).

use crate::command::Step;
use crate::graph::{non_empty, Anchor, Control, Graph};
use relief_model::{NodeRef, Role, SemanticGraph};

#[derive(Debug)]
pub enum Resolution<'g> {
    One(&'g Control),
    Many(Vec<&'g Control>),
    None,
}

/// Rollenwörter, über die ein Ziel auch ohne Namen gefunden wird.
const ROLE_WORDS: &[(&str, &[Role])] = &[
    ("suche", &[Role::SearchBox]),
    ("search", &[Role::SearchBox]),
    ("suchfeld", &[Role::SearchBox, Role::Textbox]),
    ("menü", &[Role::Button, Role::MenuItem]),
    // Unbenannte Elemente sind nur über ihre Rolle erreichbar; bei mehreren
    // Treffern fragt die Auflösung nach.
    ("button", &[Role::Button]),
    ("schaltfläche", &[Role::Button]),
    ("link", &[Role::Link]),
];

pub fn resolve<'g>(
    graph: &'g Graph,
    query: &str,
    accept: impl Fn(&Control) -> bool,
) -> Resolution<'g> {
    let q = normalize(query);
    let mut best = 0;
    let mut hits: Vec<&Control> = Vec::new();

    for control in graph.reachable_controls().filter(|c| accept(c)) {
        let score = score(control, &q);
        if score == 0 {
            continue;
        }
        if score > best {
            best = score;
            hits.clear();
        }
        if score == best {
            hits.push(control);
        }
    }

    // Gleicher Name, gleiche Rolle, gleiches Ziel im DOM ist kein echter
    // Konflikt (z. B. doppelt gemeldeter Knoten).
    hits.dedup_by_key(|c| c.dom_node_id);

    match hits.len() {
        0 => Resolution::None,
        1 => Resolution::One(hits[0]),
        _ => Resolution::Many(hits),
    }
}

/// Wie `resolve`, versucht bei keinem Treffer aber gebeugte Formen:
/// „Größen“ → „Größe“, „sizes“ → „size“ (für „welche Größen gibt es?“).
pub fn resolve_inflected<'g>(
    graph: &'g Graph,
    query: &str,
    accept: impl Fn(&Control) -> bool,
) -> Resolution<'g> {
    let first = resolve(graph, query, &accept);
    if !matches!(first, Resolution::None) {
        return first;
    }
    let lower = query.to_lowercase();
    for suffix in ["en", "n", "s", "e"] {
        if lower.ends_with(suffix) && lower.chars().count() > suffix.len() + 2 {
            let found = resolve(graph, &query[..query.len() - suffix.len()], &accept);
            if !matches!(found, Resolution::None) {
                return found;
            }
        }
    }
    Resolution::None
}

/// Ziel einer Navigation oder eines Vorlesebefehls: ein Abschnitt unter
/// einer Überschrift oder ein Bereich (Landmark, Dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Index in [`Graph::headings`].
    Heading(usize),
    /// Index in [`Graph::regions`].
    Region(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlaceResolution {
    One(Place),
    Many(Vec<Place>),
    None,
}

/// Wörter für Bereiche ohne Namen.
const REGION_WORDS: &[(&str, Role)] = &[
    ("hauptinhalt", Role::Main),
    ("inhalt", Role::Main),
    ("main", Role::Main),
    ("navigation", Role::Navigation),
    ("kopfbereich", Role::Banner),
    ("kopfzeile", Role::Banner),
    ("header", Role::Banner),
    ("fußzeile", Role::ContentInfo),
    ("fußbereich", Role::ContentInfo),
    ("footer", Role::ContentInfo),
    ("seitenleiste", Role::Complementary),
    ("sidebar", Role::Complementary),
    ("suche", Role::Search),
    ("search", Role::Search),
    ("dialog", Role::Dialog),
];

/// Überschrift oder Bereich nach Namen. Trägt ein Bereich denselben Namen
/// wie eine Überschrift in ihm (`<section aria-labelledby>`), zählt die
/// Überschrift — dort landet die Navigation, und ihr Abschnitt ist der
/// Inhalt. Bei offenem modalem Dialog nur dessen Inhalt.
pub fn resolve_place(graph: &Graph, query: &str) -> PlaceResolution {
    let q = normalize(query);
    let mut scored: Vec<(u8, Place)> = Vec::new();
    for (i, h) in graph.headings.iter().enumerate() {
        if graph.is_reachable(h.region, &h.node) {
            scored.push((name_score(&h.text, &q), Place::Heading(i)));
        }
    }
    for (i, r) in graph.regions.iter().enumerate() {
        if !graph.is_reachable(Some(i), &r.node) {
            continue;
        }
        let by_name = r.name.as_deref().map_or(0, |n| name_score(n, &q));
        let by_role = REGION_WORDS
            .iter()
            .any(|(word, role)| *word == q && *role == r.role);
        scored.push((by_name.max(u8::from(by_role)), Place::Region(i)));
    }
    let best = scored.iter().map(|(s, _)| *s).max().unwrap_or(0);
    if best == 0 {
        return PlaceResolution::None;
    }
    let hits: Vec<Place> = scored
        .into_iter()
        .filter(|(s, _)| *s == best)
        .map(|(_, p)| p)
        .collect();
    let covered = |r: usize| {
        hits.iter().any(|other| {
            matches!(other, Place::Heading(h) if graph.within(graph.headings[*h].region, r))
        })
    };
    let hits: Vec<Place> = hits
        .iter()
        .copied()
        .filter(|p| !matches!(p, Place::Region(r) if covered(*r)))
        .collect();
    match hits.as_slice() {
        [one] => PlaceResolution::One(*one),
        _ => PlaceResolution::Many(hits),
    }
}

/// Abschnitt oder Bereich am Fokus: die Überschrift des Abschnitts, sonst
/// der Bereich des fokussierten Bedienelements.
pub fn current_place(graph: &Graph, focus: Option<&NodeRef>) -> Option<Place> {
    let anchor = graph.anchor(focus)?;
    if let Some(h) = graph.section_of(anchor) {
        return Some(Place::Heading(h));
    }
    match anchor {
        Anchor::Control(i) => graph.controls[i].region.map(Place::Region),
        Anchor::Heading(_) => None,
    }
}

/// Nächste/vorige erreichbare Überschrift vom Fokus aus. Ohne Fokus auf
/// Bedienelement oder Überschrift beginnt „nächste“ am Seitenanfang.
pub fn step_heading(graph: &Graph, focus: Option<&NodeRef>, step: Step) -> Option<usize> {
    let reachable = |h: &usize| {
        let h = &graph.headings[*h];
        graph.is_reachable(h.region, &h.node)
    };
    let anchor = graph.anchor(focus);
    match step {
        Step::Next => {
            let start = match anchor {
                Some(Anchor::Heading(h)) => h + 1,
                Some(Anchor::Control(i)) => graph.controls[i].heading.map_or(0, |h| h + 1),
                None => 0,
            };
            (start..graph.headings.len()).find(reachable)
        }
        Step::Previous => {
            // Aus einem Abschnitt heraus zuerst zu seiner eigenen Überschrift.
            let end = match anchor {
                Some(Anchor::Heading(h)) => h,
                Some(Anchor::Control(i)) => graph.controls[i].heading.map_or(0, |h| h + 1),
                None => 0,
            };
            (0..end).rev().find(reachable)
        }
    }
}

/// Rollen, die als Formularfeld zählen.
fn is_field(role: &Role) -> bool {
    matches!(
        role,
        Role::Textbox
            | Role::SearchBox
            | Role::Combobox
            | Role::Listbox
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Slider
            | Role::SpinButton
    )
}

/// Nächstes/voriges erreichbares, nicht deaktiviertes Formularfeld vom
/// Fokus aus. Steht der Fokus auf einer Überschrift, zählt ihre Stelle.
pub fn step_field<'g>(
    graph: &'g Graph,
    focus: Option<&NodeRef>,
    step: Step,
) -> Option<&'g Control> {
    let anchor = graph.anchor(focus);
    let is_field =
        |c: &Control| is_field(&c.role) && !c.disabled && graph.is_reachable(c.region, &c.node);
    let after = |i: usize, c: &Control| match anchor {
        Some(Anchor::Control(f)) => i > f,
        Some(Anchor::Heading(h)) => c.heading.is_some_and(|x| x >= h),
        None => true,
    };
    let before = |i: usize, c: &Control| match anchor {
        Some(Anchor::Control(f)) => i < f,
        Some(Anchor::Heading(h)) => c.heading.is_none_or(|x| x < h),
        None => false,
    };
    let mut fields = graph.controls.iter().enumerate();
    let hit = match step {
        Step::Next => fields.find(|(i, c)| is_field(c) && after(*i, c)),
        Step::Previous => fields.rev().find(|(i, c)| is_field(c) && before(*i, c)),
    };
    hit.map(|(_, c)| c)
}

/// Wie sich das Offene schließen lässt.
#[derive(Debug)]
pub enum Dismissal<'g> {
    /// Schließen-Button im Ziel-Dialog.
    Button(&'g Control),
    /// Kein Schließen-Button: Escape an das fokussierte Element.
    Escape {
        /// Was geschlossen werden soll.
        target: String,
        /// Liegt der Fokus nicht im Ziel, erreicht Escape stattdessen dieses
        /// Element bzw. das Dokument. `None`: Der Fokus liegt im Ziel.
        reaches: Option<String>,
    },
    NothingOpen,
}

const CLOSE_WORDS: &[&str] = &["schließen", "close", "abbrechen", "cancel", "zurück", "x"];

/// Was „schließe den Dialog“ schließt, in dieser Reihenfolge:
///
/// 1. ein modaler Dialog: der mit dem Fokus, sonst der zuletzt geöffnete
///    ([`Graph::active_modal`]) — er sperrt die Bedienung;
/// 2. ein nicht-modaler Dialog, der den Fokus enthält;
/// 3. ein aufgeklapptes Popup-Element (`expanded` mit `hasPopup`): Das hat
///    die Nutzerin geöffnet, anders als ein seit dem Laden stehendes Banner;
/// 4. der letzte Dialog in Dokumentreihenfolge.
///
/// Dialoge sind im Graph nur, solange sie nicht ignoriert sind. `focus` ist
/// der fokussierte Knoten, `model` liefert seine Vorfahren.
pub fn dismissal<'g>(
    graph: &'g Graph,
    model: &SemanticGraph,
    focus: Option<&NodeRef>,
) -> Dismissal<'g> {
    let dialogs = || {
        graph
            .regions
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, r)| matches!(r.role, Role::Dialog | Role::AlertDialog))
    };
    // Rückwärts: Bei verschachtelten Dialogen gewinnt der innerste.
    let with_focus = |modal: bool| {
        dialogs()
            .filter(|(_, r)| r.modal == modal)
            .find(|(_, r)| focus.is_some_and(|f| contains(model, &r.node, f)))
            .map(|(i, _)| i)
    };
    let expanded = || {
        graph.controls.iter().find(|c| {
            c.states.iter().any(|(k, v)| k == "expanded" && v == "true")
                && c.states.iter().any(|(k, _)| k == "hasPopup")
        })
    };
    let dialog = with_focus(true)
        .or_else(|| graph.active_modal())
        .or_else(|| with_focus(false));
    let idx = match (dialog, expanded()) {
        (Some(i), _) => i,
        (None, Some(c)) => {
            // Das Popup (`aria-controls`) gehört zum Ziel: Der Fokus steht
            // bei Menüs oft auf einem Eintrag darin.
            let popup = model
                .node(&c.node)
                .map(|n| n.relations.controls.as_slice())
                .unwrap_or_default()
                .iter()
                .map(|id| NodeRef::new(c.node.tree.clone(), *id));
            let targets: Vec<NodeRef> = std::iter::once(c.node.clone()).chain(popup).collect();
            return Dismissal::Escape {
                target: format!("{} „{}“", c.role, c.display_name()),
                reaches: escape_reaches(model, focus, &targets),
            };
        }
        (None, None) => match dialogs().next() {
            Some((i, _)) => i,
            None => return Dismissal::NothingOpen,
        },
    };
    let close = graph.controls.iter().find(|c| {
        graph.within(c.region, idx)
            && c.role == Role::Button
            && c.name.value.as_deref().is_some_and(|raw| {
                // Relief stimmt nie selbst zu: „Akzeptieren und schließen“
                // ist kein Schließen-Button (→ `crate::overlay`).
                if crate::overlay::blocks_dismissal(raw) {
                    return false;
                }
                // Symbole wie „×“ fallen bei `normalize` weg.
                if ["×", "✕"].contains(&raw.trim()) {
                    return true;
                }
                let n = normalize(raw);
                CLOSE_WORDS
                    .iter()
                    .any(|w| n == *w || (w.len() > 2 && n.split_whitespace().any(|x| x == *w)))
            })
    });
    let region = &graph.regions[idx];
    match close {
        Some(c) => Dismissal::Button(c),
        None => Dismissal::Escape {
            target: region.label(),
            reaches: escape_reaches(model, focus, std::slice::from_ref(&region.node)),
        },
    }
}

/// Escape geht an das fokussierte Element. Liegt es in keinem Knoten von
/// `target`: was Escape stattdessen erreicht.
fn escape_reaches(
    model: &SemanticGraph,
    focus: Option<&NodeRef>,
    target: &[NodeRef],
) -> Option<String> {
    let Some(focus) = focus else {
        return Some("das Dokument (kein Element fokussiert)".into());
    };
    if target.iter().any(|t| contains(model, t, focus)) {
        return None;
    }
    let node = model.node(focus)?;
    Some(match non_empty(node.name.value.as_deref()) {
        Some(name) => format!("{} „{name}“", node.role),
        None => node.role.to_string(),
    })
}

/// Liegt `node` in `ancestor` (oder ist es selbst)? Über Frame-Grenzen
/// hinweg: Die Wurzel eines iframe-Dokuments hängt am `Iframe`-Knoten
/// (`TreeData::parent`).
fn contains(model: &SemanticGraph, ancestor: &NodeRef, node: &NodeRef) -> bool {
    let mut current = Some(node.clone());
    while let Some(at) = current {
        if at == *ancestor {
            return true;
        }
        current = match model.node(&at).and_then(|n| n.parent) {
            Some(parent) => Some(NodeRef::new(at.tree.clone(), parent)),
            None => model
                .trees
                .get(&at.tree)
                .and_then(|t| t.data.parent.clone()),
        };
    }
    false
}

fn score(control: &Control, q: &str) -> u8 {
    if let Some(name) = &control.name.value {
        let s = name_score(name, q);
        if s > 0 {
            return s;
        }
    }
    if ROLE_WORDS
        .iter()
        .any(|(word, roles)| *word == q && roles.contains(&control.role))
    {
        return 1;
    }
    0
}

/// 4 gleich, 3 Anfang oder ganzes Wort, 2 Wortanfang, 0 kein Treffer.
fn name_score(name: &str, q: &str) -> u8 {
    let n = normalize(name);
    if n == q {
        return 4;
    }
    if n.starts_with(q) || n.contains(&format!(" {q} ")) || n.ends_with(&format!(" {q}")) {
        return 3;
    }
    // Nur Wortanfänge: „Suche“ darf „Suchergebnisse“ treffen, aber nicht
    // „Besucher“ (casoon.de, Korpuslauf 2026-09-24).
    if n.split_whitespace().any(|w| w.starts_with(q)) || n.contains(&format!(" {q}")) {
        return 2;
    }
    0
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '@' || c == '.' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::at;

    #[test]
    fn match_only_at_word_start() {
        assert_eq!(
            score_name(
                "Meine Webseite lädt zu langsam, Besucher springen ab",
                "suche"
            ),
            0
        );
        assert_eq!(score_name("Suchergebnisse anzeigen", "suche"), 3);
        assert_eq!(score_name("Zur Suche", "suche"), 3);
        assert_eq!(score_name("In den Warenkorb", "warenkorb"), 3);
        assert_eq!(score_name("Warenkorb (0)", "warenkorb"), 3);
        assert_eq!(score_name("Suche nach Produkten", "suche nach"), 3);
        assert_eq!(score_name("Jetzt Suchfeld öffnen", "such"), 2);
    }

    fn sample() -> Graph {
        Graph::build(&crate::graph::sample_tree())
    }

    #[test]
    fn place_by_heading_region_and_role_word() {
        let g = sample();
        // Bereich und Überschrift heißen gleich → die Überschrift.
        assert_eq!(
            resolve_place(&g, "Technische Daten"),
            PlaceResolution::One(Place::Heading(1))
        );
        assert_eq!(
            resolve_place(&g, "versand"),
            PlaceResolution::One(Place::Heading(3))
        );
        assert_eq!(
            resolve_place(&g, "Bestellformular"),
            PlaceResolution::One(Place::Region(2))
        );
        assert_eq!(
            resolve_place(&g, "Fußzeile"),
            PlaceResolution::One(Place::Region(3))
        );
        assert_eq!(resolve_place(&g, "Kommentare"), PlaceResolution::None);
        // „Bestell…“ trifft Überschrift „Bestellung“ und Formular gleich gut.
        assert!(matches!(
            resolve_place(&g, "bestell"),
            PlaceResolution::Many(_)
        ));
    }

    #[test]
    fn heading_steps_follow_focus() {
        let g = sample();
        assert_eq!(step_heading(&g, None, Step::Next), Some(0));
        assert_eq!(step_heading(&g, None, Step::Previous), None);
        assert_eq!(step_heading(&g, Some(&at(10)), Step::Next), Some(2));
        assert_eq!(step_heading(&g, Some(&at(10)), Step::Previous), Some(0));
        // Fokus auf „Menge“ im Abschnitt „Bestellung“ (Überschrift 2).
        assert_eq!(step_heading(&g, Some(&at(20)), Step::Next), Some(3));
        assert_eq!(step_heading(&g, Some(&at(20)), Step::Previous), Some(2));
        assert_eq!(step_heading(&g, Some(&at(23)), Step::Next), None);
    }

    #[test]
    fn field_steps_skip_links_and_buttons() {
        let g = sample();
        let name = |c: Option<&Control>| c.and_then(|c| c.name.value.clone());
        assert_eq!(
            name(step_field(&g, None, Step::Next)).as_deref(),
            Some("Größe")
        );
        // Von der Überschrift „Technische Daten“ aus: erstes Feld danach.
        assert_eq!(
            name(step_field(&g, Some(&at(10)), Step::Next)).as_deref(),
            Some("Größe")
        );
        assert_eq!(
            name(step_field(&g, Some(&at(16)), Step::Next)).as_deref(),
            Some("Menge")
        );
        // Nach „Menge“ kommen nur Button und Link.
        assert_eq!(
            step_field(&g, Some(&at(20)), Step::Next).map(|c| c.node.clone()),
            None
        );
        assert_eq!(
            name(step_field(&g, Some(&at(20)), Step::Previous)).as_deref(),
            Some("Größe")
        );
        assert_eq!(
            name(step_field(&g, Some(&at(23)), Step::Previous)).as_deref(),
            Some("Menge")
        );
        assert!(step_field(&g, Some(&at(3)), Step::Previous).is_none());
    }

    #[test]
    fn current_place_prefers_section() {
        let g = sample();
        assert_eq!(current_place(&g, Some(&at(20))), Some(Place::Heading(2)));
        assert_eq!(current_place(&g, Some(&at(10))), Some(Place::Heading(1)));
        assert_eq!(current_place(&g, Some(&at(13))), None);
    }

    #[test]
    fn inflected_query_finds_singular() {
        let g = sample();
        let any = |_: &Control| true;
        assert!(matches!(resolve(&g, "Größen", any), Resolution::None));
        assert!(matches!(
            resolve_inflected(&g, "Größen", any),
            Resolution::One(c) if c.role == Role::Combobox
        ));
        assert!(matches!(
            resolve_inflected(&g, "Mengen", any),
            Resolution::One(c) if c.role == Role::SpinButton
        ));
    }

    fn score_name(name: &str, q: &str) -> u8 {
        let c = Control {
            node: at(1),
            dom_node_id: Some(1),
            role: Role::Button,
            name: relief_model::Fact::known(Some(name.into())),
            region: None,
            value: None,
            options: vec![],
            selected_option: None,
            disabled: false,
            focusable: true,
            clickable: false,
            states: vec![],
            expandable: false,
            heading: None,
        };
        score(&c, &normalize(q))
    }
}
