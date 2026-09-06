//! One-entry deep dig: channel → gate → diagnose → optional oneshot.

use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

use super::check_cmd::{run_check, CheckCmd};
use super::diagnose::{run_diagnose, DiagnoseArgs};
use super::gate::{run_gate, GateArgs};
use super::oneshot_live::repair_one_url;

pub struct DigArgs {
    pub url: String,
    pub key: String,
    pub rules: Option<PathBuf>,
    pub l0_only: bool,
    pub tcp_timeout: f64,
    pub l2_timeout: f64,
    /// Skip `repair --mode oneshot` after diagnose (diagnose-only).
    pub no_repair: bool,
    pub dry_run: bool,
    pub no_verify: bool,
}

pub fn run_dig(args: DigArgs) -> ExitCode {
    eprintln!("dig: check channel…");
    let ch = run_check(CheckCmd::Channel {
        clear_stale: true,
        force_clear: false,
        reset_remote: false,
    });
    if ch != ExitCode::SUCCESS {
        eprintln!("dig: channel not idle — clear bulk/repair lock first");
        return ch;
    }

    eprintln!("dig: gate…");
    let _ = run_gate(GateArgs {
        url: args.url.clone(),
        rules: args.rules.clone(),
        l0_only: args.l0_only,
        tcp_timeout: args.tcp_timeout,
        l2_timeout: args.l2_timeout,
    });

    eprintln!("dig: diagnose…");
    let diag = run_diagnose(DiagnoseArgs {
        url: args.url.clone(),
        key: args.key.clone(),
        rules: args.rules.clone(),
        l0_only: args.l0_only,
        tcp_timeout: args.tcp_timeout,
        l2_timeout: args.l2_timeout,
        debug_file: None,
        out: None,
    });
    // 3 = gate-blocked diagnose skip — still an official diagnose artifact.
    if diag != ExitCode::SUCCESS && diag != ExitCode::from(3) {
        eprintln!("dig: diagnose failed (exit {diag:?}) — MCP fallback only if transport dead");
        return diag;
    }

    if args.no_repair {
        println!(
            "{}",
            json!({
                "dig": "diagnose_only",
                "url": args.url,
                "next": format!(
                    "source-cli repair --mode oneshot --url {} --key {}",
                    args.url, args.key
                ),
            })
        );
        return ExitCode::SUCCESS;
    }

    eprintln!("dig: repair oneshot…");
    // Dig already ran diagnose (+ artifact); skip oneshot's second diagnose.
    let rep = repair_one_url(
        &args.url,
        args.rules,
        args.l0_only,
        args.tcp_timeout,
        args.l2_timeout,
        args.dry_run,
        args.no_verify,
        None,
        true,
        &args.key,
        true, // skip_diagnose
    );
    eprintln!(
        "dig: done — close-out: ledger append + retro append (trap/script_fix) before next URL"
    );
    rep
}
