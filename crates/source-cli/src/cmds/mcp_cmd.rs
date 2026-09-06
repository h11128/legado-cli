//! MCP multi-endpoint CLI command implementation.

use std::process::ExitCode;
use crate::cli_subs::McpSub;
use source_mcp::{probe_mcp, repo_root, sync_cursor_mcp_json, McpEndpoint, McpEndpointRecord};

pub fn run_mcp(cmd: McpSub) -> ExitCode {
    let root = match repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error locating repo root: {e}");
            return ExitCode::from(1);
        }
    };
    let path = root.join("config/mcp_defaults.json");

    match cmd {
        McpSub::List { probe } => {
            let cfg = match McpEndpoint::load_config(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error reading mcp_defaults: {e}");
                    return ExitCode::from(1);
                }
            };
            println!("Active MCP: {}", cfg.mcp_url);
            println!("{:<2} {:<35} {:<8} {:<12} {:<10} {:<15}", "  ", "MCP URL", "TOKEN", "STATUS", "LAST SEEN", "NOTE");
            println!("{}", "-".repeat(85));
            for ep in &cfg.endpoints {
                let is_active = ep.mcp_url == cfg.mcp_url;
                let marker = if is_active { "*" } else { " " };
                let status = if probe {
                    if probe_mcp(&ep.mcp_url, &ep.token, 1.5) {
                        "ALIVE"
                    } else {
                        "DOWN"
                    }
                } else if is_active {
                    "ACTIVE"
                } else {
                    "-"
                };
                let last_seen = ep.last_seen.as_deref().unwrap_or("-");
                let note = ep.note.as_deref().unwrap_or("");
                println!("{:<2} {:<35} {:<8} {:<12} {:<10} {:<15}", marker, ep.mcp_url, ep.token, status, last_seen, note);
            }
            ExitCode::SUCCESS
        }
        McpSub::Add { url, token, note, switch } => {
            let host = url
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(':')
                .next()
                .unwrap_or("127.0.0.1");
            let web_api = format!("http://{host}:1122");
            let record = McpEndpointRecord {
                mcp_url: url.clone(),
                token: token.clone(),
                web_api: Some(web_api),
                last_seen: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                note,
            };
            if let Err(e) = McpEndpoint::add_or_update_endpoint(&path, record, switch) {
                eprintln!("failed to add endpoint: {e}");
                return ExitCode::from(1);
            }
            if switch {
                let _ = sync_cursor_mcp_json(&url, &token);
                println!("Saved and switched active MCP to: {url}");
            } else {
                println!("Saved remembered MCP endpoint: {url}");
            }
            ExitCode::SUCCESS
        }
        McpSub::Switch { url } => {
            match McpEndpoint::switch_active(&path, &url) {
                Ok(ep) => {
                    let _ = sync_cursor_mcp_json(&ep.mcp_url, &ep.token);
                    println!("Switched active MCP to: {}", ep.mcp_url);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("failed to switch endpoint: {e}");
                    ExitCode::from(1)
                }
            }
        }
        McpSub::Remove { url } => {
            match McpEndpoint::remove_endpoint(&path, &url) {
                Ok(true) => {
                    println!("Removed MCP endpoint: {url}");
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    println!("Endpoint not found in list: {url}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("failed to remove endpoint: {e}");
                    ExitCode::from(1)
                }
            }
        }
        McpSub::Probe { timeout } => {
            let endpoints = McpEndpoint::load_endpoints(&path);
            if endpoints.is_empty() {
                eprintln!("no remembered endpoints found in mcp_defaults.json");
                return ExitCode::from(1);
            }
            println!("Probing {} remembered endpoint(s)...", endpoints.len());
            for ep in &endpoints {
                print!("  Checking {} ... ", ep.mcp_url);
                if probe_mcp(&ep.mcp_url, &ep.token, timeout) {
                    println!("ALIVE");
                    if let Ok(switched) = McpEndpoint::switch_active(&path, &ep.mcp_url) {
                        let _ = sync_cursor_mcp_json(&switched.mcp_url, &switched.token);
                        println!("Active MCP switched to: {}", switched.mcp_url);
                        return ExitCode::SUCCESS;
                    }
                } else {
                    println!("DOWN");
                }
            }
            eprintln!("All remembered endpoints are unreachable.");
            ExitCode::from(1)
        }
    }
}
