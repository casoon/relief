//! Formular-Zusicherungen (Aufgabenzeile `assert: …`): der Host erhebt die
//! Fakten, `relief_interaction::assertions` wertet sie browserfrei aus.
//!
//! - **Modell und Fokus**: aus der Session (Aufnahme wie bei `do:`), Fokus
//!   live abgefragt; dazu das Modell vor der letzten `do:`-Zeile.
//! - **DOM-Fakten** (nur `namen-wie-accname`): `DOM.getDocument` (mit
//!   `pierce`) als `DomFacts`, nur die Attribute aus `dom_attribute_needed`;
//!   iframes im selben Renderer-Prozess als eigene Dokumente, Shadow DOM
//!   (außer dem des Browsers selbst) flach unter dem Host. Dazu `display` und
//!   `visibility` je Element und die Leerraum-Textknoten, die
//!   `DOM.getDocument` auslässt, aus `DOMSnapshot.captureSnapshot`.
//! - **Tab-Folge** (nur `tabfolge`): echte Tab-Tasten ab Dokumentanfang, nach
//!   jeder Taste der Fokus. Wie Escape und Scrollen ohne `ActionPlan`: Tab hat
//!   kein Zielelement und bewegt nur den Fokus.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{
    DescribeNodeParams, GetDocumentParams, Node, ShadowRootType,
};
use chromiumoxide::cdp::browser_protocol::dom_snapshot::{
    CaptureSnapshotParams, CaptureSnapshotReturns,
};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::Page;
use relief_interaction::assertions::{
    self, dom_attribute_needed, Assertion, DomFacts, DomFactsBuilder, Observed,
};
use relief_model::{NodeRef, SemanticGraph};

use crate::{act, Session};

/// Höchstens so viele Tab-Schritte (Schutz vor Seiten ohne Ende der Folge).
const MAX_TABS: usize = 60;

/// Unsichtbarer Startpunkt am Dokumentanfang: fokussierbar nur per Skript,
/// also selbst nicht in der Tab-Folge.
const TAB_START: &str = "(() => {
  const s = document.createElement('span');
  s.id = 'relief-tab-start';
  s.tabIndex = -1;
  document.body.prepend(s);
  s.focus();
})()";

const TAB_END: &str = "document.getElementById('relief-tab-start')?.remove()";

/// Elemente, deren Inhalt nicht dargestellt wird; ihr Text gehört nicht in
/// die DOM-Fakten.
const NOT_RENDERED: &[&str] = &["script", "style", "template", "noscript"];

/// Eine Zusicherung prüfen; Ergebnis als Text (Befunde oder „Keine Befunde.“).
pub async fn run(session: &mut Session, text: &str) -> Result<String> {
    let assertion = match Assertion::parse(text) {
        Ok(a) => a,
        Err(msg) => return Ok(msg),
    };
    session.update(false, None).await?;
    let dom = if assertion.needs_dom() {
        Some(dom_facts(&session.page).await?)
    } else {
        None
    };
    let tabs = if assertion.needs_tab_walk() {
        Some(tab_walk(&session.page, &session.model).await?)
    } else {
        None
    };
    let focus = session.focus().await;
    let findings = assertions::check(
        &assertion,
        &Observed {
            model: &session.model,
            dom: dom.as_ref(),
            focus,
            before: session.before_action.as_ref(),
            tab_sequence: tabs.as_deref(),
        },
    );
    Ok(assertions::render(&findings))
}

/// DOM-Knotentypen (Zahlen des DOM-Standards, von CDP durchgereicht).
const ELEMENT_NODE: i64 = 1;
const TEXT_NODE: i64 = 3;

/// Seite als DOM-Fakten.
async fn dom_facts(page: &Page) -> Result<DomFacts> {
    // Wie `live.rs` (Tiefe -1, pierce): der DOM-Agent bleibt im selben Zustand.
    let document = page
        .execute(GetDocumentParams::builder().depth(-1).pierce(true).build())
        .await?;
    let snapshot = page
        .execute(CaptureSnapshotParams::new(vec![
            "display".to_string(),
            "visibility".to_string(),
        ]))
        .await?;
    let root = &document.result.root;
    let html = html_of(root).ok_or_else(|| anyhow!("Dokument ohne Wurzelelement"))?;
    let mut by_backend = HashMap::new();
    index_nodes(root, &mut by_backend);
    let walk = Walk {
        layout: Layout::from_snapshot(&snapshot.result),
        by_backend,
    };
    Ok(walk.node(DomFacts::builder(), html).build())
}

/// Das Wurzelelement unter einem Dokumentknoten.
fn html_of(document: &Node) -> Option<&Node> {
    document
        .children
        .iter()
        .flatten()
        .find(|n| n.node_type == ELEMENT_NODE)
}

