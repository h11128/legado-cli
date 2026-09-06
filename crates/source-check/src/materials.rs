//! Fail-result classification + materials dump (Python `batch_check_mcp`).

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};
use source_types::PortError;

pub const FAIL_TAGS: &[&str] = &[
    "网站失效",
    "域名失效",
    "搜索失效",
    "发现失效",
    "校验超时",
    "js失效",
    "搜索目录失效",
    "发现目录失效",
    "搜索正文失效",
    "发现正文失效",
    "搜索链接规则为空",
    "发现规则为空",
];

/// Bucket check results by failure tag (group/message substring match).
pub fn classify_results(results: &[Value]) -> Map<String, Value> {
    let mut buckets: Map<String, Value> = Map::new();
    for tag in FAIL_TAGS {
        buckets.insert(tag.to_string(), json!([]));
    }
    buckets.insert("other_fail".into(), json!([]));
    buckets.insert("success".into(), json!([]));

    for item in results {
        let Some(obj) = item.as_object() else {
            continue;
        };
        if obj.get("success").and_then(|v| v.as_bool()) == Some(true) {
            push_bucket(&mut buckets, "success", item.clone());
            continue;
        }
        let group = obj
            .get("group")
            .or_else(|| obj.get("raw").and_then(|r| r.get("group")))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let message = obj
            .get("message")
            .or_else(|| obj.get("raw").and_then(|r| r.get("message")))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let hit = FAIL_TAGS
            .iter()
            .find(|tag| group.contains(**tag) || message.contains(**tag))
            .map(|s| (*s).to_string())
            .unwrap_or_else(|| "other_fail".into());
        push_bucket(&mut buckets, &hit, item.clone());
    }
    buckets.retain(|_, v| v.as_array().is_some_and(|a| !a.is_empty()));
    buckets
}

fn push_bucket(buckets: &mut Map<String, Value>, key: &str, item: Value) {
    let arr = buckets
        .entry(key.to_string())
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("bucket array");
    arr.push(item);
}

pub fn tag_counts(classified: &Map<String, Value>) -> Map<String, Value> {
    classified
        .iter()
        .map(|(k, v)| {
            let n = v.as_array().map(|a| a.len()).unwrap_or(0);
            (k.clone(), json!(n))
        })
        .collect()
}

fn safe_filename(url: &str) -> String {
    url.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '_') {
                ch
            } else {
                '_'
            }
        })
        .take(120)
        .collect()
}

/// Write per-tag JSON files + summary.tsv (skips success bucket).
pub fn dump_fail_materials(
    classified: &Map<String, Value>,
    out_dir: &Path,
) -> Result<(), PortError> {
    fs::create_dir_all(out_dir)
        .map_err(|e| PortError::Permanent(format!("mkdir materials: {e}")))?;
    let mut summary = String::new();
    for (tag, items) in classified {
        if tag == "success" {
            continue;
        }
        let Some(arr) = items.as_array() else {
            continue;
        };
        let tag_dir = out_dir.join(tag.replace('/', "_"));
        fs::create_dir_all(&tag_dir)
            .map_err(|e| PortError::Permanent(format!("mkdir tag: {e}")))?;
        summary.push_str(&format!("{tag}\t{}\n", arr.len()));
        for item in arr {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let path = tag_dir.join(format!("{}.json", safe_filename(url)));
            let body = serde_json::to_string_pretty(item)
                .map_err(|e| PortError::Permanent(e.to_string()))?;
            fs::write(&path, body)
                .map_err(|e| PortError::Permanent(format!("write material: {e}")))?;
        }
    }
    fs::write(out_dir.join("summary.tsv"), summary)
        .map_err(|e| PortError::Permanent(format!("write summary: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_search_fail() {
        let rows = vec![json!({
            "url": "https://a/",
            "success": false,
            "message": "校验失败:搜索失效",
        })];
        let c = classify_results(&rows);
        assert!(c.contains_key("搜索失效"));
    }
}
