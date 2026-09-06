//! After MCP search-empty diagnose: sniff phone HTTP logs for 搜索间隔 / ss_search_delay.

use regex::Regex;
use serde_json::json;
use source_gate::sniff_search_rate_limit;
use source_mcp::McpClient;
use source_types::{DiagnoseResult, Layer};

const TIP: &str =
    "TRAP ss_search_delay_cookie (HTTP log): App search body has 搜索间隔/ss_search_delay — \
NOT a selector bug. Run `source-cli check clear-cookies --url <bookSourceUrl>` \
(Cursor MCP catalog may omit clear_cookies; CLI is the SOT entry). \
Set enabledCookieJar=false; optional searchUrl @js cookie.removeCookie(base). Retest.";

/// When diagnose layer is Search (list=0 / fake_detail), sniff throttle evidence.
/// Call **after** `attach_probe_tips` so tips are not wiped by `layer_tips` reset.
pub fn enrich_http_log_on_empty_search(
    client: &McpClient,
    diag: &mut DiagnoseResult,
    url: &str,
    key: &str,
    first_debug: &str,
) {
    if diag.layer != Layer::Search && diag.fake_detail != Some(true) {
        return;
    }
    // Cheap path: throttle text already in the first debug transcript.
    if let Some(hit) = sniff_search_rate_limit(first_debug) {
        push_hit(diag, hit, first_debug);
        return;
    }

    let _ = client.tools_call("set_http_log_recording", json!({ "enabled": true }));
    let second = match client.tools_call("debug_source", json!({ "url": url, "key": key })) {
        Ok(v) => McpClient::extract_text(&v),
        Err(e) => {
            diag.tips
                .push(format!("http_log: re-debug_source failed: {e}"));
            let _ = client.tools_call("set_http_log_recording", json!({ "enabled": false }));
            return;
        }
    };
    if let Some(hit) = sniff_search_rate_limit(&second) {
        push_hit(diag, hit, &second);
        let _ = client.tools_call("set_http_log_recording", json!({ "enabled": false }));
        return;
    }

    let logs_raw = match client.tools_call("get_http_logs", json!({ "limit": 20 })) {
        Ok(v) => McpClient::extract_text(&v),
        Err(e) => {
            diag.tips
                .push(format!("http_log: get_http_logs failed: {e}"));
            let _ = client.tools_call("set_http_log_recording", json!({ "enabled": false }));
            return;
        }
    };
    let mut bodies = vec![logs_raw.clone()];
    for id in parse_log_ids(&logs_raw).into_iter().rev().take(8) {
        if let Ok(v) = client.tools_call("get_http_log", json!({ "id": id })) {
            bodies.push(McpClient::extract_text(&v));
        }
    }
    let _ = client.tools_call("set_http_log_recording", json!({ "enabled": false }));

    let blob = bodies.join("\n");
    if let Some(hit) = sniff_search_rate_limit(&blob) {
        push_hit(diag, hit, &blob);
    } else {
        diag.tips.push(
            "http_log: no 搜索间隔/ss_search_delay in debug/HTTP logs — \
             treat as selector/WAF or empty index; still prefer clear-cookies once if unsure"
                .into(),
        );
    }
}

fn push_hit(diag: &mut DiagnoseResult, hit: &str, blob: &str) {
    diag.tips.insert(0, format!("{TIP} (hit={hit})"));
    if diag.evidence.debug_snippet.is_none() {
        let snip: String = blob.chars().take(400).collect();
        diag.evidence.debug_snippet = Some(format!("http_log_rate_limit:{snip}"));
    }
}

/// App summary lines look like: `#12 2026-… GET https://… -> 200 …`
fn parse_log_ids(raw: &str) -> Vec<i64> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?m)^#(\d+)\b").expect("log id re"));
    let mut ids: Vec<i64> = re
        .captures_iter(raw)
        .filter_map(|c| c.get(1)?.as_str().parse().ok())
        .collect();
    // Fallbacks for JSON-ish dumps
    for line in raw.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("id=") {
            if let Ok(n) = rest.split_whitespace().next().unwrap_or("").parse() {
                ids.push(n);
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hash_ids_from_app_summary() {
        let raw = "#12 2026-08-04 GET http://h/search -> 200\n#7 2026-08-04 POST http://h -> 403\n";
        assert_eq!(parse_log_ids(raw), vec![7, 12]);
    }

    #[test]
    fn sniff_in_blob() {
        assert!(sniff_search_rate_limit(r#"alert("搜索间隔: 30")"#).is_some());
    }
}
