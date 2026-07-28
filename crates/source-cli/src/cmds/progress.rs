//! Progress status/next — L2-gated candidates from phone index / RT queue.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use super::progress_goal::goal_status;
use super::progress_ledger;
use super::progress_ledger::ledger_blocked;
use serde_json::{json, Value};
use source_gate::{classify_one_l0, load_rules, SkipRule};

#[cfg(feature = "gate_full")]
use source_gate::{classify_one, ClassifyOpts};

pub struct ProgressArgs {
    pub cmd: String, // status | next
    pub index: Option<PathBuf>,
    pub rules: Option<PathBuf>,
    pub l0_only: bool,
    pub goal: Option<usize>,
}

fn default_index() -> PathBuf {
    for p in [
        PathBuf::from("temp/full_fix/phone_source_index.json"),
        PathBuf::from("../temp/full_fix/phone_source_index.json"),
    ] {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("temp/full_fix/phone_source_index.json")
}

fn default_queue() -> Option<PathBuf> {
    [
        PathBuf::from("temp/full_fix/queues/repair_serial100_queue.json"),
        PathBuf::from("../temp/full_fix/queues/repair_serial100_queue.json"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

fn default_rules() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json")
}

fn load_json(path: &PathBuf) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn phone_url_set(index: &Value) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Some(by) = index.get("by_url").and_then(|v| v.as_object()) {
        for url in by.keys() {
            set.insert(progress_ledger::norm_url(url));
        }
    }
    set
}

/// Queue items must still exist on phone when index is non-empty (stale serial
/// snapshots keep migrated/deleted URLs). Empty `on_phone` skips that filter
/// so unit tests / broken index paths can still use queue-only mode.
fn candidates_from_queue(
    queue: &Value,
    on_phone: &HashSet<String>,
    blocked: &HashSet<String>,
) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    let Some(arr) = queue
        .get("items")
        .or_else(|| queue.get("urls"))
        .and_then(|v| v.as_array())
    else {
        return out;
    };
    for row in arr {
        let url = row
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !url.starts_with("http") {
            continue;
        }
        let nu = progress_ledger::norm_url(&url);
        if !on_phone.is_empty() && !on_phone.contains(&nu) {
            continue;
        }
        if blocked.contains(&nu) {
            continue;
        }
        out.push((url, row.clone()));
    }
    out
}

fn candidate_urls(index: &Value, queue: Option<&Value>) -> Vec<(String, Value)> {
    let blocked = ledger_blocked();
    let on_phone = phone_url_set(index);
    if let Some(q) = queue {
        let from_q = candidates_from_queue(q, &on_phone, &blocked);
        if !from_q.is_empty() {
            return from_q;
        }
    }
    let mut out = Vec::new();
    if let Some(by) = index.get("by_url").and_then(|v| v.as_object()) {
        for (url, meta) in by {
            let g = meta.get("group").and_then(|v| v.as_str()).unwrap_or("");
            if !g.contains("搜索失效") {
                continue;
            }
            if meta.get("enabled") == Some(&json!(false)) {
                continue;
            }
            if !url.starts_with("http") {
                continue;
            }
            if blocked.contains(&progress_ledger::norm_url(url)) {
                continue;
            }
            out.push((url.clone(), meta.clone()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn gate_one(url: &str, rules: &[SkipRule], l0_only: bool) -> Value {
    let g = if l0_only {
        classify_one_l0(url, rules)
    } else {
        #[cfg(feature = "gate_full")]
        {
            classify_one(
                url,
                rules,
                &ClassifyOpts {
                    tcp_timeout_s: 1.5,
                    l2_timeout_s: 4.0,
                },
            )
        }
        #[cfg(not(feature = "gate_full"))]
        {
            classify_one_l0(url, rules)
        }
    };
    serde_json::to_value(&g).unwrap_or(json!({}))
}

fn ensure_closeout_ready() -> Result<(), String> {
    let paths = source_closeout::CloseoutPaths::from_repo().map_err(|e| e.to_string())?;
    source_closeout::ensure_ready_for_next(&paths)
        .map(|_| ())
        .map_err(|errors| errors.join("\n"))
}

pub fn run_progress(args: ProgressArgs) -> ExitCode {
    if args.l0_only {
        eprintln!("progress: warn: --l0-only skips L2 dead/wall; do not use for live repair pick");
    }
    let index_path = args.index.unwrap_or_else(default_index);
    let index = match load_json(&index_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("progress: index: {e}");
            return ExitCode::from(4);
        }
    };
    if args.cmd == "status" {
        let queue = default_queue().and_then(|p| load_json(&p).ok());
        let cands = candidate_urls(&index, queue.as_ref());
        let mut status = json!({
            "index": index_path.to_string_lossy(),
            "search_fail_candidates": cands.len(),
            "from_queue": queue.is_some(),
            "total": index.get("total").cloned().unwrap_or(json!(null)),
        });
        if let Some(goal) = args.goal {
            status["goal"] = goal_status(goal, None);
        }
        println!("{}", status);
        return ExitCode::SUCCESS;
    }
    if let Err(msg) = ensure_closeout_ready() {
        eprintln!("progress: {msg}");
        return ExitCode::from(3);
    }
    let queue = default_queue().and_then(|p| load_json(&p).ok());
    let rules_path = args.rules.unwrap_or_else(default_rules);
    let rules = match load_rules(&rules_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("progress: rules: {e}");
            return ExitCode::from(4);
        }
    };
    let cands = candidate_urls(&index, queue.as_ref());
    for (url, meta) in cands.iter().take(40) {
        let g = gate_one(url, &rules, args.l0_only);
        let action = g.get("action").and_then(|a| a.as_str()).unwrap_or("");
        if action == "verify" {
            println!(
                "{}",
                json!({
                    "next": {
                        "url": url,
                        "name": meta.get("name"),
                        "group": meta.get("group"),
                        "respondTime": meta.get("respondTime"),
                        "l2_gate": g,
                        "status": "candidate",
                    }
                })
            );
            return ExitCode::SUCCESS;
        }
    }
    println!(
        "{}",
        json!({"next": null, "hint": "no verify-pass candidate in first 40"})
    );
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn queue_drops_urls_not_on_phone() {
        let index = json!({
            "by_url": {
                "http://i.new.test/": {"group": "搜索失效", "enabled": true}
            }
        });
        let queue = json!({
            "items": [
                {"url": "http://m.old.test/"},
                {"url": "http://i.new.test/"}
            ]
        });
        let on_phone = phone_url_set(&index);
        let blocked = HashSet::new();
        let cands = candidates_from_queue(&queue, &on_phone, &blocked);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].0, "http://i.new.test/");
    }

    #[test]
    fn empty_phone_set_keeps_queue_items() {
        let queue = json!({
            "items": [{"url": "http://stale.only.in.queue/"}]
        });
        let on_phone = HashSet::new();
        let blocked = HashSet::new();
        let cands = candidates_from_queue(&queue, &on_phone, &blocked);
        assert_eq!(cands.len(), 1);
    }
}
