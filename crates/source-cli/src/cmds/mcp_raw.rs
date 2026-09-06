//! Thin 1:1 wrappers over remaining `mcp__legado__*` tools that had no CLI
//! equivalent yet (`debug_source`, `list_sources`, `get_source`, `delete_sources`,
//! http-log tools, cookie tools, `reset_mcp_channel`). Called from `source_cmd.rs`
//! (`source debug/get/list/delete`) and `check_cmd.rs` (`check cookie-*` /
//! `check log-*` / `check channel --reset-remote`) — kept in one file since they
//! share the same thin call/print shape, but each surfaces under whichever
//! existing command family it actually belongs to, not a standalone group.
//! Every command here re-reads `mcp_defaults.json` per invocation via
//! `McpEndpoint::load_defaults()` — no session restart needed when the phone's
//! LAN address changes (unlike an IDE's persistent MCP client).

use std::process::ExitCode;
use std::sync::Arc;

use serde_json::{json, Value};
use source_mcp::{McpClient, McpEndpoint, McpSourceRepository};
use source_ports::SourceRepository;
use source_types::SourceKey;

pub enum McpRawCmd {
    Debug {
        url: String,
        key: String,
        timeout_sec: u64,
    },
    Get {
        url: String,
    },
    List {
        search: Option<String>,
    },
    Delete {
        urls: Vec<String>,
    },
    LogRecording {
        enabled: bool,
    },
    LogList {
        limit: u32,
    },
    LogGet {
        id: i64,
    },
    CookieGet {
        url: String,
    },
    CookieSet {
        url: String,
        cookie: String,
    },
    ChannelReset,
}

fn client() -> Result<Arc<McpClient>, String> {
    let ep = McpEndpoint::load_defaults().map_err(|e| e.to_string())?;
    Ok(Arc::new(McpClient::new(ep).with_client_name("mcp_raw")))
}

fn ensure(client: &McpClient) -> Result<(), String> {
    client.ensure_session().map_err(|e| e.to_string())
}

fn call_and_print(client: &McpClient, tool: &str, args: Value) -> ExitCode {
    match client.tools_call(tool, args) {
        Ok(v) => {
            let text = McpClient::extract_text(&v);
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mcp {tool}: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_mcp_raw(cmd: McpRawCmd) -> ExitCode {
    let client = match client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mcp: {e}");
            return ExitCode::from(4);
        }
    };
    if let Err(e) = ensure(&client) {
        eprintln!("mcp: session: {e}");
        return ExitCode::from(2);
    }

    match cmd {
        McpRawCmd::Debug {
            url,
            key,
            timeout_sec,
        } => call_and_print(
            &client,
            "debug_source",
            json!({ "url": url, "key": key, "timeoutSec": timeout_sec }),
        ),
        McpRawCmd::Get { url } => {
            // Reuses the same phone-cache + url-fallback logic as `source triage`/`repair`,
            // but prints the full raw BookSource JSON instead of a triage summary.
            let repo = McpSourceRepository::new(client);
            match repo.get(&SourceKey::new(&url)) {
                Ok(src) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(src.as_value()).unwrap_or_default()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("mcp get: {e}");
                    ExitCode::from(1)
                }
            }
        }
        McpRawCmd::List { search } => {
            let mut args = serde_json::Map::new();
            if let Some(s) = search {
                args.insert("search".into(), json!(s));
            }
            call_and_print(&client, "list_sources", Value::Object(args))
        }
        McpRawCmd::Delete { urls } => {
            let repo = McpSourceRepository::new(client);
            let keys: Vec<SourceKey> = urls.iter().map(|u| SourceKey::new(u)).collect();
            match repo.delete(&keys) {
                Ok(()) => {
                    println!("deleted {} source(s)", urls.len());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("mcp delete: {e}");
                    ExitCode::from(1)
                }
            }
        }
        McpRawCmd::LogRecording { enabled } => call_and_print(
            &client,
            "set_http_log_recording",
            json!({ "enabled": enabled }),
        ),
        McpRawCmd::LogList { limit } => {
            call_and_print(&client, "get_http_logs", json!({ "limit": limit }))
        }
        McpRawCmd::LogGet { id } => call_and_print(&client, "get_http_log", json!({ "id": id })),
        McpRawCmd::CookieGet { url } => {
            call_and_print(&client, "get_cookies", json!({ "url": url }))
        }
        McpRawCmd::CookieSet { url, cookie } => call_and_print(
            &client,
            "set_cookie",
            json!({ "url": url, "cookie": cookie }),
        ),
        McpRawCmd::ChannelReset => call_and_print(&client, "reset_mcp_channel", json!({})),
    }
}