/// Jeder Knoten samt Shadow-Roots nach Backend-ID, damit ein `<slot>` die ihm
/// zugewiesenen Light-DOM-Knoten findet.
fn index_nodes<'n>(node: &'n Node, map: &mut HashMap<i64, &'n Node>) {
    map.insert(*node.backend_node_id.inner(), node);
    for child in node
        .shadow_roots
        .iter()
        .flatten()
        .chain(node.children.iter().flatten())
    {
        index_nodes(child, map);
    }
}

/// Rendering aus `DOMSnapshot.captureSnapshot`, nach Backend-ID.
///
/// Elemente mit Layout-Objekt tragen ihr berechnetes `display` und
/// `visibility`. Ein Element **ohne** Layout-Objekt ist `display: none` (oder
/// liegt darunter) oder `display: contents`, Letzteres genau dann, wenn
/// darunter etwas gerendert wird. Dazu die Leerraum-Textknoten, die
/// `DOM.getDocument` auslässt: Zwischen `<span>a</span> <span>b</span>` trennt
/// nur dieser Knoten die Wörter, sobald Inline-Elemente direkt anschließen.
/// Verfahren aus auditmysite (`accessibility/dom_document.rs`, dort am
/// accname-Differentiallauf auf echten Seiten erprobt).
#[derive(Default)]
struct Layout {
    styles: HashMap<i64, (String, Option<String>)>,
    /// Knoten, vor denen ein reiner Leerraum-Textknoten steht.
    whitespace_before: HashSet<i64>,
    /// Knoten, deren letztes Kind ein reiner Leerraum-Textknoten ist.
    whitespace_last: HashSet<i64>,
}

impl Layout {
    fn from_snapshot(snapshot: &CaptureSnapshotReturns) -> Self {
        let string = |i: i64| {
            usize::try_from(i)
                .ok()
                .and_then(|i| snapshot.strings.get(i))
        };
        let mut layout = Layout::default();
        for document in &snapshot.documents {
            let nodes = &document.nodes;
            let (Some(backend), Some(types), Some(parents)) = (
                nodes.backend_node_id.as_ref(),
                nodes.node_type.as_ref(),
                nodes.parent_index.as_ref(),
            ) else {
                continue;
            };
            let mut styles: Vec<Option<(String, Option<String>)>> = vec![None; backend.len()];
            let mut renders_below = vec![false; backend.len()];
            for (layout_index, &node_index) in document.layout.node_index.iter().enumerate() {
                let Ok(node_index) = usize::try_from(node_index) else {
                    continue;
                };
                if let Some(values) = document.layout.styles.get(layout_index) {
                    let values = values.inner();
                    styles[node_index] = values.first().and_then(|d| string(*d.inner())).map(|d| {
                        let visibility = values.get(1).and_then(|v| string(*v.inner()));
                        (d.clone(), visibility.cloned())
                    });
                }
                // Jeder Vorfahre eines gerenderten Knotens hat Gerendertes unter sich.
                let mut current = parents.get(node_index).copied().unwrap_or(-1);
                while let Ok(parent) = usize::try_from(current) {
                    if renders_below[parent] {
                        break;
                    }
                    renders_below[parent] = true;
                    current = parents.get(parent).copied().unwrap_or(-1);
                }
            }
            // Der Snapshot steht in Dokumentreihenfolge; je Elternknoten wird
            // vermerkt, ob zuletzt ein Leerraum-Textknoten kam.
            let values = nodes.node_value.as_ref();
            let mut pending = vec![false; backend.len()];
            for (i, id) in backend.iter().enumerate() {
                let Some(parent) = parents.get(i).and_then(|p| usize::try_from(*p).ok()) else {
                    continue;
                };
                let blank = types.get(i) == Some(&TEXT_NODE)
                    && values
                        .and_then(|v| v.get(i))
                        .and_then(|v| string(*v.inner()))
                        .is_some_and(|t| t.trim().is_empty());
                if blank {
                    pending[parent] = true;
                } else if std::mem::take(&mut pending[parent]) {
                    layout.whitespace_before.insert(*id.inner());
                }
            }
            for (i, open) in pending.into_iter().enumerate() {
                if open {
                    layout.whitespace_last.insert(*backend[i].inner());
                }
            }
            for (i, id) in backend.iter().enumerate() {
                if types.get(i) != Some(&ELEMENT_NODE) {
                    continue;
                }
                let style = styles[i].take().unwrap_or_else(|| {
                    let display = if renders_below[i] { "contents" } else { "none" };
                    (display.to_string(), None)
                });
                layout.styles.insert(*id.inner(), style);
            }
        }
        layout
    }
}

/// Umkopieren des CDP-Baums in die DOM-Fakten.
struct Walk<'n> {
    layout: Layout,
    by_backend: HashMap<i64, &'n Node>,
}

