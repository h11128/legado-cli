//! Search-wave: parallel search-form patch + one batch verify.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value};
use source_mcp::{
    batch_check_urls, batch_max_wait_s, is_repair_success, FsChannelPort, McpClient, McpEndpoint,
    McpSourceRepository,
};
use source_ports::{ChannelPort, SourceRepository};
use source_types::{PortError, SourceKey};

use crate::batch::load_urls_file;
use crate::search_form::find_search_action;

#[derive(Debug, Clone)]
pub struct SearchWaveOpts {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub workers: usize,
    pub thread_count: u32,
    pub timeout_ms: u64,
    pub out: PathBuf,
}

fn header_map(source: &Value) -> Vec<(String, String)> {
    let mut h = Vec::new();
    if let Some(obj) = source.get("headerMap").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                h.push((k.clone(), s.to_string()));
            }
        }
    }
    if let Some(ua) = source.get("userAgent").and_then(|v| v.as_str()) {
        h.push(("User-Agent".into(), ua.to_string()));
    }
    h
}

fn fetch_page(url: &str, headers: &[(String, String)]) -> Result<String, PortError> {
    let mut req = ureq::get(url);
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = req
        .call()
        .map_err(|e| PortError::Transient(format!("fetch: {e}")))?;
    let body = resp
        .into_string()
        .map_err(|e| PortError::Transient(format!("read: {e}")))?;
    Ok(body)
}

fn work_one(client: Arc<McpClient>, url: &str) -> Value {
    let t0 = Instant::now();
    let mut row = json!({ "url": url });
    let repo = McpSourceRepository::new(client);
    match work_inner(&repo, url) {
        Ok(inner) => {
            if let Some(obj) = inner.as_object() {
                for (k, v) in obj {
                    row[k] = v.clone();
                }
            }
        }
        Err(e) => {
            row["action"] = json!("error");
            row["error"] = json!(e.to_string());
        }
    }
    row["ms"] = json!(t0.elapsed().as_millis() as u64);
    row
}

fn work_inner(repo: &McpSourceRepository, url: &str) -> Result<Value, PortError> {
    let src = repo.get(&SourceKey::new(url))?;
    let name = src
        .as_value()
        .get("bookSourceName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base = url.split('#').next().unwrap_or(url);
    let base = if base.contains("://") {
        base.to_string()
    } else {
        format!("http://{base}")
    };
    let html = fetch_page(&base, &header_map(src.as_value()))?;
    let action = find_search_action(&html, &base);
    let current = src
        .search_url()
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let mut out = json!({ "name": name, "search_action": action });
    if let Some(ref act) = action {
        if act != &current {
            let mut v = src.into_value();
            v["searchUrl"] = json!(act);
            if v.get("concurrentRate").is_none() {
                v["concurrentRate"] = json!("1000");
            }
            repo.save(&source_types::BookSource::new(v))?;
            out["save"] = json!(true);
            out["action"] = json!("patched_search");
        } else {
            out["action"] = json!("search_unchanged");
        }
    } else {
        out["action"] = json!("no_form");
    }
    Ok(out)
}

pub fn run_search_wave(opts: SearchWaveOpts) -> Result<Value, PortError> {
    let urls = load_urls_file(&opts.urls_file)?;
    let t0 = Instant::now();
    let channel = FsChannelPort::from_repo()?;
    let _guard = channel.acquire_repair()?;
    let ep = McpEndpoint::load_defaults()?;
    let client = Arc::new(McpClient::new(ep).with_client_name("search_wave"));
    client.ensure_session()?;

    let workers = opts.workers.max(1).min(urls.len().max(1));
    let chunk = urls.len().div_ceil(workers);
    let rows: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for part in urls.chunks(chunk.max(1)) {
        let part: Vec<String> = part.to_vec();
        let client = Arc::clone(&client);
        let rows = Arc::clone(&rows);
        handles.push(thread::spawn(move || {
            for url in part {
                rows.lock()
                    .expect("rows")
                    .push(work_one(Arc::clone(&client), &url));
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let mut per = rows.lock().expect("rows").clone();
    per.sort_by(|a, b| {
        a.get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("url").and_then(|v| v.as_str()).unwrap_or(""))
    });

    let verify_urls: Vec<String> = per
        .iter()
        .filter(|r| r.get("action").and_then(|v| v.as_str()) == Some("patched_search"))
        .filter_map(|r| r.get("url").and_then(|v| v.as_str()).map(String::from))
        .collect();

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
            false,
        )?;
        for row in &mut per {
            if row.get("action").and_then(|v| v.as_str()) != Some("patched_search") {
                continue;
            }
            let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let cr = check_results
                .iter()
                .find(|r| r.get("url").and_then(|v| v.as_str()) == Some(url));
            if let Some(cr) = cr {
                let msg = cr.get("message").and_then(|v| v.as_str()).unwrap_or("");
                row["check"] = cr.clone();
                row["fixed"] = json!(is_repair_success(msg));
            }
        }
    }

    let report = json!({
        "ts": Utc::now().to_rfc3339(),
        "n": urls.len(),
        "wall_s": t0.elapsed().as_secs_f64(),
        "verify_urls": verify_urls,
        "per": per,
        "check": check_results,
    });
    write_json(&opts.out, &report)?;
    Ok(report)
}

fn write_json(path: &Path, value: &Value) -> Result<(), PortError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(format!("mkdir: {e}")))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(format!("write: {e}")))?;
    Ok(())
}
