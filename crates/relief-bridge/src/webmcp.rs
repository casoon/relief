//! Werkzeuge aus der CDP-Domäne `WebMCP` ins Modell (Paket 143).
//!
//! Der Fork hört je Tab über eine eigene DevTools-Sitzung auf
//! `WebMCP.toolsAdded`/`toolsRemoved` und reicht je Dokument die ganze
//! Liste weiter (`Tool`-Objekte, `WebMCP.pdl` in Chromium 154). Was die
//! Website über ein Werkzeug sagt, übernimmt Relief unverändert als ihre
//! Behauptung ([`relief_model::DeclaredTool`]).

use relief_model::DeclaredTool;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdpTool {
    name: String,
    #[serde(default)]
    description: String,
    input_schema: Option<serde_json::Value>,
    #[serde(default)]
    annotations: CdpAnnotations,
    backend_node_id: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CdpAnnotations {
    #[serde(default)]
    read_only: bool,
    #[serde(default)]
    untrusted_content: bool,
    #[serde(default)]
    consequential: bool,
}

/// `Tool`-Objekte (JSON-Array) → Werkzeuge. Fehler: nicht lesbar.
pub fn parse_tools(json: &str) -> Result<Vec<DeclaredTool>, String> {
    let tools: Vec<CdpTool> =
        serde_json::from_str(json).map_err(|e| format!("WebMCP-Werkzeuge nicht lesbar: {e}"))?;
    Ok(tools
        .into_iter()
        .map(|t| DeclaredTool {
            name: t.name,
            description: t.description,
            input_schema: t.input_schema.map(|s| s.to_string()),
            read_only: t.annotations.read_only,
            untrusted_content: t.annotations.untrusted_content,
            consequential: t.annotations.consequential,
            dom_node_id: t.backend_node_id,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn werkzeuge_aus_cdp() {
        let tools = parse_tools(
            r#"[{"name":"suche","description":"Sucht.","frameId":"F1",
                 "inputSchema":{"type":"object","properties":{"q":{"type":"string"}}},
                 "annotations":{"readOnly":true}},
                {"name":"buchen","description":"Bucht.","frameId":"F1","backendNodeId":7,
                 "annotations":{"consequential":true,"autosubmit":true}}]"#,
        )
        .unwrap();
        assert_eq!(tools.len(), 2);
        assert!(tools[0].read_only && !tools[0].consequential);
        assert!(tools[0].input_schema.as_deref().unwrap().contains("\"q\""));
        assert!(tools[1].consequential && !tools[1].read_only);
        assert_eq!(tools[1].dom_node_id, Some(7));
        assert!(parse_tools("{}").is_err());
    }
}
