//! Serial repair orchestration: heartbeat, retro, per-URL timeout via subprocess.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use source_check::load_urls_file;
use source_closeout::{append_retro, CloseoutPaths, RetroAppendOpts};
use source_mcp::repo_root;

use super::serial_spawn::{run_one_in_process, run_one_subprocess, serial_status};

pub struct SerialArgs {
    pub urls_file: PathBuf,
    pub limit: usize,
    pub keyword: String,
    pub l0_only: bool,
    pub tcp_timeout: f64,
    pub l2_timeout: f64,
    pub rules: Option<PathBuf>,
    pub rebuild_queue: bool,
    pub auto_retro: bool,
    pub out: Option<PathBuf>,
    /// Per-URL wall clock; kill child repair and continue. 0 = in-process (no kill).
    pub url_timeout_s: f64,
    /// Force in-process oneshot (ignore url_timeout_s kill path).
    pub in_process: bool,
}

fn serial_trap_harness(report: &Value, waste_s: f64, status: &str) -> (String, String, String) {
    let notes_join = report
        .get("notes")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let mut trap = String::new();
    let mut harness = String::new();
    let mut script_fix = String::new();
    if notes_join.contains("url_timeout") {
        trap = "serial_url_timeout".into();
        script_fix = "source-cli/serial_spawn".into();
        harness = "url_timeout_kill".into();
    } else if notes_join.contains("js_api") {
        trap = "js_search_api".into();
        script_fix = "source-probe/js_engine".into();
    }
    if harness.is_empty() {
        if waste_s > 120.0 {
            harness = "over_budget_>2min".into();
        } else if status == "fail" && waste_s > 60.0 {
            harness = "fail_after_long_probe".into();
        } else if status == "skip" && notes_join.contains("l2") {
            harness = "ok_failfast_l2".into();
            if script_fix.is_empty() {
                script_fix = "source-check/prefilter".into();
            }
        }
    }
    (trap, harness, script_fix)
}

fn append_serial_retro(paths: &CloseoutPaths, url: &str, name: &str, report: &Value, waste_s: f64) {
    let status = serial_status(report);
    let msg = report
        .get("message")
        .or_else(|| report.get("msg"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect::<String>();
    let (trap, harness, script_fix) = serial_trap_harness(report, waste_s, &status);
    if let Err(errs) = append_retro(
        paths,
        RetroAppendOpts {
            url: url.to_string(),
            status,
            msg,
            name: name.to_string(),
            respond_time: report.get("respondTime").and_then(|v| v.as_i64()),
            waste_s,
            trap,
            harness,
            script_fix,
            skill_fix: false,
            seal: false,
        },
    ) {
        for e in errs {
            eprintln!("serial retro: {e}");
        }
    }
}

fn now_s() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn write_heartbeat(n: usize, url: &str, child_pid: Option<u32>) {
    let Ok(root) = repo_root() else {
        return;
    };
    let path = root.join("temp/full_fix/serial_heartbeat.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = json!({
        "n": n,
        "url": url,
        "parent_pid": std::process::id(),
        "child_pid": child_pid,
        "started_at": now_s(),
        "ts": chrono::Utc::now().to_rfc3339(),
    });
    let _ = std::fs::write(path, body.to_string());
}

fn write_summary(path: &Path, summary: &Value) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        path,
        serde_json::to_string_pretty(summary).unwrap_or_default(),
    );
}

pub fn run_serial_cmd(args: SerialArgs) -> ExitCode {
    if args.rebuild_queue {
        use source_queue::{build_rt_queue_full, default_serial_queue_path, RtBuildOpts};
        let index = PathBuf::from("temp/full_fix/phone_source_index.json");
        let out = default_serial_queue_path()
            .unwrap_or_else(|_| PathBuf::from("temp/full_fix/queues/repair_serial100_queue.json"));
        if let Ok(doc) = build_rt_queue_full(
            &index,
            &RtBuildOpts {
                max_rt_ms: 8000,
                limit: args.limit.max(1),
                enabled_only: true,
                search_tag_only: true,
                all_sources_path: None,
                ledger_path: None,
            },
        ) {
            if let Some(parent) = out.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&out, serde_json::to_string_pretty(&doc).unwrap_or_default());
            eprintln!("serial: rebuilt RT queue → {}", out.display());
        }
    }
    let urls = match load_urls_file(&args.urls_file) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("serial: {e}");
            return ExitCode::from(4);
        }
    };
    let closeout = if args.auto_retro {
        CloseoutPaths::from_repo().ok()
    } else {
        None
    };
    let lim = args.limit.max(1);
    let use_sub = !args.in_process && args.url_timeout_s > 0.0;
    let mut last = ExitCode::SUCCESS;
    let t0_all = Instant::now();
    let mut summary = json!({
        "n_in": urls.len(),
        "limit": lim,
        "url_timeout_s": args.url_timeout_s,
        "in_process": !use_sub,
        "results": [],
    });
    let out_path = args
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from("temp/full_fix/serial_last.json"));
    for (idx, url) in urls.into_iter().take(lim).enumerate() {
        let n = idx + 1;
        write_heartbeat(n, &url, None);
        eprintln!(
            "serial: [{n}/{lim}] {url} (timeout={}s subprocess={use_sub})",
            args.url_timeout_s
        );
        let t0 = Instant::now();
        let (exit, report) = if use_sub {
            run_one_subprocess(&url, &args, &|pid| write_heartbeat(n, &url, pid))
        } else {
            run_one_in_process(&url, &args)
        };
        let waste_s = t0.elapsed().as_secs_f64();
        let exit_code = if exit == ExitCode::SUCCESS { 0 } else { 1 };
        let name = report
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(paths) = closeout.as_ref() {
            append_serial_retro(paths, &url, &name, &report, waste_s);
        }
        summary["results"]
            .as_array_mut()
            .expect("results array")
            .push(json!({
                "n": n,
                "url": url,
                "exit": exit_code,
                "status": serial_status(&report),
                "waste_s": waste_s,
                "name": name,
            }));
        summary["elapsed_s"] = json!(t0_all.elapsed().as_secs_f64());
        write_summary(&out_path, &summary);
        if exit != ExitCode::SUCCESS {
            last = exit;
        }
    }
    summary["elapsed_s"] = json!(t0_all.elapsed().as_secs_f64());
    write_summary(&out_path, &summary);
    println!("{}", summary);
    last
}
