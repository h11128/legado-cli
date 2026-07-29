//! Progress goal tracking — parity with `repair_progress.py --goal`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use source_mcp::{default_jsonl_path, repo_root};

pub fn progress_json_path() -> PathBuf {
    repo_root()
        .map(|r| r.join("temp/full_fix/repair_progress.json"))
        .unwrap_or_else(|_| PathBuf::from("temp/full_fix/repair_progress.json"))
}

pub fn count_fixed(ledger_path: Option<&Path>) -> usize {
    let path = ledger_path
        .map(Path::to_path_buf)
        .or_else(|| default_jsonl_path().ok())
        .unwrap_or_else(|| PathBuf::from("temp/full_fix/repair_session_ledger.jsonl"));
    let Ok(raw) = std::fs::read_to_string(path) else {
        return 0;
    };
    let mut fixed = HashSet::new();
    for line in raw.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let step = row.get("step").and_then(|v| v.as_str()).unwrap_or("");
        let result = row.get("result").and_then(|v| v.as_str()).unwrap_or("");
        let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
        if url.is_empty() || step != "check" {
            continue;
        }
        if result.contains("校验成功") || result.starts_with("fixed") {
            // Do not inflate goal with hedged fake success rows.
            if source_closeout::ledger_result_blocked(result).is_some() {
                continue;
            }
            fixed.insert(url.to_string());
        }
    }
    fixed.len()
}

pub fn goal_status(goal: usize, ledger_path: Option<&Path>) -> Value {
    let fixed = count_fixed(ledger_path);
    let path = progress_json_path();
    let mut doc = if path.is_file() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("goal".into(), json!(goal));
        obj.insert("fixed_count".into(), json!(fixed));
        obj.insert(
            "remaining".into(),
            json!(goal.saturating_sub(fixed.min(goal))),
        );
        obj.insert(
            "updated".into(),
            json!(chrono::Utc::now().to_rfc3339()),
        );
    }
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).unwrap_or_default(),
    );
    json!({
        "goal": goal,
        "fixed_count": fixed,
        "remaining": goal.saturating_sub(fixed.min(goal)),
        "progress_path": path.to_string_lossy(),
    })
}