impl Walk<'_> {
    fn node(&self, builder: DomFactsBuilder, node: &Node) -> DomFactsBuilder {
        match node.node_type {
            TEXT_NODE => builder.text(&node.node_value),
            ELEMENT_NODE if !NOT_RENDERED.contains(&node.local_name.as_str()) => {
                self.element(builder, node)
            }
            _ => builder,
        }
    }

    fn element(&self, builder: DomFactsBuilder, node: &Node) -> DomFactsBuilder {
        let backend = *node.backend_node_id.inner();
        let mut b = builder.open(&node.local_name, backend);
        // CDP liefert Attribute als flache Liste [Name, Wert, Name, Wert, …].
        for pair in node.attributes.as_deref().unwrap_or_default().chunks(2) {
            if let [name, value] = pair {
                if dom_attribute_needed(name) {
                    b = b.attr(name, value);
                }
            }
        }
        if let Some((display, visibility)) = self.layout.styles.get(&backend) {
            b = b.style(display, visibility.as_deref());
        }
        // iframe im selben Renderer-Prozess: eigenes Dokument. Frames in
        // einem anderen Prozess (Site Isolation) liefert `pierce` nicht mit.
        if let Some(html) = node.content_document.as_deref().and_then(html_of) {
            return self.node(b.frame(), html).end_frame().close();
        }
        // Der flache Baum, wie Browser und Assistenztechnik ihn sehen: Die
        // Kinder eines Shadow-Roots hängen unter dem Host, Light-DOM-Kinder
        // nur dort, wo ein `<slot>` sie aufnimmt. Shadow-Roots des Browsers
        // (Innenleben von `<input>`, `<textarea>` …) bleiben außen vor.
        let shadows: Vec<&Node> = node
            .shadow_roots
            .iter()
            .flatten()
            .filter(|s| s.shadow_root_type != Some(ShadowRootType::UserAgent))
            .collect();
        let assigned: Vec<&Node> = node
            .distributed_nodes
            .iter()
            .flatten()
            .filter_map(|n| self.by_backend.get(n.backend_node_id.inner()).copied())
            .collect();
        if !shadows.is_empty() {
            for shadow in shadows {
                b = self.children(b, shadow);
            }
        } else if !assigned.is_empty() {
            for child in assigned {
                b = self.node(b, child);
            }
        } else {
            // Auch ein `<slot>` ohne Zuweisung: Er zeigt seinen Ersatzinhalt.
            b = self.children(b, node);
        }
        b.close()
    }

    fn children(&self, mut b: DomFactsBuilder, parent: &Node) -> DomFactsBuilder {
        for child in parent.children.iter().flatten() {
            if self
                .layout
                .whitespace_before
                .contains(child.backend_node_id.inner())
            {
                b = b.text(" ");
            }
            b = self.node(b, child);
        }
        if self
            .layout
            .whitespace_last
            .contains(parent.backend_node_id.inner())
        {
            b = b.text(" ");
        }
        b
    }
}

/// Tab ab Dokumentanfang, bis der Fokus auf `body` fällt (Ende der Folge),
/// ein Element zum zweiten Mal kommt oder [`MAX_TABS`] erreicht ist.
async fn tab_walk(page: &Page, model: &SemanticGraph) -> Result<Vec<Option<NodeRef>>> {
    page.evaluate(TAB_START).await?;
    let body = backend_of(page, "document.body").await?;
    let mut seen = Vec::new();
    let mut sequence = Vec::new();
    for _ in 0..MAX_TABS {
        act::press_tab(page).await?;
        let focus = backend_of(page, "document.activeElement").await?;
        if focus == body || seen.contains(&focus) {
            break;
        }
        seen.push(focus);
        sequence.push(node_with_dom_id(model, focus));
    }
    page.evaluate(TAB_END).await?;
    Ok(sequence)
}

/// Backend-ID des Elements, zu dem `expression` auswertet.
async fn backend_of(page: &Page, expression: &str) -> Result<i64> {
    let eval = EvaluateParams::builder()
        .expression(expression)
        .build()
        .map_err(|e| anyhow!(e))?;
    let object_id = page
        .execute(eval)
        .await?
        .result
        .result
        .object_id
        .clone()
        .ok_or_else(|| anyhow!("{expression}: kein Element"))?;
    let described = page
        .execute(DescribeNodeParams::builder().object_id(object_id).build())
        .await?;
    Ok(*described.node.backend_node_id.inner())
}

fn node_with_dom_id(model: &SemanticGraph, dom: i64) -> Option<NodeRef> {
    model.trees.values().find_map(|t| {
        t.nodes
            .values()
            .find(|n| n.dom_node_id == Some(dom))
            .map(|n| NodeRef::new(t.id.clone(), n.id))
    })
}
