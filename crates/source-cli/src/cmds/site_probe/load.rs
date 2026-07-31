//! Load helpers for `site-probe`.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(super) struct Cand {
    pub id: String,
    pub url: String,
    pub kind: String,
    pub note: String,
    pub status: String,
}

pub(super) fn default_rules_path() -> PathBuf {
    let candidates = [
        PathBuf::from("config/verify_skip_rules.json"),
        PathBuf::from("../config/verify_skip_rules.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json"),
    ];
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json")
}

pub(super) fn preset_path(name: &str) -> PathBuf {
    let file = match name {
        "publish" => "site_candidates_publish.json",
        other => other,
    };
    let candidates = [
        PathBuf::from("config").join(file),
        PathBuf::from("../config").join(file),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../config")
            .join(file),
    ];
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("config").join(file)
}

pub(super) fn load_preset(path: &Path) -> Result<Vec<Cand>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let v: Value =
        serde_json::from_str(&raw).map_err(|e| format!("json {}: {e}", path.display()))?;
    let arr = v
        .get("candidates")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "preset missing candidates[]".to_string())?;
    let mut out = Vec::new();
    for c in arr {
        let url = c
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if url.is_empty() {
            continue;
        }
        out.push(Cand {
            id: c.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
            url,
            kind: c.get("kind").and_then(|x| x.as_str()).unwrap_or("").into(),
            note: c.get("note").and_then(|x| x.as_str()).unwrap_or("").into(),
            status: c
                .get("status")
                .and_then(|x| x.as_str())
                .unwrap_or("candidate")
                .into(),
        });
    }
    Ok(out)
}

pub(super) fn load_urls_file(path: &Path) -> Result<Vec<Cand>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '|').collect();
        out.push(Cand {
            id: format!("u{i}"),
            url: parts[0].trim().to_string(),
            kind: parts.get(1).map(|s| s.trim().into()).unwrap_or_default(),
            note: parts.get(2).map(|s| s.trim().into()).unwrap_or_default(),
            status: "candidate".into(),
        });
    }
    Ok(out)
}
