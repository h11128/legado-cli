//! Repair wave: prefilter → parallel patch → one batch verify.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value};
use source_mcp::{
    batch_check_urls, batch_max_wait_s, repo_root, FsChannelPort, McpClient, McpEndpoint,
};
use source_ports::{ChannelPort, LedgerPort, SourceRepository};
use source_types::{LedgerRow, LedgerStep, PortError, Url};

use crate::prefilter::filter_urls;
use crate::wave_patch::patch_one;

#[derive(Debug, Clone)]
pub struct WaveOpts {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub thread_count: u32,
    pub patch_workers: usize,
    pub timeout_ms: u64,
    pub check_discovery: bool,
    pub disable_dropped: bool,
    pub out: PathBuf,
    pub rules: PathBuf,
    pub l2_timeout: f64,
}

pub fn run_wave(opts: WaveOpts) -> Result<Value, PortError> {
    let urls = crate::batch::load_urls_file(&opts.urls_file)?;
    let t0 = Instant::now();
    let root = repo_root()?;
    let channel = FsChannelPort::new(&root);
    let _guard = channel.acquire_repair()?;

    let pref = filter_urls(&urls, &opts.rules, 16, opts.l2_timeout)?;
    let ep = McpEndpoint::load_defaults()?;
    let client = Arc::new(McpClient::new(ep).with_client_name("source_wave"));
    client.ensure_session()?;

    let mut per: Vec<Value> = Vec::new();
    append_prefilter_ledger(&pref, &mut per)?;

    if opts.disable_dropped {
        for row in &pref.disable {
            let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url.is_empty() {
                continue;
            }
            let repo = source_mcp::McpSourceRepository::new(Arc::clone(&client));
            let key = source_types::SourceKey::new(url);
            let _ = repo.disable(&key);
        }
    }

    let workers = opts.patch_workers.max(1).min(pref.verify_urls.len().max(1));
    let mut verify_urls: Vec<String> = Vec::new();
    let patch_t0 = Instant::now();
    if workers == 1 || pref.verify_urls.len() <= 1 {
        for url in &pref.verify_urls {
            let row = patch_one(Arc::clone(&client), url);
            if row.get("verify").and_then(|v| v.as_bool()) == Some(true) {
                verify_urls.push(url.clone());
            }
            per.push(row);
        }
    } else {
        let chunks: Vec<Vec<String>> = pref
            .verify_urls
            .chunks(pref.verify_urls.len().div_ceil(workers))
            .map(|c| c.to_vec())
            .collect();
        let mut handles = Vec::new();
        for chunk in chunks {
            let client = Arc::clone(&client);
            handles.push(thread::spawn(move || {
                chunk
                    .into_iter()
                    .map(|url| {
                        let row = patch_one(Arc::clone(&client), &url);
                        (url, row)
                    })
                    .collect::<Vec<_>>()
            }));
        }
        for h in handles {
            for (url, row) in h
                .join()
                .map_err(|_| PortError::Permanent("patch thread".into()))?
            {
                if row.get("verify").and_then(|v| v.as_bool()) == Some(true) {
                    verify_urls.push(url);
                }
                per.push(row);
            }
        }
    }
    let patch_s = patch_t0.elapsed().as_secs_f64();

    let mut check_results = Vec::new();
    if !verify_urls.is_empty() {
        let max_wait = batch_max_wait_s(verify_urls.len(), opts.timeout_ms as f64 / 1000.0);
        check_results = batch_check_urls(
            &client,
            &verify_urls,
            &opts.keyword,
            opts.thread_count,
            opts.timeout_ms,
            max_wait,
            opts.check_discovery,
        )?;
    }

    let report = json!({
        "ts": Utc::now().to_rfc3339(),
        "n_in": urls.len(),
        "phases": {
            "prefilter_s": t0.elapsed().as_secs_f64(),
            "patch_s": patch_s,
            "patch_workers": workers,
            "total_s": t0.elapsed().as_secs_f64(),
        },
        "prefilter": {
            "verify": pref.verify_urls.len(),
            "skip": pref.skip.len(),
            "disable": pref.disable.len(),
            "video": pref.video.len(),
            "hunt": pref.hunt.len(),
        },
        "per": per,
        "check": check_results,
        "policy": {
            "checkDiscovery": opts.check_discovery,
        },
    });
    write_report(&opts.out, &report)?;
    Ok(report)
}

fn append_prefilter_ledger(
    pref: &crate::prefilter::PrefilterSummary,
    per: &mut Vec<Value>,
) -> Result<(), PortError> {
    let ledger = source_mcp::DualLedgerPort::from_defaults()?;
    let ts = Utc::now().to_rfc3339();
    // Hunt is NOT a final skip — policy requires seed probe before disable.
    // Leave hunt rows in the report for `source-cli repair` / `hunt --probe`.
    for bucket in [&pref.skip, &pref.disable, &pref.video] {
        for row in bucket {
            let url_s = row.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url_s.is_empty() {
                continue;
            }
            let u = Url::new(url_s).map_err(|e| PortError::Permanent(e.to_string()))?;
            let action = row.get("action").and_then(|v| v.as_str()).unwrap_or("skip");
            let reason = row.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            let mut lr = LedgerRow::new(ts.clone(), u, LedgerStep::Skip, action.to_string());
            lr.note = Some(reason.to_string());
            let _ = ledger.append(&lr);
            per.push(json!({
                "url": url_s,
                "action": format!("prefilter_{action}"),
                "reason": reason,
            }));
        }
    }
    for row in &pref.hunt {
        let url_s = row.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if url_s.is_empty() {
            continue;
        }
        let reason = row.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        per.push(json!({
            "url": url_s,
            "action": "prefilter_hunt",
            "reason": reason,
            "policy": "hunt_before_disable",
            "next": "source-cli repair --mode oneshot --url …  OR  source-cli hunt --url … --probe",
        }));
    }
    Ok(())
}

fn write_report(path: &Path, report: &Value) -> Result<(), PortError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(format!("mkdir: {e}")))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(report).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(format!("write {}: {e}", path.display())))?;
    Ok(())
}

pub fn default_rules_path() -> PathBuf {
    for p in [
        PathBuf::from("config/verify_skip_rules.json"),
        PathBuf::from("../config/verify_skip_rules.json"),
    ] {
        if p.is_file() {
            return p;
        }
    }
    repo_root()
        .map(|r| r.join("config/verify_skip_rules.json"))
        .unwrap_or_else(|_| PathBuf::from("config/verify_skip_rules.json"))
}
