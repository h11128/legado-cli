//! Deep-wave: budgeted toc-clear / searchUrl patch + verify per URL.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::{json, Value};
use source_mcp::{FsChannelPort, McpClient, McpEndpoint, McpSourceRepository, McpVerifyPort};
use source_ports::{ChannelPort, SourceRepository, VerifyPort};
use source_types::{CheckOpts, PortError, SourceKey};

use crate::batch::load_urls_file;
use crate::search_form::find_search_action;

#[derive(Debug, Clone)]
pub struct DeepWaveOpts {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub budget_s: f64,
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

fn fetch_page(
    url: &str,
    headers: &[(String, String)],
    timeout: Duration,
) -> Result<String, PortError> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let mut req = agent.get(url);
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = req
        .call()
        .map_err(|e| PortError::Transient(format!("fetch: {e}")))?;
    resp.into_string()
        .map_err(|e| PortError::Transient(format!("read: {e}")))
}

fn debug_book_count(client: &McpClient, url: &str, key: &str) -> Result<usize, PortError> {
    let raw = client.tools_call("debug_source", json!({"url": url, "key": key}))?;
    let text = McpClient::extract_text(&raw);
    let n = text.matches("bookUrl").count();
    Ok(n)
}

fn deep_one(client: Arc<McpClient>, url: &str, key: &str, budget_s: f64) -> Value {
    let t0 = Instant::now();
    let mut row = json!({ "url": url, "steps": [] });
    let left = || budget_s - t0.elapsed().as_secs_f64();
    if left() < 5.0 {
        row["result"] = json!("skip_budget");
        return row;
    }
    let repo = McpSourceRepository::new(Arc::clone(&client));
    let Ok(mut src) = repo.get(&SourceKey::new(url)) else {
        row["result"] = json!("get_fail");
        return row;
    };
    row["name"] = src
        .as_value()
        .get("bookSourceName")
        .cloned()
        .unwrap_or(json!(null));
    let headers = header_map(src.as_value());

    let n = debug_book_count(&client, url, key).unwrap_or(0);
    row["steps"]
        .as_array_mut()
        .expect("steps")
        .push(json!({"debug_books": n}));

    if n > 0 && left() > 10.0 {
        let mut v = src.as_value().clone();
        if let Some(info) = v.get_mut("ruleBookInfo").and_then(|x| x.as_object_mut()) {
            if info
                .get("tocUrl")
                .and_then(|x| x.as_str())
                .is_some_and(|s| !s.is_empty())
            {
                info.insert("tocUrl".into(), json!(""));
                v["ruleBookInfo"] = json!(info);
                if v.get("concurrentRate").is_none() {
                    v["concurrentRate"] = json!("1000");
                }
                src = source_types::BookSource::new(v);
                let _ = repo.save(&src);
                row["steps"]
                    .as_array_mut()
                    .expect("steps")
                    .push(json!({"patch": "clear_tocUrl"}));
                if left() > 8.0 {
                    let verify = McpVerifyPort::new(client.clone());
                    if let Ok(vr) = verify.check(&SourceKey::new(url), CheckOpts::default()) {
                        row["check"] = json!({"success": vr.success, "message": vr.message});
                        row["result"] = json!(if vr.success {
                            "fixed"
                        } else {
                            "failed_after_toc_clear"
                        });
                        row["wall_s"] = json!(t0.elapsed().as_secs_f64());
                        return row;
                    }
                }
            }
        }
    }

    if n == 0 && left() > 15.0 {
        let base = url.split('#').next().unwrap_or(url);
        let base = if base.contains("://") {
            base.to_string()
        } else {
            format!("http://{base}")
        };
        let headers = headers.clone();
        let timeout = Duration::from_secs_f64(left().clamp(2.0, 12.0));
        match fetch_page(&base, &headers, timeout) {
            Ok(html) => {
                let action = find_search_action(&html, &base);
                row["steps"]
                    .as_array_mut()
                    .expect("steps")
                    .push(json!({"search_action": action}));
                if let Some(act) = action {
                    let mut v = src.as_value().clone();
                    v["searchUrl"] = json!(act);
                    if v.get("concurrentRate").is_none() {
                        v["concurrentRate"] = json!("1000");
                    }
                    src = source_types::BookSource::new(v);
                    let _ = repo.save(&src);
                    row["steps"]
                        .as_array_mut()
                        .expect("steps")
                        .push(json!({"patch": format!("searchUrl={act}")}));
                    if left() > 8.0 {
                        thread::sleep(Duration::from_secs(2));
                        let verify = McpVerifyPort::new(client);
                        if let Ok(vr) = verify.check(&SourceKey::new(url), CheckOpts::default()) {
                            row["check"] = json!({"success": vr.success, "message": vr.message});
                            row["result"] = json!(if vr.success {
                                "fixed"
                            } else {
                                "failed_after_searchUrl"
                            });
                            row["wall_s"] = json!(t0.elapsed().as_secs_f64());
                            return row;
                        }
                    }
                }
            }
            Err(e) => {
                row["steps"]
                    .as_array_mut()
                    .expect("steps")
                    .push(json!({"fetch_err": e.to_string()}));
            }
        }
    }

    if row.get("result").is_none() {
        row["result"] = json!("skip_no_quick_fix");
    }
    row["wall_s"] = json!(t0.elapsed().as_secs_f64());
    row
}

pub fn run_deep_wave(opts: DeepWaveOpts) -> Result<Value, PortError> {
    let urls = load_urls_file(&opts.urls_file)?;
    let channel = FsChannelPort::from_repo()?;
    let _guard = channel.acquire_repair()?;
    let ep = McpEndpoint::load_defaults()?;
    let client = Arc::new(McpClient::new(ep).with_client_name("deep_wave"));
    client.ensure_session()?;
    let mut per = Vec::new();
    for url in urls {
        per.push(deep_one(
            Arc::clone(&client),
            &url,
            &opts.keyword,
            opts.budget_s,
        ));
    }
    let report = json!({
        "ts": Utc::now().to_rfc3339(),
        "n": per.len(),
        "budget_s": opts.budget_s,
        "per": per,
    });
    if let Some(parent) = opts.out.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(format!("mkdir: {e}")))?;
    }
    fs::write(
        &opts.out,
        serde_json::to_string_pretty(&report).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(format!("write: {e}")))?;
    Ok(report)
}
