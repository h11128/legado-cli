//! MCP helpers: clear cookies + push BookSource JSON from file.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use serde_json::{json, Value};
use source_mcp::{McpClient, McpEndpoint};

pub enum McpToolsCmd {
    ClearCookies { url: String },
    Push {
        file: PathBuf,
        /// When true: use JSON enabled/group (new drafts). When false: preserve phone flags.
        overwrite: bool,
    },
}

pub fn run_mcp_tools(cmd: McpToolsCmd) -> ExitCode {
    match cmd {
        McpToolsCmd::ClearCookies { url } => clear_cookies(&url),
        McpToolsCmd::Push { file, overwrite } => push_source(&file, overwrite),
    }
}

fn client() -> Result<Arc<McpClient>, String> {
    let ep = McpEndpoint::load_defaults().map_err(|e| e.to_string())?;
    Ok(Arc::new(McpClient::new(ep)))
}

fn clear_cookies(url: &str) -> ExitCode {
    // Cursor IDE MCP catalogs often omit `clear_cookies` even when the App exposes it —
    // this CLI entry is the SOT path (with eval_js fallback).
    let client = match client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("clear-cookies: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = client.ensure_session() {
        eprintln!("clear-cookies: session: {e}");
        return ExitCode::from(1);
    }
    match client.tools_call("clear_cookies", json!({ "url": url })) {
        Ok(v) => {
            println!("{}", McpClient::extract_text(&v));
            ExitCode::SUCCESS
        }
        Err(e) => {
            let msg = e.to_string();
            // Older App builds / filtered Cursor catalogs may lack the tool; try eval_js.
            if msg.contains("clear_cookies")
                || msg.contains("not found")
                || msg.contains("Unknown")
                || msg.contains("unknown")
                || msg.contains("Method not found")
            {
                eprintln!("clear-cookies: tool unavailable ({msg}); trying eval_js fallback");
                return clear_cookies_via_js(&client, url);
            }
            eprintln!("clear-cookies: {e}");
            ExitCode::from(1)
        }
    }
}

fn clear_cookies_via_js(client: &McpClient, url: &str) -> ExitCode {
    let js = format!(
        "cookie.removeCookie({u}); 'cleared:'+{u}",
        u = serde_json::to_string(url).unwrap_or_else(|_| format!("\"{url}\""))
    );
    match client.tools_call("eval_js", json!({ "js": js, "timeoutSec": 30 })) {
        Ok(v) => {
            println!("{}", McpClient::extract_text(&v));
            ExitCode::SUCCESS
        }
        Err(e2) => {
            eprintln!(
                "clear-cookies: fallback failed: {e2}\n\
                 Manual: App 书源编辑 → 清除 Cookie, or searchUrl @js: cookie.removeCookie('{url}')"
            );
            ExitCode::from(1)
        }
    }
}

fn push_source(file: &PathBuf, overwrite: bool) -> ExitCode {
    let raw = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("push: read {}: {e}", file.display());
            return ExitCode::from(1);
        }
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("push: JSON {}: {e}", file.display());
            return ExitCode::from(1);
        }
    };
    let Some(url) = value.get("bookSourceUrl").and_then(|u| u.as_str()) else {
        eprintln!("push: missing bookSourceUrl");
        return ExitCode::from(2);
    };
    let client = match client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("push: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = client.ensure_session() {
        eprintln!("push: session: {e}");
        return ExitCode::from(1);
    }
    let payload = match serde_json::to_string(&value) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("push: serialize: {e}");
            return ExitCode::from(1);
        }
    };
    let preserve = !overwrite;
    match client.tools_call(
        "save_source",
        json!({
            "source": payload,
            "format": "json",
            "preserveEnabled": preserve,
            "preserveGroup": preserve,
        }),
    ) {
        Ok(v) => {
            println!("{}", McpClient::extract_text(&v));
            eprintln!("push: ok url={url} overwrite={overwrite}");
            // Same anti-stall gate as diagnose/repair: must close-out before next site.
            // Claim failure is hard for create — otherwise stop/progress gates never arm.
            let claim = source_closeout::CloseoutPaths::from_repo().and_then(|paths| {
                source_closeout::claim_active(&paths, url, "create push").map(|_| ())
            });
            match claim {
                Ok(()) => {
                    eprintln!(
                        "push: claimed deep_active — REQUIRED before next site:\n\
                           source-cli ledger append --url {url} --step check --result '…'\n\
                           source-cli retro append --url {url} --status fixed|skip|fail \\\n\
                             --trap '…' --skill-fix 0|1 --script-fix '…'\n\
                         novel trap → update skill + diagnose_tips/harness, then git commit"
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!(
                        "push: FAIL claim deep_active: {e}\n\
                         source saved on device but create close-out gate is not armed.\n\
                         Fix paths/repo root, then: source-cli closeout claim --url {url} --note 'create push'"
                    );
                    ExitCode::from(2)
                }
            }
        }
        Err(e) => {
            eprintln!("push: {e}");
            ExitCode::from(1)
        }
    }
}
