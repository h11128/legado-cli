//! RespondTime-sorted repair queue from phone index.

use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};
use source_mcp::repo_root;
use source_types::PortError;

#[derive(Debug, Clone)]
pub struct RtQueueItem {
    pub url: String,
    pub respond_time: i64,
    pub meta: Value,
}

pub fn build_rt_queue(
    index_path: &Path,
    group_contains: &str,
) -> Result<Vec<RtQueueItem>, PortError> {
    let raw = std::fs::read_to_string(index_path)
        .map_err(|e| PortError::Permanent(format!("read index: {e}")))?;
    let index: Value =
        serde_json::from_str(&raw).map_err(|e| PortError::Permanent(format!("json: {e}")))?;
    let mut out = Vec::new();
    if let Some(by) = index.get("by_url").and_then(|v| v.as_object()) {
        for (url, meta) in by {
            let group = meta
                .get("group")
                .or_else(|| meta.get("bookSourceGroup"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !group.contains(group_contains) {
                continue;
            }
            if meta.get("enabled") == Some(&json!(false)) {
                continue;
            }
            let rt = meta
                .get("respondTime")
                .and_then(|v| v.as_i64())
                .unwrap_or(999_999);
            out.push(RtQueueItem {
                url: url.clone(),
                respond_time: rt,
                meta: meta.clone(),
            });
        }
    }
    out.sort_by_key(|i| i.respond_time);
    Ok(out)
}

/// Write RT queue JSON for `progress next` (`items` array, respondTime order).
pub fn write_rt_queue(
    out_path: &Path,
    items: &[RtQueueItem],
    limit: usize,
) -> Result<Value, PortError> {
    let trimmed: Vec<Value> = items
        .iter()
        .take(limit)
        .map(|i| {
            let name = i
                .meta
                .get("name")
                .or_else(|| i.meta.get("bookSourceName"))
                .cloned()
                .unwrap_or(json!(null));
            let group = i
                .meta
                .get("group")
                .or_else(|| i.meta.get("bookSourceGroup"))
                .cloned()
                .unwrap_or(json!(null));
            json!({
                "url": i.url,
                "respondTime": i.respond_time,
                "name": name,
                "group": group,
            })
        })
        .collect();
    let doc = json!({
        "items": trimmed,
        "total": items.len(),
        "written": trimmed.len(),
        "generated_at": Utc::now().to_rfc3339(),
    });
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PortError::Permanent(format!("mkdir queue: {e}")))?;
    }
    let raw = serde_json::to_string_pretty(&doc)
        .map_err(|e| PortError::Permanent(format!("json encode: {e}")))?;
    std::fs::write(out_path, raw).map_err(|e| PortError::Permanent(format!("write queue: {e}")))?;
    Ok(doc)
}

pub fn default_serial_queue_path() -> Result<std::path::PathBuf, PortError> {
    Ok(repo_root()?.join("temp/full_fix/queues/repair_serial100_queue.json"))
}
