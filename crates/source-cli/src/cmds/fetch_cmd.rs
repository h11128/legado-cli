//! `source-cli fetch` — PC HTML fetch with source headers (Python `repair_source fetch`).

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use serde_json::{json, Value};
use source_mcp::{McpClient, McpEndpoint, McpSourceRepository};
use source_ports::SourceRepository;
use source_types::{HeaderMap, PortError, SourceKey, Url};

pub struct FetchArgs {
    pub url: String,
    pub page: Option<String>,
    pub dump_dir: PathBuf,
    pub out: Option<PathBuf>,
}

fn header_map(source: &Value) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(obj) = source.get("headerMap").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                h.insert(k.clone(), s.to_string());
            }
        }
    }
    if let Some(ua) = source.get("userAgent").and_then(|v| v.as_str()) {
        h.insert("User-Agent".into(), ua.to_string());
    }
    h
}

fn safe_name(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '_') {
                ch
            } else {
                '_'
            }
        })
        .take(100)
        .collect()
}

fn fetch_page(url: &str, headers: &HeaderMap) -> Result<(u16, String), PortError> {
    let u = Url::new(url).map_err(|e| PortError::Permanent(e.to_string()))?;
    let mut req = ureq::get(u.as_str());
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = req
        .call()
        .map_err(|e| PortError::Transient(format!("fetch {url}: {e}")))?;
    let status = resp.status();
    let body = resp
        .into_string()
        .map_err(|e| PortError::Transient(format!("read body: {e}")))?;
    Ok((status, body))
}

pub fn run_fetch(args: FetchArgs) -> ExitCode {
    let ep = match McpEndpoint::load_defaults() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("fetch: {e}");
            return ExitCode::from(4);
        }
    };
    let client = Arc::new(McpClient::new(ep).with_client_name("source_cli_fetch"));
    if let Err(e) = client.ensure_session() {
        eprintln!("fetch: session: {e}");
        return ExitCode::from(2);
    }
    let repo = McpSourceRepository::new(client);
    let source = match repo.get(&SourceKey::new(args.url.trim())) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fetch: get_source: {e}");
            return ExitCode::from(2);
        }
    };
    let page = args.page.as_deref().unwrap_or(args.url.trim()).to_string();
    let headers = header_map(source.as_value());
    let (status, body) = match fetch_page(&page, &headers) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fetch: {e}");
            return ExitCode::from(2);
        }
    };
    let host = url::Url::parse(&page)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.replace(':', "_")))
        .unwrap_or_else(|| "host".into());
    let _ = fs::create_dir_all(&args.dump_dir);
    let safe = safe_name(&page);
    let html_path = args.dump_dir.join(format!("{host}_{safe}.html"));
    let meta_path = args.dump_dir.join(format!("{host}_{safe}.json"));
    let meta = json!({
        "url": args.url,
        "page": page,
        "status": status,
        "headers": headers,
        "html_path": html_path.to_string_lossy(),
        "bytes": body.len(),
    });
    if fs::write(&html_path, &body).is_err()
        || fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        )
        .is_err()
    {
        eprintln!("fetch: write dump failed");
        return ExitCode::from(1);
    }
    if let Some(out) = args.out {
        if let Some(parent) = out.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            &out,
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&meta).unwrap_or_default()
    );
    ExitCode::SUCCESS
}
