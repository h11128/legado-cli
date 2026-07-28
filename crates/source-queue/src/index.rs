//! Pull list_sources from MCP → phone_source_index.json.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};
use source_mcp::{McpClient, McpEndpoint};
use source_types::PortError;

#[derive(Debug, Clone)]
pub struct RefreshIndexResult {
    pub path: PathBuf,
    pub total: usize,
    pub cache_hit: bool,
}

pub fn default_index_path(root: &Path) -> PathBuf {
    root.join("temp/full_fix/phone_source_index.json")
}

pub fn refresh_phone_index(out: Option<PathBuf>) -> Result<RefreshIndexResult, PortError> {
    let root = source_mcp::repo_root()?;
    let out = out.unwrap_or_else(|| default_index_path(&root));
    let ep = McpEndpoint::load_defaults()?;
    let client = Arc::new(McpClient::new(ep).with_client_name("source_queue_index"));
    client.ensure_session()?;

    let mut items: Vec<Value> = Vec::new();
    let mut offset = 0usize;
    let page = 200usize;
    loop {
        let result =
            client.tools_call("list_sources", json!({ "offset": offset, "limit": page }))?;
        let data = McpClient::parse_json_text(&McpClient::extract_text(&result));
        let chunk = data
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let total = data.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let chunk_len = chunk.len();
        items.extend(chunk);
        if chunk_len == 0 || (total > 0 && items.len() >= total) {
            break;
        }
        offset += chunk_len;
    }

    let mut by_url = serde_json::Map::new();
    for row in &items {
        let url = row
            .get("bookSourceUrl")
            .or_else(|| row.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if url.is_empty() {
            continue;
        }
        // Alias MCP field names so progress/rt_queue (group/name) keep working.
        let mut row = row.clone();
        if let Some(obj) = row.as_object_mut() {
            if !obj.contains_key("group") {
                if let Some(g) = obj.get("bookSourceGroup").cloned() {
                    obj.insert("group".into(), g);
                }
            }
            if !obj.contains_key("name") {
                if let Some(n) = obj.get("bookSourceName").cloned() {
                    obj.insert("name".into(), n);
                }
            }
        }
        by_url.insert(url.to_string(), row);
    }
    let payload = json!({
        "schema_version": 1,
        "total": by_url.len(),
        "by_url": by_url,
    });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(e.to_string()))?;
    }
    fs::write(
        &out,
        serde_json::to_string_pretty(&payload).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(e.to_string()))?;

    Ok(RefreshIndexResult {
        path: out,
        total: by_url.len(),
        cache_hit: false,
    })
}
