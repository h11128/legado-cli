//! Bulk check / channel / precheck CLI.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use source_check::{
    channel_status, dedupe_urls, load_alive_from_precheck, load_urls_file, precheck_report,
    run_batch_check, BatchCheckOpts,
};
use source_mcp::{force_clear_locks, repo_root};

use super::mcp_raw::{run_mcp_raw, McpRawCmd};

pub enum CheckCmd {
    Channel {
        clear_stale: bool,
        force_clear: bool,
        reset_remote: bool,
    },
    Precheck {
        urls_file: PathBuf,
        timeout: f64,
        concurrency: usize,
        out: Option<PathBuf>,
    },
    Batch {
        urls_file: PathBuf,
        keyword: String,
        batch_size: usize,
        thread_count: u32,
        timeout: f64,
        materials_dir: Option<PathBuf>,
        report_path: Option<PathBuf>,
    },
    Full {
        urls_file: PathBuf,
        keyword: String,
        batch_size: usize,
        thread_count: u32,
        timeout: f64,
        precheck_json: Option<PathBuf>,
        materials_dir: Option<PathBuf>,
        report_path: Option<PathBuf>,
    },
}

fn run_batch(opts: BatchCheckOpts) -> ExitCode {
    match run_batch_check(opts) {
        Ok(summary) => {
            println!(
                "{}",
                serde_json::json!({
                    "started": summary.started,
                    "success": summary.success,
                    "failed": summary.failed,
                    "by_failure_tag": summary.by_failure_tag,
                    "results": summary.results,
                })
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("check batch: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_check(cmd: CheckCmd) -> ExitCode {
    match cmd {
        CheckCmd::Channel {
            clear_stale,
            force_clear,
            reset_remote,
        } => {
            if force_clear {
                match repo_root().and_then(|r| force_clear_locks(&r)) {
                    Ok(n) => eprintln!("check channel: force_clear removed {n} lock(s)"),
                    Err(e) => {
                        eprintln!("check channel: force_clear: {e}");
                        return ExitCode::from(1);
                    }
                }
            } else if clear_stale {
                // status() already clears stale; explicit flag is documentation for agents.
                eprintln!("check channel: clear_stale (also runs on plain status)");
            }
            if reset_remote {
                eprintln!("check channel: reset_remote (phone-side reset_mcp_channel)");
                let rc = run_mcp_raw(McpRawCmd::ChannelReset);
                if rc != ExitCode::SUCCESS {
                    return rc;
                }
            }
            match channel_status() {
                Ok(v) => {
                    println!("{}", v);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("check channel: {e}");
                    ExitCode::from(1)
                }
            }
        }
        CheckCmd::Precheck {
            urls_file,
            timeout,
            concurrency,
            out,
        } => {
            let urls = match load_urls_file(&urls_file) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(4);
                }
            };
            let report = precheck_report(&urls, timeout, concurrency);
            if let Some(path) = out {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Ok(raw) = serde_json::to_string_pretty(&report) {
                    if fs::write(&path, raw).is_err() {
                        eprintln!("precheck: write {}", path.display());
                        return ExitCode::from(1);
                    }
                }
            }
            println!("{}", report);
            ExitCode::SUCCESS
        }
        CheckCmd::Batch {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            materials_dir,
            report_path,
        } => {
            let urls = match load_urls_file(&urls_file) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(4);
                }
            };
            run_batch(BatchCheckOpts {
                urls,
                keyword,
                thread_count,
                batch_size,
                timeout_ms: (timeout * 1000.0) as u64,
                check_discovery: false,
                materials_dir,
                report_path,
            })
        }
        CheckCmd::Full {
            urls_file,
            keyword,
            batch_size,
            thread_count,
            timeout,
            precheck_json,
            materials_dir,
            report_path,
        } => {
            let urls = match load_urls_file(&urls_file) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(4);
                }
            };
            let alive = if let Some(path) = precheck_json {
                match load_alive_from_precheck(&path) {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("check full: precheck json: {e}");
                        return ExitCode::from(4);
                    }
                }
            } else {
                let precheck_timeout = timeout.min(8.0);
                let report = precheck_report(&urls, precheck_timeout, 32);
                report
                    .get("alive_urls")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let alive = dedupe_urls(alive);
            eprintln!("check full: precheck alive {}/{}", alive.len(), urls.len());
            run_batch(BatchCheckOpts {
                urls: alive,
                keyword,
                thread_count,
                batch_size,
                timeout_ms: (timeout * 1000.0) as u64,
                check_discovery: false,
                materials_dir,
                report_path,
            })
        }
    }
}
