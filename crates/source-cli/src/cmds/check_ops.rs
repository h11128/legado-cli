//! `source-cli check shard|disable-dead` — URL sharding + dead-source mutations.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use serde_json::json;
use source_check::{
    apply_disable_dead, apply_limit, default_rules_path, filter_alias_dead_urls, filter_urls,
    load_dead_urls, load_shard_urls_file, load_urls_file, plan_disable_dead, shard_urls,
    write_report, write_shards, DisableDeadOpts,
};
use source_mcp::{McpClient, McpEndpoint, McpSourceRepository};

pub enum CheckOpsCmd {
    Shard {
        urls_file: PathBuf,
        nodes: String,
        virtual_nodes: u32,
        out: PathBuf,
    },
    DisableDead {
        precheck_json: PathBuf,
        disable: bool,
        tag: bool,
        limit: usize,
        out: PathBuf,
        dry_run: bool,
    },
    Prefilter {
        urls_file: PathBuf,
        out: Option<PathBuf>,
        concurrency: usize,
        l2_timeout: f64,
        rules: Option<PathBuf>,
    },
}

pub fn run_check_ops(cmd: CheckOpsCmd) -> ExitCode {
    match run_inner(cmd) {
        Ok(v) => {
            println!("{v}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("check: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_inner(cmd: CheckOpsCmd) -> Result<serde_json::Value, String> {
    match cmd {
        CheckOpsCmd::Shard {
            urls_file,
            nodes,
            virtual_nodes,
            out,
        } => {
            let node_list: Vec<String> = nodes
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if node_list.is_empty() {
                return Err("nodes empty".into());
            }
            let urls = load_shard_urls_file(&urls_file).map_err(|e| e.to_string())?;
            let shards = shard_urls(&urls, &node_list, virtual_nodes).map_err(|e| e.to_string())?;
            write_shards(&out, &shards).map_err(|e| e.to_string())?;
            let summary: serde_json::Map<String, serde_json::Value> = shards
                .iter()
                .map(|(k, v)| (k.clone(), json!(v.len())))
                .collect();
            Ok(json!({
                "out": out.display().to_string(),
                "total": urls.len(),
                "counts": summary,
            }))
        }
        CheckOpsCmd::DisableDead {
            precheck_json,
            disable,
            tag,
            limit,
            out,
            dry_run,
        } => {
            let opts = DisableDeadOpts {
                disable,
                tag,
                limit,
            };
            opts.validate().map_err(|e| e.to_string())?;
            let report = if dry_run {
                plan_disable_dead(&precheck_json, &opts).map_err(|e| e.to_string())?
            } else {
                let (filtered, _skipped) = filter_alias_dead_urls(
                    load_dead_urls(&precheck_json).map_err(|e| e.to_string())?,
                );
                let dead = apply_limit(filtered, limit);
                // Alias bookSourceUrl labels are filtered before apply_limit.
                let ep = McpEndpoint::load_defaults().map_err(|e| e.to_string())?;
                let client = Arc::new(McpClient::new(ep).with_client_name("disable_dead"));
                client.ensure_session().map_err(|e| e.to_string())?;
                let repo = McpSourceRepository::new(client);
                apply_disable_dead(&repo, &dead, &opts).map_err(|e| e.to_string())?
            };
            write_report(&out, &report).map_err(|e| e.to_string())?;
            Ok(json!({
                "out": out.display().to_string(),
                "report": report,
            }))
        }
        CheckOpsCmd::Prefilter {
            urls_file,
            out,
            concurrency,
            l2_timeout,
            rules,
        } => {
            let urls = load_urls_file(&urls_file).map_err(|e| e.to_string())?;
            let rules_path = rules.unwrap_or_else(default_rules_path);
            let pref = filter_urls(&urls, &rules_path, concurrency, l2_timeout)
                .map_err(|e| e.to_string())?;
            let payload = json!({
                "total": pref.total,
                "verify_urls": pref.verify_urls,
                "skip": pref.skip,
                "disable": pref.disable,
                "video": pref.video,
                "hunt": pref.hunt,
                "results": pref.results,
            });
            if let Some(path) = out {
                std::fs::write(
                    &path,
                    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            }
            Ok(payload)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn shard_writes_out() {
        let dir = TempDir::new().unwrap();
        let urls = dir.path().join("urls.txt");
        std::fs::write(&urls, "https://a/\nhttps://b/\n").unwrap();
        let out = dir.path().join("shards.json");
        let v = run_inner(CheckOpsCmd::Shard {
            urls_file: urls,
            nodes: "phoneA,phoneB".into(),
            virtual_nodes: 64,
            out: out.clone(),
        })
        .unwrap();
        assert_eq!(v["total"], 2);
        assert!(out.is_file());
    }

    #[test]
    fn disable_dead_dry_run() {
        let dir = TempDir::new().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, r#"{"dead_urls":["https://dead/"]}"#).unwrap();
        let out = dir.path().join("report.json");
        let v = run_inner(CheckOpsCmd::DisableDead {
            precheck_json: pre,
            disable: true,
            tag: true,
            limit: 0,
            out: out.clone(),
            dry_run: true,
        })
        .unwrap();
        assert_eq!(v["report"]["dry_run"], true);
        assert_eq!(v["report"]["total"], 1);
        assert!(out.is_file());
    }
}
