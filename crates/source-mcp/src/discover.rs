//! MCP LAN discovery — adb/dns-sd/subnet probe + Cursor `mcp.json` sync.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};
use source_types::PortError;
use url::Url;

use crate::endpoint::McpEndpoint;
use crate::root::repo_root;

const DEFAULT_PORT: u16 = 1236;
const MCP_PATH: &str = "/mcp";

pub fn mcp_url_for(host: &str, port: u16) -> String {
    let host = host.trim().trim_matches(['[', ']']);
    format!("http://{host}:{port}{MCP_PATH}")
}

pub fn probe_mcp(url: &str, token: &str, timeout_s: f64) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs_f64(timeout_s.max(1.0)))
        .build();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "source_mcp_discover", "version": "1.0"},
        }
    });
    match agent
        .post(url)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json, text/event-stream")
        .set("X-Legado-Token", token)
        .send_string(&body.to_string())
    {
        Ok(r) => {
            let st = r.status();
            (200..500).contains(&st)
        }
        Err(ureq::Error::Status(code, _)) => matches!(code, 401 | 403 | 406 | 415),
        Err(_) => false,
    }
}

fn adb_wlan_ips() -> Vec<String> {
    let out = Command::new("adb")
        .args(["shell", "ip", "-f", "inet", "addr", "show", "wlan0"])
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    if !out.status.success() {
        return adb_phone_ip_route().into_iter().collect();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut ips = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.trim().split("inet ").nth(1) {
            let ip = rest.split('/').next().unwrap_or("").trim();
            if !ip.is_empty() && !ip.starts_with("127.") {
                ips.push(ip.to_string());
            }
        }
    }
    if ips.is_empty() {
        ips.extend(adb_phone_ip_route());
    }
    ips
}

fn adb_phone_ip_route() -> Option<String> {
    let out = Command::new("adb")
        .args(["shell", "ip", "route"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.contains("wlan") {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                return Some(parts[8].to_string());
            }
        }
    }
    None
}

fn discover_adb(token: &str, port: u16, timeout_s: f64) -> Vec<Value> {
    let mut out = Vec::new();
    for ip in adb_wlan_ips() {
        let url = mcp_url_for(&ip, port);
        if probe_mcp(&url, token, timeout_s) {
            out.push(json!({
                "host": ip,
                "port": port,
                "mcp_url": url,
                "via": "adb",
            }));
        }
    }
    out
}

fn discover_dns_sd(token: &str, port: u16, timeout_s: f64) -> Vec<Value> {
    let Ok(out) = Command::new("dns-sd")
        .args(["-B", "_legado-mcp._tcp", "local."])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut hits = Vec::new();
    for line in text.lines() {
        if !line.contains("_legado-mcp._tcp") {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if let Some(name) = parts.last() {
            if let Ok(r) = Command::new("dns-sd")
                .args(["-L", name, "_legado-mcp._tcp", "local."])
                .output()
            {
                let body = String::from_utf8_lossy(&r.stdout);
                if let Some(host) = body
                    .split("can be reached at")
                    .nth(1)
                    .and_then(|s| s.split(':').next())
                {
                    let host = host.trim().trim_end_matches('.');
                    let url = mcp_url_for(host, port);
                    if probe_mcp(&url, token, timeout_s) {
                        hits.push(json!({
                            "host": host,
                            "port": port,
                            "mcp_url": url,
                            "via": "dns-sd",
                            "name": name,
                        }));
                    }
                }
            }
        }
    }
    hits
}

fn subnet_prefix(host: &str) -> Option<String> {
    let parts: Vec<_> = host.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        return Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]));
    }
    None
}

fn discover_subnet(seed_host: &str, token: &str, port: u16, timeout_s: f64) -> Vec<Value> {
    let Some(prefix) = subnet_prefix(seed_host) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for last in 1u8..=254 {
        let ip = format!("{prefix}.{last}");
        if ip == seed_host {
            continue;
        }
        let url = mcp_url_for(&ip, port);
        if probe_mcp(&url, token, timeout_s.min(1.5)) {
            hits.push(json!({
                "host": ip,
                "port": port,
                "mcp_url": url,
                "via": "subnet",
            }));
            break;
        }
    }
    hits
}

