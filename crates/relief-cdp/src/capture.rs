//! Aufnahme über CDP: nativer AXTree + Fokus → `AXSnapshot`.
//!
//! Die Umwandlung des CDP-JSON entspricht `auditmysite/src/accessibility/extractor.rs`
//! (dort gemessen und korrigiert, u. a. Name-Source `title`).

use std::time::{Duration, Instant};

use a11y_perception::{
    AXNode, AXProperty, AXSnapshot, AXTree, AXValue, FocusSnapshot, NameSource, RelatedNode,
};
use anyhow::{anyhow, Context, Result};
use chromiumoxide::cdp::browser_protocol::accessibility::GetFullAxTreeParams;
use chromiumoxide::cdp::browser_protocol::dom::{
    BackendNodeId, DescribeNodeParams, GetFrameOwnerParams,
};
use chromiumoxide::cdp::browser_protocol::page::{FrameTree, GetFrameTreeParams};
use chromiumoxide::Page;
use serde_json::Value;

use crate::frames::{encode, Doc, Frames};

pub struct Capture {
    pub snapshot: AXSnapshot,
    /// Dauer von `getFullAXTree` inklusive Umwandlung und iframes.
    pub tree_ms: u128,
    pub frames: FrameCount,
}

/// iframes je Weg, für `measure`.
#[derive(Debug, Default)]
pub struct FrameCount {
    /// Im Prozess des Elterndokuments (`getFullAXTree { frameId }` in dessen
    /// Sitzung): eingehängt / nicht erreichbar.
    pub local: (usize, usize),
    /// In einem anderen Prozess (eigene Sitzung, `frames.rs`): eingehängt /
    /// nicht erreichbar.
    pub remote: (usize, usize),
    /// Je nicht erreichbarem iframe: Weg, Frame und Fehler.
    pub unreachable: Vec<String>,
}

pub async fn capture(page: &Page, frames: &Frames, label: &str) -> Result<Capture> {
    let started = Instant::now();
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        page.execute(GetFullAxTreeParams::default()),
    )
    .await
    .context("getFullAXTree: Zeitüberschreitung")??;
    let json = serde_json::to_value(&response.nodes)?;
    let mut nodes: Vec<AXNode> = json
        .as_array()
        .ok_or_else(|| anyhow!("getFullAXTree ohne nodes"))?
        .iter()
        .map(convert_node)
        .collect();
    let mut count = FrameCount::default();
    attach_frames(&Doc::Page(page), "f", &mut nodes, &mut count)
        .await
        .ok();
    attach_remote_frames(page, frames, &mut nodes, &mut count).await;
    drop_palette(&mut nodes);
    let tree = AXTree::from_nodes(nodes);
    let tree_ms = started.elapsed().as_millis();

    let url = page.url().await?.unwrap_or_default();
    let title = page.get_title().await?.unwrap_or_default();
    let focus = focus(page, frames, &tree).await.unwrap_or_default();

    Ok(Capture {
        snapshot: AXSnapshot::new(label, url, title, now_ms(), tree, focus),
        tree_ms,
        frames: count,
    })
}

/// Namen der eingefügten Befehlsleiste und ihres Ergebnishinweises
/// (`palette.js`). Sie gehören nicht zur Seite und dürfen weder im Graph noch
/// im Diff auftauchen.
const PALETTE_NAMES: &[&str] = &["Relief-Befehlsleiste", "Relief-Ergebnis"];

fn drop_palette(nodes: &mut Vec<AXNode>) {
    for name in PALETTE_NAMES {
        drop_subtree_named(nodes, name);
    }
}

