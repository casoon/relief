//! Formular-Zusicherungen (Aufgabenzeile `assert: …`): der Host erhebt die
//! Fakten, `relief_interaction::assertions` wertet sie browserfrei aus.
//!
//! - **Modell und Fokus**: aus der Session (Aufnahme wie bei `do:`), Fokus
//!   live abgefragt.
//! - **DOM-Fakten** (nur `namen-wie-accname`): `DOM.getDocument` des
//!   Hauptdokuments als `DomFacts`, nur die Attribute aus
//!   `dom_attribute_needed`; ohne iframes und Shadow DOM.
//! - **Tab-Folge** (nur `tabfolge`): echte Tab-Tasten ab Dokumentanfang, nach
//!   jeder Taste der Fokus. Wie Escape und Scrollen ohne `ActionPlan`: Tab hat
//!   kein Zielelement und bewegt nur den Fokus.

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{DescribeNodeParams, GetDocumentParams, Node};
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
            tab_sequence: tabs.as_deref(),
        },
    );
    Ok(assertions::render(&findings))
}

/// Hauptdokument als DOM-Fakten.
async fn dom_facts(page: &Page) -> Result<DomFacts> {
    // Wie `live.rs` (Tiefe -1, pierce): der DOM-Agent bleibt im selben Zustand.
    let document = page
        .execute(GetDocumentParams::builder().depth(-1).pierce(true).build())
        .await?;
    let html = document
        .result
        .root
        .children
        .iter()
        .flatten()
        .find(|n| n.node_type == 1)
        .ok_or_else(|| anyhow!("Dokument ohne Wurzelelement"))?;
    Ok(add(DomFacts::builder(), html).build())
}

fn add(builder: DomFactsBuilder, node: &Node) -> DomFactsBuilder {
    match node.node_type {
        3 => builder.text(&node.node_value),
        1 if !NOT_RENDERED.contains(&node.local_name.as_str()) => {
            let mut b = builder.open(&node.local_name, *node.backend_node_id.inner());
            // CDP liefert Attribute als flache Liste [Name, Wert, Name, Wert, …].
            for pair in node.attributes.as_deref().unwrap_or_default().chunks(2) {
                if let [name, value] = pair {
                    if dom_attribute_needed(name) {
                        b = b.attr(name, value);
                    }
                }
            }
            for child in node.children.iter().flatten() {
                b = add(b, child);
            }
            b.close()
        }
        _ => builder,
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