pub fn discover_all(token: &str, timeout_s: f64) -> Vec<Value> {
    let mut hits = discover_dns_sd(token, DEFAULT_PORT, timeout_s);
    if hits.is_empty() {
        hits = discover_adb(token, DEFAULT_PORT, timeout_s);
    }
    if hits.is_empty() {
        if let Ok(ep) = McpEndpoint::load_defaults() {
            if let Ok(u) = Url::parse(&ep.mcp_url) {
                if let Some(h) = u.host_str() {
                    hits = discover_subnet(h, token, DEFAULT_PORT, timeout_s);
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    hits.into_iter()
        .filter(|h| {
            h.get("mcp_url")
                .and_then(|v| v.as_str())
                .is_some_and(|u| seen.insert(u.to_string()))
        })
        .collect()
}

pub fn discover(timeout_s: f64) -> Result<Value, PortError> {
    let defaults = McpEndpoint::load_defaults().ok();
    let token = defaults
        .as_ref()
        .map(|d| d.token.clone())
        .unwrap_or_else(|| "1234".into());

    let hits = discover_all(&token, timeout_s);
    if let Some(chosen) = hits.first() {
        let host = chosen
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1");
        return Ok(json!({
            "found": true,
            "mcp_url": chosen.get("mcp_url"),
            "web_api": format!("http://{host}:1122"),
            "token": token,
            "via": chosen.get("via").cloned().unwrap_or(json!("probe")),
            "hits": hits,
        }));
    }

    let mut candidates: Vec<String> = Vec::new();
    if let Some(ep) = &defaults {
        if let Ok(u) = Url::parse(&ep.mcp_url) {
            if let Some(h) = u.host_str() {
                candidates.push(h.to_string());
            }
        }
    }
    candidates.extend(adb_wlan_ips());
    candidates.sort();
    candidates.dedup();
    Ok(json!({ "found": false, "candidates": candidates, "hits": hits }))
}

pub fn sync_cursor_mcp_json(mcp_url: &str, token: &str) -> Value {
    let mcp_json = dirs_home().join(".cursor").join("mcp.json");
    let mut out = json!({
        "path": mcp_json.to_string_lossy(),
        "updated": false,
    });
    if !mcp_json.is_file() {
        out["error"] = json!("missing");
        return out;
    }
    let Ok(raw) = fs::read_to_string(&mcp_json) else {
        out["error"] = json!("read");
        return out;
    };
    let Ok(mut cfg) = serde_json::from_str::<Value>(&raw) else {
        out["error"] = json!("json");
        return out;
    };
    let Some(servers) = cfg.get_mut("mcpServers").and_then(|v| v.as_object_mut()) else {
        out["error"] = json!("no mcpServers");
        return out;
    };
    let key = if servers.contains_key("legado") {
        "legado"
    } else if servers.contains_key("user-legado") {
        "user-legado"
    } else {
        out["error"] = json!("no legado server entry");
        return out;
    };
    let Some(entry) = servers.get_mut(key).and_then(|v| v.as_object_mut()) else {
        out["error"] = json!("legado entry not an object");
        return out;
    };
    let old = entry
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let old_tok = entry
        .get("headers")
        .and_then(|h| h.get("X-Legado-Token"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if old == mcp_url && old_tok == token {
        out["unchanged"] = json!(true);
        return out;
    }
    entry.insert("url".into(), json!(mcp_url));
    let headers = entry
        .entry("headers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("headers object");
    headers.insert("X-Legado-Token".into(), json!(token));
    headers.insert(
        "X-Legado-Client".into(),
        json!(format!(
            "discover-{}",
            chrono::Utc::now().format("%Y-%m-%d")
        )),
    );
    if fs::write(
        &mcp_json,
        serde_json::to_string_pretty(&cfg).unwrap_or_default() + "\n",
    )
    .is_ok()
    {
        out["updated"] = json!(true);
        out["old_url"] = json!(old);
        out["new_url"] = json!(mcp_url);
    } else {
        out["error"] = json!("write");
    }
    out
}

fn dirs_home() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn apply_discovery(write: bool, timeout_s: f64, path: &Path) -> Result<Value, PortError> {
    let data = if path.is_file() {
        serde_json::from_str::<Value>(
            &fs::read_to_string(path)
                .map_err(|e| PortError::Permanent(format!("read {}: {e}", path.display())))?,
        )
        .map_err(|e| PortError::Permanent(format!("json: {e}")))?
    } else {
        json!({})
    };
    let token = data.get("token").and_then(|v| v.as_str()).unwrap_or("1234");
    let hits = discover_all(token, timeout_s);
    let chosen = hits.first().cloned();
    let mut result = json!({
        "hits": hits,
        "chosen": chosen,
        "defaults_path": path.display().to_string(),
        "wrote": false,
    });
    let Some(ch) = chosen else {
        return Ok(result);
    };
    let mcp_url = ch
        .get("mcp_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PortError::Permanent("chosen missing mcp_url".into()))?;
    let host = ch
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1");
    let mut out_data = data.clone();
    out_data["mcp_url"] = json!(mcp_url);
    out_data["token"] = json!(token);
    out_data["web_api"] = json!(format!("http://{host}:1122"));
    out_data["updated"] = json!(chrono::Utc::now().format("%Y-%m-%d").to_string());
    out_data["discovered_via"] = ch.get("via").cloned().unwrap_or(json!("probe"));
    out_data["note"] = json!("Single SOT for phone LAN endpoint. Updated by source-cli discover.");
    if write {
        write_defaults(
            &json!({
                "found": true,
                "mcp_url": mcp_url,
                "token": token,
                "web_api": out_data.get("web_api"),
                "via": ch.get("via"),
            }),
            path,
        )?;
        result["wrote"] = json!(true);
        result["agent_sync"] = json!({
            "cursor_mcp_json": sync_cursor_mcp_json(mcp_url, token),
        });
    }
    result["defaults"] = out_data;
    Ok(result)
}

/// Return reachable endpoint; rediscover+write if current URL is dead.
pub fn ensure_reachable(path: Option<&Path>, timeout_s: f64) -> Result<McpEndpoint, PortError> {
    let path = path.map(Path::to_path_buf).unwrap_or_else(|| {
        repo_root()
            .unwrap_or_default()
            .join("config/mcp_defaults.json")
    });
    let current = McpEndpoint::load_path(&path).ok();
    if let Some(ep) = &current {
        if probe_mcp(&ep.mcp_url, &ep.token, timeout_s) {
            // Ensure current active endpoint is in endpoints list
            let _ = McpEndpoint::add_or_update_endpoint(
                &path,
                crate::endpoint::McpEndpointRecord {
                    mcp_url: ep.mcp_url.clone(),
                    token: ep.token.clone(),
                    web_api: None,
                    last_seen: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    note: None,
                },
                false,
            );
            return Ok(ep.clone());
        }
    }
    // Probe remembered endpoints before full network scan
    let endpoints = McpEndpoint::load_endpoints(&path);
    for cand in &endpoints {
        if let Some(cur) = &current {
            if cand.mcp_url == cur.mcp_url {
                continue;
            }
        }
        if probe_mcp(&cand.mcp_url, &cand.token, timeout_s.min(3.0)) {
            if let Ok(switched) = McpEndpoint::switch_active(&path, &cand.mcp_url) {
                let _ = sync_cursor_mcp_json(&switched.mcp_url, &switched.token);
                return Ok(switched);
            }
        }
    }
    let discovered = apply_discovery(true, timeout_s.max(4.0), &path)?;
    if discovered.get("wrote") == Some(&json!(true)) {
        return McpEndpoint::load_path(&path);
    }
    current
        .ok_or_else(|| PortError::Transient("MCP unreachable and discovery found nothing".into()))
}

pub fn write_defaults(discovery: &Value, path: &Path) -> Result<(), PortError> {
    if discovery.get("found") != Some(&json!(true)) {
        return Err(PortError::Permanent("no MCP endpoint discovered".into()));
    }
    let mcp_url = discovery
        .get("mcp_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PortError::Permanent("missing mcp_url".into()))?;
    let token = discovery
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("1234");
    let web_api = discovery
        .get("web_api")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1122");
    // Keep timeout knobs across discover rewrites (SOT: mcp_defaults.json).
    let timeouts = crate::timeouts::timeouts_from_existing_defaults(path);
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Preserve and update endpoints list
    let mut endpoints = McpEndpoint::load_endpoints(path);
    let mut found = false;
    for ep in &mut endpoints {
        if ep.mcp_url == mcp_url {
            ep.token = token.to_string();
            ep.web_api = Some(web_api.to_string());
            ep.last_seen = Some(now.clone());
            found = true;
            break;
        }
    }
    if !found {
        endpoints.insert(
            0,
            crate::endpoint::McpEndpointRecord {
                mcp_url: mcp_url.to_string(),
                token: token.to_string(),
                web_api: Some(web_api.to_string()),
                last_seen: Some(now.clone()),
                note: discovery.get("via").and_then(|v| v.as_str()).map(|s| s.to_string()),
            },
        );
    }

    let mut payload = json!({
        "mcp_url": mcp_url,
        "token": token,
        "web_api": web_api,
        "updated": now,
        "note": "Single SOT for phone LAN endpoint + MCP timeouts. Updated by source-cli discover.",
        "discovered_via": discovery.get("via").cloned().unwrap_or(json!("probe")),
        "endpoints": endpoints,
    });
    payload = crate::timeouts::McpTimeouts::merge_into_defaults_json(&payload, &timeouts);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| PortError::Permanent(e.to_string()))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&payload).map_err(|e| PortError::Permanent(e.to_string()))?
            + "\n",
    )
    .map_err(|e| PortError::Permanent(e.to_string()))?;
    Ok(())
}
