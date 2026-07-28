//! Full RT queue build — parity with `repair_rt_queue.py`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};
use source_mcp::{default_jsonl_path, repo_root};
use source_types::PortError;
use url::Url;

const DEAD_GROUP: &[&str] = &["网站失效", "域名失效"];
const SEARCH_HINTS: &[&str] = &["搜索失效", "搜索目录失效", "搜索正文失效"];

const DEAD_SKIP_PREFIXES: &[&str] = &[
    "l2_",
    "l1_",
    "missing:",
    "search_endpoint_dead",
    "known_auth",
    "captcha",
    "api_signature",
    "migrate_target_dead",
    "non_book",
    "repurposed",
    "waf_",
    "dead_site",
    "why_title",
    "bad_bookSourceUrl",
];

#[derive(Debug, Clone)]
pub struct RtBuildOpts {
    pub max_rt_ms: i64,
    pub limit: usize,
    pub enabled_only: bool,
    pub search_tag_only: bool,
    pub all_sources_path: Option<std::path::PathBuf>,
    pub ledger_path: Option<std::path::PathBuf>,
}

fn norm(u: &str) -> String {
    u.trim().to_string()
}

fn host_key(url: &str) -> String {
    let n = norm(url);
    let raw = n.split("##").next().unwrap_or("").split('#').next().unwrap_or("");
    let with = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    Url::parse(&with)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_string()
}

fn with_scheme(u: &str) -> String {
    let u = norm(u);
    if u.is_empty() {
        return u;
    }
    if !u.split("##").next().unwrap_or("").contains("://") {
        format!("http://{}", u.trim_start_matches('/'))
    } else {
        u
    }
}

fn ledger_sets(path: &Path) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let mut fixed = HashSet::new();
    let mut hard = HashSet::new();
    let mut retryable = HashSet::new();
    let mut last_skip: HashMap<String, String> = HashMap::new();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return (fixed, hard, retryable);
    };
    for line in raw.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let u = norm(row.get("url").and_then(|v| v.as_str()).unwrap_or(""));
        if u.is_empty() {
            continue;
        }
        let result = row.get("result").and_then(|v| v.as_str()).unwrap_or("");
        let step = row.get("step").and_then(|v| v.as_str()).unwrap_or("");
        if step == "check"
            && (result.contains("校验成功") || result.starts_with("fixed") || result.starts_with("fixed:"))
        {
            fixed.insert(u.clone());
        }
        if step == "skip"
            || result.starts_with("skip:")
            || result.starts_with("repurposed:")
            || result.starts_with("disable:")
            || result.starts_with("disable")
        {
            last_skip.insert(u, result.to_string());
        }
    }
    for (u, reason) in last_skip {
        if fixed.contains(&u) {
            continue;
        }
        if DEAD_SKIP_PREFIXES.iter().any(|p| reason.starts_with(p)) {
            hard.insert(u);
        } else if reason.contains("no_patch") || reason.contains("搜索") || reason.contains("verify_fail") {
            retryable.insert(u);
        } else {
            hard.insert(u);
        }
    }
    (fixed, hard, retryable)
}

fn load_rt_map(path: &Path) -> HashMap<String, i64> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(data) = serde_json::from_str::<Value>(&raw) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    if let Some(arr) = data.get("data").and_then(|v| v.as_array()) {
        for s in arr {
            let Some(obj) = s.as_object() else {
                continue;
            };
            let u = norm(obj.get("bookSourceUrl").and_then(|v| v.as_str()).unwrap_or(""));
            if u.is_empty() {
                continue;
            }
            if let Some(rt) = obj.get("respondTime").and_then(|v| v.as_i64()) {
                out.insert(u, rt);
            }
        }
    }
    out
}

