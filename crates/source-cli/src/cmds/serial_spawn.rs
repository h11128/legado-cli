//! Spawn one `source-cli repair --mode oneshot` with wall-clock kill.

use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use source_mcp::{clear_stale_locks, repo_root, DualLedgerPort};
use source_ports::LedgerPort;
use source_spine::REPORT_JSON_PREFIX;
use source_types::{LedgerRow, LedgerStep, ReportStatus, Url};

use super::oneshot_live::repair_one_outcome;
use super::repair_outcome::parse_report_line;
use super::serial_cmd::SerialArgs;

pub(crate) fn serial_status(report: &Value) -> String {
    match report.get("status").and_then(|v| v.as_str()) {
        Some("fixed") => "fixed".into(),
        Some("skipped") => "skip".into(),
        Some("failed") => "fail".into(),
        other => other.unwrap_or("fail").to_string(),
    }
}

fn seal_url_timeout(url: &str, waste_s: f64) {
    let Ok(ledger) = DualLedgerPort::from_defaults() else {
        return;
    };
    let Ok(u) = Url::new(url) else {
        return;
    };
    let mut row = LedgerRow::new(
        chrono::Utc::now().to_rfc3339(),
        u,
        LedgerStep::Check,
        "skip:url_timeout",
    );
    row.report_status = Some(ReportStatus::Skipped);
    row.waste = Some(format!("{waste_s:.1}s"));
    row.note = Some("serial killed child after --url-timeout-s".into());
    let _ = ledger.append(&row);
}

fn timeout_report(url: &str, timeout_s: f64) -> Value {
    json!({
        "url": url,
        "status": "skipped",
        "message": format!("skip:url_timeout after {timeout_s:.0}s"),
        "notes": ["url_timeout"],
    })
}

pub(crate) fn run_one_in_process(url: &str, args: &SerialArgs) -> (ExitCode, Value) {
    let outcome = repair_one_outcome(
        url,
        args.rules.clone(),
        args.l0_only,
        args.tcp_timeout,
        args.l2_timeout,
        false,
        false,
        None,
        true,
        &args.keyword,
        false,
    );
    (outcome.exit, outcome.report)
}

pub(crate) fn run_one_subprocess(
    url: &str,
    args: &SerialArgs,
    heartbeat: &dyn Fn(Option<u32>),
) -> (ExitCode, Value) {
    let timeout = Duration::from_secs_f64(args.url_timeout_s.max(15.0));
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("serial: current_exe: {e}; falling back in-process");
            return run_one_in_process(url, args);
        }
    };
    let mut cmd = Command::new(&exe);
    cmd.arg("repair")
        .arg("--mode")
        .arg("oneshot")
        .arg("--url")
        .arg(url)
        .arg("--key")
        .arg(&args.keyword)
        .arg("--tcp-timeout")
        .arg(args.tcp_timeout.to_string())
        .arg("--l2-timeout")
        .arg(args.l2_timeout.to_string());
    if args.l0_only {
        cmd.arg("--l0-only");
    }
    if let Some(rules) = &args.rules {
        cmd.arg("--rules").arg(rules);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::inherit());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("serial: spawn repair: {e}; falling back in-process");
            return run_one_in_process(url, args);
        }
    };
    let child_pid = child.id();
    heartbeat(Some(child_pid));
    let stdout = child.stdout.take();
    let reader = thread::spawn(move || {
        let mut report: Option<Value> = None;
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                println!("{line}");
                if line.contains(REPORT_JSON_PREFIX) {
                    report = Some(parse_report_line(&line));
                }
            }
        }
        report
    });
    let started = Instant::now();
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break true;
                }
                thread::sleep(Duration::from_millis(400));
            }
            Err(_) => {
                let _ = child.kill();
                break true;
            }
        }
    };
    let report = reader.join().ok().flatten();
    if timed_out {
        if let Ok(root) = repo_root() {
            let _ = clear_stale_locks(&root);
        }
        let waste = started.elapsed().as_secs_f64();
        seal_url_timeout(url, waste);
        eprintln!(
            "serial: url_timeout {:.0}s → kill child pid={child_pid} url={url}",
            args.url_timeout_s
        );
        return (ExitCode::SUCCESS, timeout_report(url, args.url_timeout_s));
    }
    let report = report.unwrap_or_else(
        || json!({"url": url, "status": "failed", "message": "no REPORT_JSON from child"}),
    );
    let st = serial_status(&report);
    let exit = if st == "fixed" || st == "skip" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    (exit, report)
}
