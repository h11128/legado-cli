//! Session fix index — parity with `repair_claim.py` (`append_index`, `assert_fixed_allowed`).

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};
use source_types::PortError;

const INDEX_FIELDS: &[&str] = &["url", "name", "evidence", "agent", "root_cause"];

fn empty_index() -> Value {
    json!({
        "session_id": "local",
        "verified_fixed": [],
        "unverified_claimed_fixed": [],
        "skipped": [],
        "failed": [],
    })
}

pub fn load_check_json(path: &Path) -> Result<Value, PortError> {
    if !path.is_file() {
        return Err(PortError::Permanent(format!(
            "check-json missing: {}",
            path.display()
        )));
    }
    let raw = fs::read_to_string(path).map_err(|e| PortError::Permanent(e.to_string()))?;
    let data: Value =
        serde_json::from_str(&raw).map_err(|e| PortError::Permanent(format!("json: {e}")))?;
    if !data.is_object() {
        return Err(PortError::Permanent("check-json must be an object".into()));
    }
    Ok(data)
}

/// Refuse status=fixed unless device verify evidence says success.
pub fn assert_fixed_allowed(check: Option<&Value>) -> Result<(), PortError> {
    let Some(check) = check else {
        return Err(PortError::Permanent(
            "Refuse status=fixed without --check-json from source verify".into(),
        ));
    };
    if check.get("success").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(());
    }
    if let Some(nested) = check.get("check").and_then(|v| v.as_object()) {
        if nested.get("final").and_then(|v| v.as_str()) == Some("pass") {
            return Ok(());
        }
    }
    if let Some(arr) = check.get("attempts").and_then(|v| v.as_array()) {
        if arr
            .iter()
            .any(|a| a.get("success").and_then(|v| v.as_bool()) == Some(true))
        {
            return Ok(());
        }
    }
    let msg = check
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("check-json success!=true");
    Err(PortError::Permanent(format!("Refuse status=fixed: {msg}")))
}

fn entry_item(entry: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    for key in INDEX_FIELDS {
        if let Some(v) = entry.get(*key) {
            if v.as_str().is_some_and(|s| s.is_empty()) {
                continue;
            }
            m.insert((*key).to_string(), v.clone());
        }
    }
    m
}

fn dedupe_url(bucket: &mut Vec<Value>, url: &str) {
    bucket.retain(|x| x.get("url").and_then(|v| v.as_str()) != Some(url));
}

/// Append/update session index row by status bucket.
pub fn append_index(index_path: &Path, entry: &Value) -> Result<Value, PortError> {
    let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let url = entry.get("url").and_then(|v| v.as_str()).unwrap_or("");
    if url.is_empty() {
        return Err(PortError::Permanent("index entry missing url".into()));
    }
    let mut data = if index_path.is_file() {
        let raw = fs::read_to_string(index_path)
            .map_err(|e| PortError::Permanent(format!("read index: {e}")))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| empty_index())
    } else {
        empty_index()
    };
    let item = Value::Object(entry_item(entry));
    match status {
        "fixed" => {
            if let Some(arr) = data
                .get_mut("verified_fixed")
                .and_then(|v| v.as_array_mut())
            {
                dedupe_url(arr, url);
                arr.push(item);
            }
            if let Some(arr) = data
                .get_mut("unverified_claimed_fixed")
                .and_then(|v| v.as_array_mut())
            {
                dedupe_url(arr, url);
            }
        }
        "skipped" => {
            if let Some(arr) = data.get_mut("skipped").and_then(|v| v.as_array_mut()) {
                arr.push(item);
            }
        }
        "failed" => {
            if let Some(arr) = data.get_mut("failed").and_then(|v| v.as_array_mut()) {
                arr.push(item);
            }
        }
        other => {
            return Err(PortError::Permanent(format!(
                "unknown index status: {other}"
            )));
        }
    }
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(e.to_string()))?;
    }
    fs::write(
        index_path,
        serde_json::to_string_pretty(&data).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(format!("write index: {e}")))?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fixed_requires_check() {
        assert!(assert_fixed_allowed(None).is_err());
        assert!(assert_fixed_allowed(Some(&json!({"success": false}))).is_err());
        assert!(assert_fixed_allowed(Some(&json!({"success": true}))).is_ok());
    }

    #[test]
    fn append_fixed_dedupes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("idx.json");
        append_index(
            &path,
            &json!({"status":"fixed","url":"https://a/","name":"A"}),
        )
        .unwrap();
        append_index(
            &path,
            &json!({"status":"fixed","url":"https://a/","name":"A2"}),
        )
        .unwrap();
        let data: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(data["verified_fixed"].as_array().unwrap().len(), 1);
    }
}