fn drop_subtree_named(nodes: &mut Vec<AXNode>, name: &str) {
    let Some(root) = nodes.iter().find(|n| n.name.as_deref() == Some(name)) else {
        return;
    };
    // Vom Dialog aufwärts bis zum `relief-palette`-Element — nur durch
    // Knoten mit genau einem Kind, damit nie ein Vorfahr der Seite mitfällt.
    let mut top = root.node_id.clone();
    let by_id = |id: &str| nodes.iter().find(|n| n.node_id == id);
    while let Some(parent) = by_id(&top)
        .and_then(|n| n.parent_id.clone())
        .and_then(|p| by_id(&p))
    {
        if parent.child_ids.len() != 1 || parent.role.as_deref() == Some("RootWebArea") {
            break;
        }
        top = parent.node_id.clone();
    }
    let mut remove = std::collections::HashSet::from([top.clone()]);
    let mut stack = vec![top];
    while let Some(id) = stack.pop() {
        if let Some(n) = nodes.iter().find(|n| n.node_id == id) {
            for c in &n.child_ids {
                if remove.insert(c.clone()) {
                    stack.push(c.clone());
                }
            }
        }
    }
    nodes.retain(|n| !remove.contains(&n.node_id));
    for n in nodes.iter_mut() {
        n.child_ids.retain(|c| !remove.contains(c));
    }
}

/// `getFullAXTree` liefert nur den Frame der Sitzung. Die Bäume der iframes
/// im selben Prozess werden einzeln geholt und unter ihrem `iframe`-Knoten
/// eingehängt; ihre Knoten-IDs bekommen ein Frame-Präfix (`prefix` und
/// laufende Nummer), weil Chrome sie je Frame vergibt. `doc` ist die Seite
/// oder ein Frame in einem anderen Prozess (dann Backend-IDs über
/// [`encode`], auch die des Besitzers).
///
/// Frames in einem anderen Renderer-Prozess (Site Isolation) kennt der
/// Frame-Baum der Sitzung nicht; sie hängt [`attach_remote_frames`] ein.
async fn attach_frames(
    doc: &Doc<'_>,
    prefix: &str,
    nodes: &mut Vec<AXNode>,
    count: &mut FrameCount,
) -> Result<()> {
    let tree = doc.execute(GetFrameTreeParams::default()).await?.frame_tree;
    let index = doc.index();
    let mut stack: Vec<FrameTree> = tree.child_frames.unwrap_or_default();
    let mut n = 0;
    while let Some(frame) = stack.pop() {
        stack.extend(frame.child_frames.clone().unwrap_or_default());
        n += 1;
        let id = frame.frame.id.clone();
        let mut unreachable = |why: String| {
            count.local.1 += 1;
            count
                .unreachable
                .push(format!("im Prozess, {}: {why}", frame.frame.url));
        };
        let owner = match doc.execute(GetFrameOwnerParams::new(id.clone())).await {
            Ok(owner) => owner,
            Err(e) => {
                unreachable(format!("getFrameOwner: {e}"));
                continue;
            }
        };
        let owner_backend = encode(index, *owner.backend_node_id.inner());
        let resp = match doc
            .execute(GetFullAxTreeParams::builder().frame_id(id.clone()).build())
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                unreachable(format!("getFullAXTree: {e}"));
                continue;
            }
        };
        let prefix = format!("{prefix}{n}");
        let mut frame_nodes: Vec<AXNode> = serde_json::to_value(&resp.nodes)?
            .as_array()
            .map(|a| a.iter().map(convert_node).collect())
            .unwrap_or_default();
        for n in &mut frame_nodes {
            n.node_id = format!("{prefix}:{}", n.node_id);
            n.parent_id = n.parent_id.as_ref().map(|p| format!("{prefix}:{p}"));
            n.child_ids = n
                .child_ids
                .iter()
                .map(|c| format!("{prefix}:{c}"))
                .collect();
            renumber(n, index);
        }
        let Some(owner_node) = nodes
            .iter_mut()
            .find(|n| n.backend_dom_node_id == Some(owner_backend))
        else {
            unreachable("iframe-Element nicht im AXTree".into());
            continue;
        };
        if let Some(root) = frame_nodes.first_mut() {
            root.parent_id = Some(owner_node.node_id.clone());
            owner_node.child_ids.push(root.node_id.clone());
        }
        nodes.extend(frame_nodes);
        count.local.0 += 1;
    }
    Ok(())
}

