//! Antworttexte. Unsicheres wird als unsicher formuliert.

use std::collections::HashSet;

use crate::command::ScrollDirection;
use crate::graph::{focused, invalid_name, Anchor, Control, Graph};
use crate::page::PageType;
use crate::resolve::Place;
use relief_model::{Certainty, NodeRef, Role, SemanticGraph, SemanticNode, TreeData, TreeDelta};

pub fn describe(graph: &Graph) -> String {
    let mut out = String::new();
    match &graph.title {
        Some(t) => out.push_str(&format!("Seite „{t}“.")),
        None => out.push_str("Seite ohne Titel."),
    }
    if let Some(t) = page_type(graph) {
        out.push(' ');
        out.push_str(&t);
    }
    if let Some(h1) = graph.headings.iter().find(|h| h.level == 1) {
        out.push_str(&format!(" Hauptüberschrift: „{}“.", h1.text));
    }
    // Benannte `section`s werden zu `region` — auf Artikelseiten dutzende.
    // Sie sind Gliederung, keine Orientierung auf Seitenebene.
    let sections = graph
        .regions
        .iter()
        .filter(|r| r.role == Role::Region)
        .count();
    let mut regions: Vec<String> = graph
        .regions
        .iter()
        .filter(|r| r.role != Role::Region || sections <= 3)
        .map(|r| r.label())
        .collect();
    if sections > 3 {
        regions.push(format!("{sections} benannte Abschnitte"));
    }
    if !regions.is_empty() {
        out.push_str(&format!(" Bereiche: {}.", regions.join(", ")));
    }
    let unsure = graph
        .controls
        .iter()
        .filter(|c| c.name.certainty != Certainty::Known)
        .count();
    out.push_str(&format!(
        " {} Überschriften, {} Bedienelemente",
        graph.headings.len(),
        graph.controls.len()
    ));
    if unsure > 0 {
        out.push_str(&format!(", davon {unsure} ohne gesicherten Namen"));
    }
    out.push('.');
    out
}

/// Seitentyp als Satz, immer als Vermutung und mit Evidence; bei
/// „unbekannt“ nichts (keine Aussage ist hier ehrlicher als eine leere).
pub fn page_type(graph: &Graph) -> Option<String> {
    let kind = &graph.page.kind;
    let t = kind.value.filter(|t| *t != PageType::Unknown)?;
    Some(match kind.certainty {
        Certainty::Uncertain => format!(
            "Seitentyp möglicherweise {}, unsicher (Hinweise: {}).",
            t.label(),
            kind.evidence.join(", ")
        ),
        _ => format!(
            "Seitentyp vermutlich {} (erschlossen: {}).",
            t.label(),
            kind.evidence.join(", ")
        ),
    })
}

pub fn list_actions(graph: &Graph) -> String {
    let mut out = String::new();
    let mut current: Option<Option<usize>> = None;
    for c in graph.reachable_controls() {
        if current != Some(c.region) {
            out.push_str(&format!("{}:\n", graph.region_label(c.region)));
            current = Some(c.region);
        }
        out.push_str(&format!("  - {}\n", control_line(c)));
    }
    if out.is_empty() {
        out.push_str("Keine Bedienelemente gefunden.");
    }
    // Ein modaler Dialog im iframe sperrt nur sein Dokument; bleibt dort
    // nichts gesperrt, gibt es nichts zu melden.
    let blocked = graph.controls.len() - graph.reachable_controls().count();
    if let Some(m) = graph.active_modal().filter(|_| blocked > 0) {
        out.push_str(&format!(
            "\n{blocked} weitere Bedienelemente der Seite sind gesperrt, solange „{}“ offen ist.",
            graph.regions[m].name.as_deref().unwrap_or("der Dialog")
        ));
    }
    out.trim_end().to_string()
}

pub fn control_line(c: &Control) -> String {
    let mut line = format!("[{}] {}", c.role, c.display_name());
    match c.name.certainty {
        Certainty::Known => {}
        Certainty::Inferred => {
            line.push_str(&format!(" (erschlossen: {})", c.name.evidence.join(", ")))
        }
        Certainty::Uncertain => line.push_str(" (Name unbekannt)"),
    }
    if !c.options.is_empty() {
        line.push_str(&format!(" Optionen: {}", c.options.join(", ")));
        if let Some(sel) = &c.selected_option {
            line.push_str(&format!("; gewählt: {sel}"));
        }
    } else if let Some(v) = &c.value {
        line.push_str(&format!(" = „{v}“"));
    }
    for (state, value) in &c.states {
        line.push_str(&format!(" {state}={value}"));
    }
    if c.disabled {
        line.push_str(" deaktiviert");
    }
    line
}

