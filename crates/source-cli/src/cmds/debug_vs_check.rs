//! Compare debug_source vs check — bookUrl→infoHtml trap (Python `repair_debug_vs_check.py`).

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use regex::Regex;
use serde_json::{json, Value};
use source_mcp::{
    batch_check_urls, batch_max_wait_s, repo_root, FsChannelPort, McpClient, McpEndpoint,
    McpSourceRepository,
};
use source_ports::{ChannelPort, LedgerPort, SourceRepository};
use source_types::{LedgerRow, LedgerStep, PortError, SourceKey, Url};

pub struct DebugVsCheckArgs {
    pub url: String,
    pub key: String,
    pub out: PathBuf,
    pub no_ledger: bool,
}

pub fn run_debug_vs_check(args: DebugVsCheckArgs) -> ExitCode {
    match run_inner(&args) {
        Ok((report, check_ok)) => {
            if let Err(e) = write_report(&args.out, &report) {
                eprintln!("debug-vs-check: {e}");
                return ExitCode::from(2);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
            println!("wrote {}", args.out.display());
            if check_ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("debug-vs-check: {e}");
            ExitCode::from(3)
        }
    }
}

fn run_inner(args: &DebugVsCheckArgs) -> Result<(Value, bool), PortError> {
    let root = repo_root()?;
    let channel = FsChannelPort::new(&root);
    let _guard = channel.acquire_repair()?;

    let ep = McpEndpoint::load_defaults()?;
    let client = Arc::new(McpClient::new(ep).with_client_name("debug_vs_check"));
    client.ensure_session()?;
    let repo = McpSourceRepository::new(Arc::clone(&client));
    let key = SourceKey::new(&args.url);
    let src = repo.get(&key)?;
    let src_val = src.as_value();

    let mut report = json!({
        "ts": Utc::now().to_rfc3339(),
        "url": args.url,
        "key": args.key,
        "bookSourceType": src_val.get("bookSourceType"),
        "name": src_val.get("bookSourceName"),
    });

    let _ = client.tools_call("set_http_log_recording", json!({ "enabled": true }));

    let t0 = Instant::now();
    let debug_result =
        client.tools_call("debug_source", json!({ "url": args.url, "key": args.key }))?;
    let debug_text = McpClient::extract_text(&debug_result);
    report["debug_ms"] = json!(t0.elapsed().as_millis() as u64);
    report["debug_has_m3u8"] = json!(debug_text.contains("m3u8"));
    report["debug_empty_dl"] = json!(debug_text.contains("下载链接为空"));

    let t1 = Instant::now();
    let rows = batch_check_urls(
        &client,
        std::slice::from_ref(&args.url),
        &args.key,
        1,
        60_000,
        batch_max_wait_s(1, 60.0),
        false,
    )?;
    report["check_ms"] = json!(t1.elapsed().as_millis() as u64);
    let row = rows.first().cloned().unwrap_or_default();
    let msg = row.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let check_ok = row
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    report["check_ok"] = json!(check_ok);
    report["check_msg"] = json!(msg);

    let logs_raw =
        McpClient::extract_text(&client.tools_call("get_http_logs", json!({ "limit": 12 }))?);
    let host = host_of(&args.url);
    let urls = parse_log_urls(&logs_raw, &host);
    report["http_urls"] = json!(urls.iter().rev().take(8).collect::<Vec<_>>());
    let diagnosis = classify(&debug_text, msg, &urls);
    report["diagnosis"] = json!(diagnosis);
    report["hint"] = json!(hint_for(diagnosis));

    if !args.no_ledger {
        let ledger = source_mcp::DualLedgerPort::from_defaults()?;
        let u = Url::new(&args.url).map_err(|e| PortError::Permanent(e.to_string()))?;
        let mut lr = LedgerRow::new(
            Utc::now().to_rfc3339(),
            u,
            LedgerStep::Diagnose,
            diagnosis.to_string(),
        );
        lr.note = Some(msg.to_string());
        if diagnosis != "ok" {
            lr.waste = Some("see hint".into());
        }
        let _ = ledger.append(&lr);
    }

    Ok((report, check_ok))
}

fn write_report(path: &PathBuf, report: &Value) -> Result<(), PortError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| PortError::Permanent(format!("mkdir: {e}")))?;
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(report).map_err(|e| PortError::Permanent(e.to_string()))?,
    )
    .map_err(|e| PortError::Permanent(format!("write: {e}")))?;
    Ok(())
}

fn host_of(url: &str) -> String {
    url::Url::parse(url.split('#').next().unwrap_or(url))
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_default()
}

fn parse_log_urls(raw: &str, host: &str) -> Vec<String> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"GET (https?://\S+)").expect("log url re"));
    re.captures_iter(raw)
        .filter_map(|c| {
            c.get(1)
                .map(|m| m.as_str().trim_end_matches(',').to_string())
        })
        .filter(|u| host.is_empty() || u.to_lowercase().contains(host))
        .collect()
}

fn classify(debug_text: &str, check_msg: &str, http_urls: &[String]) -> &'static str {
    let has_detail = http_urls
        .iter()
        .any(|u| u.contains("/detail") || u.contains("/vod/detail") || u.contains("softdown"));
    let has_search = http_urls.iter().any(|u| {
        u.contains("search") || u.contains("wd=") || u.contains("keyword") || u.contains("q=")
    });
    let debug_dl = debug_text.contains("m3u8")
        || (debug_text.contains("下载链接") && !debug_text.contains("下载链接为空"));
    let check_empty = check_msg.contains("下载链接为空");
    if debug_dl && check_empty && has_search && !has_detail {
        return "bookUrl_infoHtml_trap";
    }
    if debug_dl && check_empty {
        return "debug_ok_check_empty_dl";
    }
    if check_msg.contains("校验成功") || check_msg.ends_with("成功") {
        return "ok";
    }
    "other_fail"
}

fn hint_for(diagnosis: &str) -> &'static str {
    match diagnosis {
        "bookUrl_infoHtml_trap" => {
            "Search bookUrl likely empty/falls back to search page; infoHtml hijacks detail. \
             Fix ruleSearch.bookUrl to detail link; avoid >-only @css if flaky."
        }
        "debug_ok_check_empty_dl" => {
            "Compare first search hit; fix downloadUrls on real detail DOM."
        }
        "ok" => "No action.",
        _ => "Inspect debug_text + check_msg; not the classic trap.",
    }
}