/// Frames in einem anderen Renderer-Prozess (Site Isolation): Ein
/// `iframe`-Knoten ohne Kinder, dessen Element eine `frameId` trägt, ist ein
/// eigenes Ziel. Dessen Baum kommt über die Sitzung des Frames
/// (`frames.rs`), Knoten-IDs mit Präfix `r<Nummer>`, Backend-IDs über
/// [`encode`]; iframes im Prozess dieses Frames hängt [`attach_frames`] in
/// dessen Sitzung ein (Präfix `r<Nummer>f<n>`). Eingehängte Knoten werden
/// weiter durchsucht, so kommen auch Frames in Frames dazu.
async fn attach_remote_frames(
    page: &Page,
    frames: &Frames,
    nodes: &mut Vec<AXNode>,
    count: &mut FrameCount,
) {
    let mut i = 0;
    while i < nodes.len() {
        let owner = &nodes[i];
        i += 1;
        if !matches!(
            owner.role.as_deref(),
            Some("Iframe" | "IframePresentational")
        ) || !owner.child_ids.is_empty()
        {
            continue;
        }
        let (Some(owner_backend), owner_id) = (owner.backend_dom_node_id, owner.node_id.clone())
        else {
            continue;
        };
        let owner_name = owner.name.clone().unwrap_or_default();
        let Ok((doc, backend)) = frames.resolve(page, owner_backend) else {
            continue;
        };
        let described = doc
            .execute(
                DescribeNodeParams::builder()
                    .backend_node_id(BackendNodeId::new(backend))
                    .build(),
            )
            .await;
        // Ohne `frameId` hat das iframe (noch) kein Dokument.
        let Some(frame_id) = described.ok().and_then(|d| d.node.frame_id) else {
            continue;
        };
        let mut unreachable = |why: String| {
            count.remote.1 += 1;
            count.unreachable.push(format!(
                "eigene Sitzung, iframe „{owner_name}“ (Frame {}): {why}",
                frame_id.as_ref()
            ));
        };
        let frame = match frames.attach(frame_id.as_ref()).await {
            Ok(frame) => frame,
            Err(e) => {
                unreachable(format!("anhängen: {e}"));
                continue;
            }
        };
        let response = match Doc::Frame(frames, frame.clone())
            .execute(GetFullAxTreeParams::default())
            .await
        {
            Ok(response) => response,
            Err(e) => {
                unreachable(format!("getFullAXTree: {e}"));
                continue;
            }
        };
        let prefix = format!("r{}", frame.index);
        let json = match serde_json::to_value(&response.nodes) {
            Ok(json) => json,
            Err(e) => {
                unreachable(format!("Knoten: {e}"));
                continue;
            }
        };
        let mut frame_nodes: Vec<AXNode> = json
            .as_array()
            .map(|a| a.iter().map(convert_node).collect())
            .unwrap_or_default();
        for n in &mut frame_nodes {
            n.node_id = format!("{prefix}:{}", n.node_id);
            n.parent_id = n.parent_id.as_ref().map(|p| format!("{prefix}:{p}"));
            n.child_ids = n
                .child_ids
                .iter()
                .map(|c| format!("{prefix}:{c}"))
                .collect();
            renumber(n, frame.index);
        }
        if let Some(root) = frame_nodes.first_mut() {
            root.parent_id = Some(owner_id.clone());
            if let Some(owner) = nodes.iter_mut().find(|n| n.node_id == owner_id) {
                owner.child_ids.push(root.node_id.clone());
            }
        }
        nodes.extend(frame_nodes);
        count.remote.0 += 1;
        attach_frames(
            &Doc::Frame(frames, frame.clone()),
            &format!("{prefix}f"),
            nodes,
            count,
        )
        .await
        .ok();
    }
}

