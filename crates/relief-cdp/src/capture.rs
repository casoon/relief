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
use chromiumoxide::cdp::browser_protocol::dom::{DescribeNodeParams, GetFrameOwnerParams};
use chromiumoxide::cdp::browser_protocol::page::{FrameTree, GetFrameTreeParams};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::Page;
use serde_json::Value;

pub struct Capture {
    pub snapshot: AXSnapshot,
    /// Dauer von `getFullAXTree` inklusive Umwandlung und iframes.
    pub tree_ms: u128,
    /// Eingehängte iframes / nicht erreichbare iframes.
    pub frames: (usize, usize),
}

pub async fn capture(page: &Page, label: &str) -> Result<Capture> {
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
    let frames = attach_frames(page, &mut nodes).await.unwrap_or((0, 0));
    drop_palette(&mut nodes);
    let tree = AXTree::from_nodes(nodes);
    let tree_ms = started.elapsed().as_millis();

    let url = page.url().await?.unwrap_or_default();
    let title = page.get_title().await?.unwrap_or_default();
    let focus = focus(page, &tree).await.unwrap_or_default();

    Ok(Capture {
        snapshot: AXSnapshot::new(label, url, title, now_ms(), tree, focus),
        tree_ms,
        frames,
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

/// `getFullAXTree` liefert nur den Hauptframe. Die Bäume der iframes werden
/// einzeln geholt und unter ihrem `iframe`-Knoten eingehängt; ihre Knoten-IDs
/// bekommen ein Frame-Präfix, weil Chrome sie je Frame vergibt.
///
/// Frames in einem anderen Renderer-Prozess (Site Isolation) antworten hier
/// nicht; sie werden gezählt, nicht verschwiegen. Der Spike startet Chrome
/// deshalb mit `--disable-site-isolation-trials`.
async fn attach_frames(page: &Page, nodes: &mut Vec<AXNode>) -> Result<(usize, usize)> {
    let tree = page
        .execute(GetFrameTreeParams::default())
        .await?
        .result
        .frame_tree;
    let mut stack: Vec<FrameTree> = tree.child_frames.unwrap_or_default();
    let (mut attached, mut failed) = (0, 0);
    while let Some(frame) = stack.pop() {
        stack.extend(frame.child_frames.clone().unwrap_or_default());
        let id = frame.frame.id.clone();
        let Ok(owner) = page.execute(GetFrameOwnerParams::new(id.clone())).await else {
            failed += 1;
            continue;
        };
        let owner_backend = *owner.result.backend_node_id.inner();
        let Ok(resp) = page
            .execute(GetFullAxTreeParams::builder().frame_id(id.clone()).build())
            .await
        else {
            failed += 1;
            continue;
        };
        let prefix = format!("f{}", attached + failed + 1);
        let mut frame_nodes: Vec<AXNode> = serde_json::to_value(&resp.result.nodes)?
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
        }
        let Some(owner_node) = nodes
            .iter_mut()
            .find(|n| n.backend_dom_node_id == Some(owner_backend))
        else {
            failed += 1;
            continue;
        };
        if let Some(root) = frame_nodes.first_mut() {
            root.parent_id = Some(owner_node.node_id.clone());
            owner_node.child_ids.push(root.node_id.clone());
        }
        nodes.extend(frame_nodes);
        attached += 1;
    }
    Ok((attached, failed))
}

pub(crate) async fn focus(page: &Page, tree: &AXTree) -> Result<FocusSnapshot> {
    let eval = EvaluateParams::builder()
        .expression("document.activeElement")
        .build()
        .map_err(|e| anyhow!(e))?;
    let result = page.execute(eval).await?;
    let object_id = result
        .result
        .result
        .object_id
        .clone()
        .ok_or_else(|| anyhow!("kein activeElement"))?;
    let described = page
        .execute(DescribeNodeParams::builder().object_id(object_id).build())
        .await?;
    let backend = *described.node.backend_node_id.inner();
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