pub fn list_headings(graph: &Graph) -> String {
    if graph.headings.is_empty() {
        return "Keine Überschriften.".into();
    }
    graph
        .headings
        .iter()
        .map(|h| {
            format!(
                "{}H{} {}",
                "  ".repeat(h.level.saturating_sub(1) as usize),
                h.level,
                h.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wo der Fokus steht: Element, Bereich, Abschnitt, Seite.
pub fn where_am_i(graph: &Graph, focus: Option<&NodeRef>) -> String {
    let mut parts = Vec::new();
    match graph.anchor(focus) {
        Some(Anchor::Control(i)) => {
            let c = &graph.controls[i];
            parts.push(format!("Fokus auf {}", control_line(c)));
            parts.push(format!("Bereich: {}", graph.region_label(c.region)));
            if let Some(h) = c.heading {
                parts.push(format!("Abschnitt: {}", heading_label(graph, h)));
            }
        }
        Some(Anchor::Heading(h)) => {
            parts.push(format!("Fokus auf Überschrift {}", heading_label(graph, h)));
            parts.push(format!(
                "Bereich: {}",
                graph.region_label(graph.headings[h].region)
            ));
        }
        None => parts.push("Fokus liegt auf keinem Bedienelement und keiner Überschrift".into()),
    }
    parts.push(match &graph.title {
        Some(t) => format!("Seite „{t}“"),
        None => "Seite ohne Titel".into(),
    });
    parts.join(". ") + "."
}

fn heading_label(graph: &Graph, h: usize) -> String {
    let h = &graph.headings[h];
    format!("H{} „{}“", h.level, h.text)
}

/// Anzeigename eines Abschnitts oder Bereichs.
pub fn place_label(graph: &Graph, place: Place) -> String {
    match place {
        Place::Heading(h) => format!("Abschnitt {}", heading_label(graph, h)),
        Place::Region(r) => format!("Bereich {}", graph.regions[r].label()),
    }
}

/// Text eines Abschnitts (bis zur nächsten Überschrift gleicher oder höherer
/// Ebene) oder Bereichs vorlesen; Unterüberschriften in eckigen Klammern,
/// Bedienelemente außer Links (die im Text stehen) danach.
pub fn read_place(graph: &Graph, place: Place) -> String {
    let (own_heading, end) = match place {
        Place::Heading(h) => (Some(h), graph.section_end(h)),
        Place::Region(_) => (None, 0),
    };
    let in_place = |heading: Option<usize>, region: Option<usize>| match place {
        Place::Heading(h) => heading.is_some_and(|x| h <= x && x < end),
        Place::Region(r) => graph.within(region, r),
    };
    let text: Vec<String> = graph
        .texts
        .iter()
        .filter(|t| in_place(t.heading, t.region))
        .filter(|t| !(t.level.is_some() && t.heading == own_heading))
        .map(|t| match t.level {
            Some(l) => format!("[H{l} {}]", t.text),
            None => t.text.clone(),
        })
        .collect();
    let controls: Vec<String> = graph
        .controls
        .iter()
        .filter(|c| c.role != Role::Link && in_place(c.heading, c.region))
        .map(|c| {
            let mut s = format!("[{}] {}", c.role, c.display_name());
            if c.name.certainty == Certainty::Uncertain {
                s.push_str(" (Name unbekannt)");
            }
            s
        })
        .collect();

    let mut out = format!("{}: ", place_label(graph, place));
    if text.is_empty() {
        out.push_str("kein Text.");
    }
    for (i, t) in text.iter().enumerate() {
        // Satzzeichen hinter einem Link sind ein eigener Textknoten.
        if i > 0 && !t.starts_with(['.', ',', ';', ':', '!', '?', ')']) {
            out.push(' ');
        }
        out.push_str(t);
    }
    if !controls.is_empty() {
        out.push_str(&format!(" Bedienelemente: {}.", controls.join(", ")));
    }
    out
}

/// Was ein Bedienelement anbietet: Optionen, Wert, Grenzen, Zustände.
pub fn inspect(c: &Control) -> String {
    let state = |key: &str| {
        c.states
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let mut out = format!("[{}] {}", c.role, c.display_name());
    match c.name.certainty {
        Certainty::Known => {}
        Certainty::Inferred => {
            out.push_str(&format!(" (erschlossen: {})", c.name.evidence.join(", ")))
        }
        Certainty::Uncertain => out.push_str(" (Name unbekannt)"),
    }
    out.push('.');
    if !c.options.is_empty() {
        out.push_str(&format!(
            " {} Optionen: {}.",
            c.options.len(),
            c.options.join(", ")
        ));
        match &c.selected_option {
            Some(sel) => out.push_str(&format!(" Gewählt: {sel}.")),
            None => out.push_str(" Nichts gewählt."),
        }
    } else if let Some(v) = &c.value {
        out.push_str(&format!(" Wert: {v}"));
        if let Some(text) = state("valuetext").filter(|t| *t != v) {
            out.push_str(&format!(" („{text}“)"));
        }
        if let (Some(min), Some(max)) = (state("valuemin"), state("valuemax")) {
            out.push_str(&format!(", möglich von {min} bis {max}"));
        }
        out.push('.');
    } else {
        out.push_str(" Kein Wert.");
    }
    let others: Vec<String> = c
        .states
        .iter()
        .filter(|(k, _)| !k.starts_with("value"))
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if !others.is_empty() {
        out.push_str(&format!(" Zustand: {}.", others.join(" ")));
    }
    if c.disabled {
        out.push_str(" Deaktiviert.");
    }
    out
}

/// Ergebnis eines Scrollbefehls aus Position vorher/nachher und größter
/// Position (Pixel).
pub fn scrolled(direction: ScrollDirection, before: f64, after: f64, max: f64) -> String {
    if max <= 0.0 {
        return "Scroll: Die Seite passt ganz ins Fenster, nichts zu scrollen.".into();
    }
    if (after - before).abs() < 1.0 {
        return match direction {
            ScrollDirection::Down | ScrollDirection::Bottom => "Scroll: schon am Seitenende.",
            ScrollDirection::Up | ScrollDirection::Top => "Scroll: schon am Seitenanfang.",
        }
        .into();
    }
    let way = match direction {
        ScrollDirection::Down => "nach unten",
        ScrollDirection::Up => "nach oben",
        ScrollDirection::Top => "zum Seitenanfang",
        ScrollDirection::Bottom => "zum Seitenende",
    };
    let percent = (after / max * 100.0).round();
    let where_ = if after >= max - 1.0 {
        "Seitenende erreicht".to_string()
    } else if after < 1.0 {
        "Seitenanfang erreicht".to_string()
    } else {
        format!("jetzt bei {percent} % der Seite")
    };
    format!("Scroll {way}: {where_}.")
}

/// Wert- und Zustandswechsel des bedienten Elements selbst (`checked`,
/// `pressed` …), über den Graph statt über den Diff: `describe_diff` verfolgt
/// nur einen Teil der Zustände.
pub fn target_change(before: &Graph, after: &Graph, target: &NodeRef) -> Option<String> {
    let find = |g: &'_ Graph| {
        g.controls
            .iter()
            .find(|c| c.node == *target)
            .map(|c| (c.states.clone(), c.value.clone()))
    };
    let ((old, old_value), (new, new_value)) = (find(before)?, find(after)?);
    let value = (old_value != new_value).then(|| {
        format!(
            "Wert {} → {}",
            old_value.as_deref().unwrap_or("leer"),
            new_value.as_deref().unwrap_or("leer")
        )
    });
    let changes: Vec<String> = value
        .into_iter()
        .chain(
            new.iter()
                .filter(|s| !old.contains(s))
                .map(|(k, v)| format!("{k}={v}")),
        )
        .chain(
            old.iter()
                .filter(|(k, _)| !new.iter().any(|(nk, _)| nk == k))
                .map(|(k, _)| format!("{k} aufgehoben")),
        )
        .collect();
    (!changes.is_empty()).then(|| format!("Ziel jetzt: {}", changes.join(", ")))
}

/// Was eine Aktion wahrnehmbar verändert hat: Vergleich zweier Stände des
/// Modells über die [`TreeDelta`] dazwischen.
///
/// „Wahrnehmbar geworden“ heißt: nachher vorhanden und nicht ignoriert,
/// vorher nicht vorhanden oder ignoriert. Chromium lässt verborgene Knoten
/// als `ignored` im Baum stehen; ein aufklappendes Menü ist also meist kein
/// neuer, sondern ein nicht mehr ignorierter Knoten. Zugeordnet wird über
/// [`NodeRef`]; nach einem Dokumentwechsel (neue Tree-ID) ist alles neu.
/// Aufgezählt wird in Dokumentreihenfolge.
pub fn describe_diff(before: &SemanticGraph, after: &SemanticGraph) -> String {
    describe_diff_at(
        before,
        after,
        focused(before).as_ref(),
        focused(after).as_ref(),
    )
}

/// Wie [`describe_diff`], mit dem Fokus vorher und nachher vom Aufrufer
/// (Position der Sitzung statt des Fokus im Modell, → `session`).
pub fn describe_diff_at(
    before: &SemanticGraph,
    after: &SemanticGraph,
    focus_before: Option<&NodeRef>,
    focus_after: Option<&NodeRef>,
) -> String {
    let delta = TreeDelta::between(before, after);
    let mut parts = Vec::new();

    let main = |g: &SemanticGraph| -> TreeData {
        g.root
            .as_ref()
            .and_then(|t| g.trees.get(t))
            .map(|t| t.data.clone())
            .unwrap_or_default()
    };
    let (old, new) = (main(before), main(after));
    if old.url != new.url {
        parts.push(format!(
            "Neue Adresse: {}",
            new.url.as_deref().unwrap_or("")
        ));
    }
    if old.title != new.title {
        parts.push(format!(
            "Neuer Titel: „{}“",
            new.title.as_deref().unwrap_or("")
        ));
    }
    if focus_before != focus_after {
        let name = focus_after
            .and_then(|at| after.node(at))
            .map(|n| format!("{} „{}“", n.role, n.name.value.as_deref().unwrap_or("")))
            .unwrap_or_else(|| "unbekanntes Element".into());
        parts.push(format!("Fokus jetzt auf {name}"));
    }

    // Knoten, die die Delta berührt: nachher angelegt oder geändert, vorher
    // entfernt oder geändert (auch mit ihrem ganzen Baum).
    let mut touched_after: HashSet<NodeRef> = HashSet::new();
    let mut touched_before: HashSet<NodeRef> = HashSet::new();
    for update in &delta.trees {
        let at = |id| NodeRef::new(update.tree.clone(), id);
        for n in update.created.iter().chain(&update.changed) {
            touched_after.insert(at(n.id));
        }
        for n in &update.changed {
            touched_before.insert(at(n.id));
        }
        for id in &update.removed {
            touched_before.insert(at(*id));
        }
    }
    for tree in &delta.removed_trees {
        for id in before.trees[tree].nodes.keys() {
            touched_before.insert(NodeRef::new(tree.clone(), *id));
        }
    }
    let perceivable = |g: &SemanticGraph, at: &NodeRef| g.node(at).is_some_and(|n| !n.ignored);

    let mut added_all: Vec<&SemanticNode> = Vec::new();
    for at in after.document_order() {
        if !touched_after.contains(&at) || !perceivable(after, &at) {
            continue;
        }
        let node = after.node(&at).expect("aus der Dokumentreihenfolge");
        if !perceivable(before, &at) {
            added_all.push(node);
            continue;
        }
        // Ein Knoten, der vorher oder nachher ignoriert ist, meldet sich
        // über Zu- und Abgang, nicht über seine Eigenschaften.
        let prev = before.node(&at).expect("vorher wahrnehmbar");
        for ((property, old), (_, new)) in tracked(prev).into_iter().zip(tracked(node)) {
            if let (Some(old), Some(new)) = (old, new) {
                if old != new {
                    parts.push(format!(
                        "„{}“: {property} {old} → {new}",
                        node.name.value.as_deref().unwrap_or("")
                    ));
                }
            }
        }
    }
    let removed_all: Vec<&SemanticNode> = before
        .document_order()
        .into_iter()
        .filter(|at| touched_before.contains(at))
        .filter(|at| perceivable(before, at) && !perceivable(after, at))
        .filter_map(|at| before.node(&at))
        .collect();

    // Textknoten entstehen bei jeder Textänderung neu; gemeldet werden die
    // Elemente, nicht ihre Textboxen.
    let is_text = |n: &&SemanticNode| matches!(n.role, Role::StaticText | Role::InlineTextBox);
    let added: Vec<&SemanticNode> = added_all.iter().copied().filter(|n| !is_text(n)).collect();
    let removed = removed_all.iter().filter(|n| !is_text(n)).count();

    let is_dialog = |n: &&SemanticNode| matches!(n.role, Role::Dialog | Role::AlertDialog);
    let opened_dialog = added.iter().find(|n| is_dialog(n));
    let closed_dialog = removed_all.iter().find(|n| is_dialog(n));
    let name = |n: &SemanticNode| n.name.value.clone().unwrap_or_default();
    if let Some(d) = closed_dialog.filter(|_| opened_dialog.is_none()) {
        parts.push(format!("Dialog „{}“ geschlossen", name(d)));
    } else if let Some(d) = opened_dialog {
        parts.push(format!("Dialog „{}“ geöffnet", name(d)));
        // Nur ein modaler Dialog sperrt den Rest; Vorschlagslisten o. ä.
        // tragen oft ebenfalls die Rolle `dialog`. Bei `aria-modal` blendet
        // Chrome den Rest nicht aus (nichts entfernt), Relief sperrt ihn im
        // Graph trotzdem (`Graph::is_reachable`) — also auch dann melden.
        // Ein Dialog im iframe sperrt nur sein Dokument, nicht die Seite.
        if d.states.modal {
            let in_main = after
                .root
                .as_ref()
                .and_then(|t| after.trees.get(t))
                .and_then(|t| t.nodes.get(&d.id))
                .is_some_and(|n| std::ptr::eq(n, *d));
            parts.push(if in_main {
                "Rest der Seite ist gesperrt, solange er offen ist".into()
            } else {
                "Rest seines Frames ist gesperrt, solange er offen ist".into()
            });
        }
    } else {
        let named: Vec<String> = added
            .iter()
            .filter_map(|n| {
                let text = n.name.value.as_deref()?;
                (!text.trim().is_empty()).then(|| format!("{} „{text}“", n.role))
            })
            .take(5)
            .collect();
        if !named.is_empty() {
            parts.push(format!("Neu wahrnehmbar: {}", named.join(", ")));
        } else if !added.is_empty() {
            parts.push(format!("{} Elemente neu wahrnehmbar", added.len()));
        }
        if removed > 0 {
            parts.push(format!("{removed} Elemente nicht mehr wahrnehmbar"));
        }
    }

    // Geänderter Text ohne neue Elemente (Zähler, Live-Region) — über die
    // Textknoten, die dazugekommen sind.
    let texts: Vec<&str> = added_all
        .iter()
        .filter(|n| n.role == Role::StaticText)
        .filter_map(|n| n.name.value.as_deref())
        .filter(|t| t.chars().any(char::is_alphanumeric))
        .take(3)
        .collect();
    if !texts.is_empty() && opened_dialog.is_none() {
        parts.push(format!(
            "Neuer Text: {}",
            texts
                .iter()
                .map(|t| format!("„{t}“"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if parts.is_empty() {
        "Keine wahrnehmbare Änderung.".into()
    } else {
        parts.join(". ") + "."
    }
}

/// Zustände, deren Wechsel `describe_diff` meldet, als Text; `None`, wo der
/// Knoten den Zustand nicht führt. `invalid` fehlt nie: kein Fehler heißt
/// `false`.
fn tracked(n: &SemanticNode) -> [(&'static str, Option<String>); 4] {
    let s = &n.states;
    [
        ("expanded", s.expanded.map(|b| b.to_string())),
        (
            "invalid",
            Some(s.invalid.as_ref().map_or("false", invalid_name).to_string()),
        ),
        ("modal", Some(s.modal.to_string())),
        ("selected", s.selected.map(|b| b.to_string())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::at;

    fn sample() -> Graph {
        Graph::build(&crate::graph::sample_tree())
    }

    #[test]
    fn where_am_i_names_control_region_section_and_page() {
        let g = sample();
        assert_eq!(
            where_am_i(&g, Some(&at(20))),
            "Fokus auf [spinbutton] Menge = „1“ valuemin=1 valuemax=5. \
             Bereich: form „Bestellformular“. Abschnitt: H2 „Bestellung“. Seite „Testseite“."
        );
        assert_eq!(
            where_am_i(&g, Some(&at(10))),
            "Fokus auf Überschrift H2 „Technische Daten“. \
             Bereich: region „Technische Daten“. Seite „Testseite“."
        );
        assert!(where_am_i(&g, None).starts_with("Fokus liegt auf keinem Bedienelement"));
    }

    #[test]
    fn read_section_until_next_heading_of_same_level() {
        let g = sample();
        assert_eq!(
            read_place(&g, Place::Heading(1)),
            "Abschnitt H2 „Technische Daten“: Obermaterial Leder."
        );
        // „Versand“ (H3) gehört zum Abschnitt „Bestellung“ (H2).
        assert_eq!(
            read_place(&g, Place::Heading(2)),
            "Abschnitt H2 „Bestellung“: [H3 Versand] Lieferung in zwei Tagen. \
             Bedienelemente: [combobox] Größe, [spinbutton] Menge, [button] Bestellen."
        );
        // Linktext bleibt im Satz.
        assert!(read_place(&g, Place::Heading(0))
            .contains("Robust und wasserdicht. Mehr in den Pflegehinweisen. [H2"));
        assert_eq!(
            read_place(&g, Place::Region(3)),
            "Bereich contentinfo: kein Text."
        );
    }

    #[test]
    fn inspect_lists_options_and_range() {
        let g = sample();
        let by = |n: &str| {
            g.controls
                .iter()
                .find(|c| c.name.value.as_deref() == Some(n))
                .unwrap()
        };
        assert_eq!(
            inspect(by("Größe")),
            "[combobox] Größe. 2 Optionen: 40, 41. Gewählt: 41."
        );
        assert_eq!(
            inspect(by("Menge")),
            "[spinbutton] Menge. Wert: 1, möglich von 1 bis 5."
        );
    }

    #[test]
    fn scroll_result_texts() {
        use ScrollDirection::*;
        assert_eq!(
            scrolled(Down, 0.0, 720.0, 2880.0),
            "Scroll nach unten: jetzt bei 25 % der Seite."
        );
        assert_eq!(
            scrolled(Bottom, 720.0, 2880.0, 2880.0),
            "Scroll zum Seitenende: Seitenende erreicht."
        );
        assert_eq!(
            scrolled(Down, 2880.0, 2880.0, 2880.0),
            "Scroll: schon am Seitenende."
        );
        assert_eq!(
            scrolled(Top, 0.0, 0.0, 2880.0),
            "Scroll: schon am Seitenanfang."
        );
        assert!(scrolled(Down, 0.0, 0.0, 0.0).contains("nichts zu scrollen"));
    }

    #[test]
    fn target_change_reports_value() {
        let before = sample();
        let mut after = before.clone();
        let i = after
            .controls
            .iter()
            .position(|c| c.node == at(20))
            .unwrap();
        after.controls[i].value = Some("2".into());
        assert_eq!(
            target_change(&before, &after, &at(20)).as_deref(),
            Some("Ziel jetzt: Wert 1 → 2")
        );
    }

    /// Ein Knoten, der nicht mehr ignoriert ist, ist wahrnehmbar geworden
    /// (so öffnen Menüs); Zustände wechseln nur an wahrnehmbaren Knoten.
    #[test]
    fn diff_reports_perceivable_and_state_changes() {
        let before = crate::graph::sample_tree();
        let mut hidden = before.clone();
        let tree = hidden.trees.values_mut().next().unwrap();
        tree.nodes.get_mut(&at(26).node).unwrap().ignored = true;
        tree.nodes.get_mut(&at(16).node).unwrap().states.expanded = Some(false);
        let mut shown = before.clone();
        let tree = shown.trees.values_mut().next().unwrap();
        tree.nodes.get_mut(&at(16).node).unwrap().states.expanded = Some(true);
        tree.data.focus = Some(at(16).node);
        assert_eq!(
            describe_diff(&hidden, &shown),
            "Fokus jetzt auf combobox „Größe“. „Größe“: expanded false → true. \
             Neu wahrnehmbar: link „Impressum“."
        );
        assert_eq!(
            describe_diff(&shown, &hidden),
            "Fokus jetzt auf unbekanntes Element. „Größe“: expanded true → false. \
             1 Elemente nicht mehr wahrnehmbar."
        );
        assert_eq!(
            describe_diff(&shown, &shown),
            "Keine wahrnehmbare Änderung."
        );
    }
}