/// Backend-IDs eines Knotens aus einem Frame in einem anderen Prozess,
/// auch in Beziehungen (`labelledby` u. a.), über [`encode`].
fn renumber(node: &mut AXNode, index: i64) {
    node.backend_dom_node_id = node.backend_dom_node_id.map(|b| encode(index, b));
    for p in node.properties.iter_mut() {
        if let AXValue::Node { related_nodes } = &mut p.value {
            for r in related_nodes {
                r.backend_dom_node_id = r.backend_dom_node_id.map(|b| encode(index, b));
            }
        }
    }
}

/// Fokus als Backend-ID im Modell, auch in Frames anderer Prozesse
/// (`Frames::active_element`).
pub(crate) async fn focus(page: &Page, frames: &Frames, tree: &AXTree) -> Result<FocusSnapshot> {
    let backend = frames.active_element(page).await?;
    Ok(FocusSnapshot {
        active_backend_node_id: Some(backend),
        ax_node_id: tree.node_by_backend_id(backend).map(|n| n.node_id.clone()),
        ..Default::default()
    })
}

pub(crate) fn convert_node(json: &Value) -> AXNode {
    let name_source = json["name"]["sources"].as_array().and_then(|sources| {
        sources.iter().find_map(|s| {
            if s["value"].is_null() {
                return None;
            }
            match s["type"].as_str()? {
                "attribute" if s["attribute"].as_str() == Some("title") => Some(NameSource::Title),
                "attribute" => Some(NameSource::Attribute),
                "relatedElement" => Some(NameSource::RelatedElement),
                "contents" => Some(NameSource::Contents),
                "placeholder" => Some(NameSource::Placeholder),
                "title" => Some(NameSource::Title),
                _ => None,
            }
        })
    });
    let props = |key: &str| -> Vec<AXProperty> {
        json[key]
            .as_array()
            .map(|props| {
                props
                    .iter()
                    .filter_map(|p| {
                        Some(AXProperty {
                            name: p["name"].as_str()?.to_string(),
                            value: convert_value(&p["value"])?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    AXNode {
        node_id: json["nodeId"].as_str().unwrap_or_default().to_string(),
        ignored: json["ignored"].as_bool().unwrap_or(false),
        ignored_reasons: props("ignoredReasons"),
        role: json["role"]["value"].as_str().map(String::from),
        name: json["name"]["value"].as_str().map(String::from),
        name_source,
        description: json["description"]["value"].as_str().map(String::from),
        value: json["value"]["value"]
            .as_str()
            .map(String::from)
            .or_else(|| json["value"]["value"].as_f64().map(|n| n.to_string())),
        properties: props("properties"),
        child_ids: json["childIds"]
            .as_array()
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        parent_id: json["parentId"].as_str().map(String::from),
        backend_dom_node_id: json["backendDOMNodeId"].as_i64(),
    }
}

fn convert_value(json: &Value) -> Option<AXValue> {
    if let Some(related) = json["relatedNodes"].as_array() {
        let nodes: Vec<RelatedNode> = related
            .iter()
            .map(|n| RelatedNode {
                backend_dom_node_id: n["backendDOMNodeId"].as_i64(),
                idref: n["idref"].as_str().map(String::from),
                text: n["text"].as_str().map(String::from),
            })
            .collect();
        if !nodes.is_empty() {
            return Some(AXValue::Node {
                related_nodes: nodes,
            });
        }
    }
    let value = &json["value"];
    Some(match value {
        Value::Null => return None,
        Value::Bool(b) => AXValue::Bool(*b),
        Value::Number(n) if n.is_i64() => AXValue::Int(n.as_i64()?),
        Value::Number(n) => AXValue::Float(n.as_f64()?),
        Value::String(s) => AXValue::String(s.clone()),
        other => AXValue::String(other.to_string()),
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
