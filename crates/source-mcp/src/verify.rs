//! `VerifyPort` via `start_check_sources` + `get_check_progress` poll.
//!
//! Polling mirrors a simplified `scripts/repair_wait.wait_check` (adaptive sleep,
//! max wait, full result page-in). Discovery-only failures score as success when
//! `CheckOpts.check_discovery` is false (Python `is_repair_success`).

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use source_ports::VerifyPort;
use source_types::{CheckOpts, Mode, PortError, SourceKey, VerifyResult};

use crate::client::McpClient;
use crate::timeouts::McpTimeouts;

const DEFAULT_KEYWORD: &str = "我的";

/// Device verify over MCP check tools.
pub struct McpVerifyPort {
    client: Arc<McpClient>,
    ready: std::sync::OnceLock<()>,
    keyword: String,
    timeout_ms: u64,
    max_wait_s: f64,
}

impl McpVerifyPort {
    pub fn new(client: Arc<McpClient>) -> Self {
        let t = McpTimeouts::load_defaults();
        Self {
            client,
            ready: std::sync::OnceLock::new(),
            keyword: DEFAULT_KEYWORD.into(),
            timeout_ms: t.verify_timeout_ms,
            max_wait_s: t.verify_max_wait_s,
        }
    }

    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keyword = keyword.into();
        self
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn with_max_wait_s(mut self, s: f64) -> Self {
        self.max_wait_s = s;
        self
    }

    fn ensure_ready(&self) -> Result<(), PortError> {
        if self.ready.get().is_some() {
            return Ok(());
        }
        self.client.ensure_session()?;
        let _ = self.ready.set(());
        Ok(())
    }

    fn tools_text(&self, name: &str, args: Value) -> Result<Value, PortError> {
        let result = self.client.tools_call(name, args)?;
        let text = McpClient::extract_text(&result);
        Ok(McpClient::parse_json_text(&text))
    }
}

impl VerifyPort for McpVerifyPort {
    fn check(&self, key: &SourceKey, opts: CheckOpts) -> Result<VerifyResult, PortError> {
        self.ensure_ready()?;
        let url = key.as_str();
        let started = Instant::now();

        let _ = self.tools_text("stop_check_sources", json!({}));

        let args = check_args(url, &self.keyword, self.timeout_ms, opts.check_discovery);
        self.tools_text("start_check_sources", args)?;

        let snap = wait_check(&self.client, self.max_wait_s)?;
        let item = match_result(&snap, url);
        let (ok, message, raw) = match item {
            Some(row) => {
                let msg = row
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let device_ok = row
                    .get("success")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);
                let success = if device_ok {
                    true
                } else if !opts.check_discovery {
                    is_repair_success(&msg)
                } else {
                    false
                };
                (success, msg, Some(row))
            }
            None => (
                false,
                format!("no check result for {url}"),
                Some(snap.clone()),
            ),
        };

        let url_typed = key
            .to_url()
            .map_err(|e| PortError::ContractViolation(e.to_string()))?;
        let mut vr = VerifyResult::new(url_typed, ok, message, Mode::Oneshot);
        vr.check_discovery = opts.check_discovery;
        vr.duration_ms = Some(started.elapsed().as_millis() as u64);
        vr.raw_check = raw;
        Ok(vr)
    }
}

fn check_args(url: &str, keyword: &str, timeout_ms: u64, check_discovery: bool) -> Value {
    json!({
        "urls": [url],
        "enabledOnly": false,
        "keyword": keyword,
        "threadCount": 1,
        "timeoutMs": timeout_ms,
        "checkDomain": false,
        "checkSearch": true,
        "checkDiscovery": check_discovery,
        "checkInfo": true,
        "checkCategory": true,
        "checkContent": true,
    })
}

pub(crate) fn wait_check(client: &McpClient, max_wait_s: f64) -> Result<Value, PortError> {
    let started = Instant::now();
    let mut interval = Duration::from_millis(400);
    let poll_max = Duration::from_millis(1200);
    let mut last: Value = json!({});

    loop {
        let result = client.tools_call(
            "get_check_progress",
            json!({ "resultOffset": 0, "resultLimit": 1 }),
        )?;
        let text = McpClient::extract_text(&result);
        let snap = McpClient::parse_json_text(&text);
        if snap.is_object() {
            last = snap;
        }
        let running = last
            .get("running")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !running {
            return fetch_all_results(client, &last);
        }
        if started.elapsed().as_secs_f64() >= max_wait_s {
            let _ = client.tools_call("stop_check_sources", json!({}));
            thread::sleep(Duration::from_millis(300));
            return fetch_all_results(client, &last);
        }
        thread::sleep(interval);
        interval = (interval.mul_f32(1.25)).min(poll_max);
    }
}

pub(crate) fn fetch_all_results(client: &McpClient, seed: &Value) -> Result<Value, PortError> {
    let mut all = Vec::new();
    let mut offset = 0usize;
    let mut total = seed
        .get("resultTotal")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let mut last = seed.clone();

    loop {
        let result = client.tools_call(
            "get_check_progress",
            json!({ "resultOffset": offset, "resultLimit": 500 }),
        )?;
        let text = McpClient::extract_text(&result);
        let page = McpClient::parse_json_text(&text);
        if !page.is_object() {
            break;
        }
        last = page;
        let chunk = last
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let n = chunk.len();
        all.extend(chunk);
        total = last
            .get("resultTotal")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(total);
        offset += n;
        if n == 0 || offset >= total {
            break;
        }
    }
    if let Some(obj) = last.as_object_mut() {
        obj.insert("results".into(), Value::Array(all));
    }
    Ok(last)
}

pub(crate) fn match_result(snap: &Value, url: &str) -> Option<Value> {
    let results = snap.get("results")?.as_array()?;
    for row in results {
        let row_url = row.get("url").and_then(|u| u.as_str())?;
        let want = url.trim().trim_end_matches('/');
        let got = row_url.trim().trim_end_matches('/');
        if row_url == url || got == want {
            return Some(row.clone());
        }
    }
    // Never fall back to results[0] — that can claim another source's success (fake fixed).
    None
}

pub fn is_repair_success(message: &str) -> bool {
    let mut msg = message.to_string();
    for tok in ["发现正文失效", "发现目录失效", "发现规则为空", "发现失效"] {
        msg = msg.replace(tok, "");
    }
    let cleaned: String = msg
        .replace("校验失败", "")
        .chars()
        .filter(|c| !matches!(c, ',' | '，' | ' ' | ':' | '：'))
        .collect();
    cleaned.is_empty() && !message.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_only_message_is_ok() {
        assert!(is_repair_success("校验失败:发现目录失效"));
        assert!(!is_repair_success("校验失败:搜索失效"));
    }

    #[test]
    fn check_args_default_no_discovery() {
        let a = check_args("https://a.example/", "我的", 45000, false);
        assert_eq!(a["checkDiscovery"], false);
        assert_eq!(a["threadCount"], 1);
    }

    #[test]
    fn match_result_no_wrong_url_fallback() {
        let snap = json!({
            "results": [
                {"url": "https://other.example/", "success": true, "message": "校验成功"}
            ]
        });
        assert!(match_result(&snap, "https://wanted.example/").is_none());
        assert!(match_result(&snap, "https://other.example/").is_some());
    }
}
