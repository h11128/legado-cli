//! DNS + HTTP precheck — parity with `precheck_sources.py`.

use serde_json::{json, Value};
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug, Clone)]
pub struct PrecheckRow {
    pub url: String,
    pub host: String,
    pub dns_ok: bool,
    pub http_ok: bool,
    pub ok: bool,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub fn parse_host(url: &str) -> Option<String> {
    let raw = url.split('#').next()?.trim();
    if raw.is_empty() {
        return None;
    }
    let with = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    Url::parse(&with).ok()?.host_str().map(str::to_string)
}

fn dns_ok(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("invalid_url".into());
    }
    if host.len() > 253 || host.split('.').any(|l| l.len() > 63) {
        return Err("dns:invalid_hostname".into());
    }
    format!("{host}:80")
        .to_socket_addrs()
        .map(|mut it| it.next())
        .map_err(|e| format!("dns:{e}"))?
        .ok_or_else(|| "dns:no_addr".to_string())?;
    Ok(())
}

fn probe_http(url: &str, timeout: Duration) -> (bool, Option<u16>, Option<String>) {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let head = agent.head(url).call();
    match head {
        Ok(resp) => {
            let st = resp.status();
            ((200..500).contains(&st), Some(st), None)
        }
        Err(ureq::Error::Status(code, _)) => (code < 500, Some(code), Some(format!("http:{code}"))),
        Err(_) => match agent.get(url).call() {
            Ok(resp) => {
                let st = resp.status();
                ((200..500).contains(&st), Some(st), None)
            }
            Err(ureq::Error::Status(code, _)) => {
                (code < 500, Some(code), Some(format!("http:{code}")))
            }
            Err(e) => (false, None, Some(format!("http:{e}"))),
        },
    }
}

pub fn probe_one(url: &str, timeout_s: f64) -> PrecheckRow {
    let t0 = Instant::now();
    let host = parse_host(url).unwrap_or_default();
    let mut row = PrecheckRow {
        url: url.to_string(),
        host: host.clone(),
        dns_ok: false,
        http_ok: false,
        ok: false,
        status: None,
        error: None,
        duration_ms: 0,
    };
    if host.is_empty() {
        row.error = Some("invalid_url".into());
        row.duration_ms = t0.elapsed().as_millis() as u64;
        return row;
    }
    if let Err(e) = dns_ok(&host) {
        row.error = Some(e);
        row.duration_ms = t0.elapsed().as_millis() as u64;
        return row;
    }
    row.dns_ok = true;
    let probe_url = {
        let base = url.split('#').next().unwrap_or(url).trim();
        if base.contains("://") {
            base.to_string()
        } else {
            format!("http://{base}")
        }
    };
    let timeout = Duration::from_secs_f64(timeout_s.max(0.5));
    let (http_ok, status, err) = probe_http(&probe_url, timeout);
    row.http_ok = http_ok;
    row.status = status;
    row.error = err;
    row.ok = row.dns_ok && row.http_ok;
    row.duration_ms = t0.elapsed().as_millis() as u64;
    row
}

pub fn precheck_urls(urls: &[String], timeout_s: f64, concurrency: usize) -> Vec<PrecheckRow> {
    let rows: Arc<Mutex<Vec<PrecheckRow>>> = Arc::new(Mutex::new(Vec::with_capacity(urls.len())));
    let workers = concurrency.max(1).min(urls.len().max(1));
    let chunk = urls.len().div_ceil(workers);
    let mut handles = Vec::new();
    for part in urls.chunks(chunk.max(1)) {
        let part: Vec<String> = part.to_vec();
        let rows = Arc::clone(&rows);
        handles.push(thread::spawn(move || {
            for url in part {
                let r = probe_one(&url, timeout_s);
                rows.lock().expect("precheck rows").push(r);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    let mut out = rows.lock().expect("precheck rows").clone();
    out.sort_by(|a, b| a.url.cmp(&b.url));
    out
}

pub fn precheck_report(urls: &[String], timeout_s: f64, concurrency: usize) -> Value {
    let results = precheck_urls(urls, timeout_s, concurrency);
    let alive_urls: Vec<String> = results
        .iter()
        .filter(|r| r.dns_ok)
        .map(|r| r.url.clone())
        .collect();
    let dead_urls: Vec<String> = results
        .iter()
        .filter(|r| !r.dns_ok)
        .map(|r| r.url.clone())
        .collect();
    json!({
        "total": results.len(),
        "dns_ok": alive_urls.len(),
        "dns_fail": dead_urls.len(),
        "alive_urls": alive_urls,
        "dead_urls": dead_urls,
        "results": results.iter().map(row_json).collect::<Vec<_>>(),
    })
}

pub fn precheck_json(urls: &[String], timeout_s: f64) -> Value {
    precheck_report(urls, timeout_s, 32)
}

fn row_json(r: &PrecheckRow) -> Value {
    json!({
        "url": r.url,
        "host": r.host,
        "dns_ok": r.dns_ok,
        "http_ok": r.http_ok,
        "ok": r.ok,
        "status": r.status,
        "error": r.error,
        "duration_ms": r.duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_no_dns() {
        let r = probe_one("", 1.0);
        assert!(!r.dns_ok);
    }

    #[test]
    fn report_has_alive_dead() {
        let urls = vec!["not-a-valid-host-xyz.invalid".into()];
        let rep = precheck_report(&urls, 1.0, 2);
        assert!(rep.get("alive_urls").is_some());
        assert!(rep.get("dead_urls").is_some());
    }
}