pub fn build_rt_queue_full(index_path: &Path, opts: &RtBuildOpts) -> Result<Value, PortError> {
    let raw = std::fs::read_to_string(index_path)
        .map_err(|e| PortError::Permanent(format!("read index: {e}")))?;
    let phone: Value =
        serde_json::from_str(&raw).map_err(|e| PortError::Permanent(format!("json: {e}")))?;
    let by_url = phone.get("by_url").and_then(|v| v.as_object()).ok_or_else(|| {
        PortError::Permanent("phone index missing by_url".into())
    })?;

    let rt_map = opts
        .all_sources_path
        .as_ref()
        .filter(|p| p.is_file())
        .map(|p| load_rt_map(p))
        .unwrap_or_default();

    let ledger_path = opts
        .ledger_path
        .clone()
        .or_else(|| default_jsonl_path().ok())
        .unwrap_or_else(|| repo_root().unwrap_or_default().join("temp/full_fix/repair_session_ledger.jsonl"));
    let (fixed, hard_skipped, retryable) = if ledger_path.is_file() {
        ledger_sets(&ledger_path)
    } else {
        (HashSet::new(), HashSet::new(), HashSet::new())
    };
    let blocked_hosts: HashSet<String> = fixed
        .iter()
        .chain(hard_skipped.iter())
        .map(|u| host_key(u))
        .filter(|h| !h.is_empty())
        .collect();

    let mut rows = Vec::new();
    let mut seen_hosts = HashSet::new();
    for (u, meta) in by_url {
        let Some(meta) = meta.as_object() else {
            continue;
        };
        let url = with_scheme(u);
        let hk = host_key(&url);
        if url.is_empty() || hk.is_empty() {
            continue;
        }
        if fixed.contains(u) || fixed.contains(&url) || blocked_hosts.contains(&hk) {
            continue;
        }
        if hard_skipped.contains(u) || hard_skipped.contains(&url) {
            continue;
        }
        if seen_hosts.contains(&hk) {
            continue;
        }
        let group = meta
            .get("group")
            .or_else(|| meta.get("bookSourceGroup"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if DEAD_GROUP.iter().any(|g| group.contains(g)) {
            continue;
        }
        if opts.search_tag_only && !SEARCH_HINTS.iter().any(|h| group.contains(*h)) {
            continue;
        }
        let enabled = meta.get("enabled");
        if opts.enabled_only && enabled == Some(&json!(false)) {
            continue;
        }
        let rt = rt_map
            .get(u)
            .copied()
            .or_else(|| rt_map.get(&url).copied())
            .or_else(|| meta.get("respondTime").and_then(|v| v.as_i64()))
            .unwrap_or(8000);
        if rt > opts.max_rt_ms {
            continue;
        }
        seen_hosts.insert(hk);
        let name = meta
            .get("name")
            .or_else(|| meta.get("bookSourceName"))
            .cloned()
            .unwrap_or(json!(null));
        rows.push(json!({
            "url": url,
            "name": name,
            "group": group,
            "enabled": enabled.cloned().unwrap_or(json!(null)),
            "respondTime": rt,
            "bookSourceType": 0,
            "status": "candidate",
            "on_phone": true,
            "retry": retryable.contains(u) || retryable.contains(&url),
        }));
    }
    rows.sort_by(|a, b| {
        let ra = a.get("respondTime").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
        let rb = b.get("respondTime").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
        ra.cmp(&rb).then_with(|| {
            a.get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(b.get("url").and_then(|v| v.as_str()).unwrap_or(""))
        })
    });
    let selected: Vec<Value> = rows.iter().take(opts.limit).cloned().collect();
    Ok(json!({
        "ts": Utc::now().to_rfc3339(),
        "max_rt_ms": opts.max_rt_ms,
        "limit": opts.limit,
        "phone_total": phone.get("total").cloned().unwrap_or(json!(null)),
        "n_candidate_all": rows.len(),
        "n_selected": selected.len(),
        "n_retryable_ledger": retryable.len(),
        "sort": "respondTime asc",
        "source": "phone_index+rt_join",
        "items": selected,
    }))
}
