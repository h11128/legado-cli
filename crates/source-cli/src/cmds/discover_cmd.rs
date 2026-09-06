//! MCP discover CLI.

use std::path::PathBuf;
use std::process::ExitCode;

use source_mcp::{apply_discovery, discover, repo_root, sync_cursor_mcp_json};

pub struct DiscoverArgs {
    pub write: bool,
    pub timeout: f64,
    pub config: Option<PathBuf>,
    pub sync_cursor: bool,
}

pub fn run_discover(args: DiscoverArgs) -> ExitCode {
    if args.write {
        let root = match repo_root() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("discover: {e}");
                return ExitCode::from(4);
            }
        };
        let path = args
            .config
            .unwrap_or_else(|| root.join("config/mcp_defaults.json"));
        match apply_discovery(true, args.timeout, &path) {
            Ok(found) => {
                println!("{}", found);
                if args.sync_cursor {
                    if let Some(def) = found.get("defaults") {
                        let mcp_url = def.get("mcp_url").and_then(|v| v.as_str()).unwrap_or("");
                        let token = def.get("token").and_then(|v| v.as_str()).unwrap_or("1234");
                        if !mcp_url.is_empty() {
                            println!("{}", sync_cursor_mcp_json(mcp_url, token));
                        }
                    }
                }
                if found.get("chosen").is_some() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("discover: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        match discover(args.timeout) {
            Ok(found) => {
                println!("{}", found);
                if found.get("found") == Some(&serde_json::json!(true)) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("discover: {e}");
                ExitCode::from(1)
            }
        }
    }
}
