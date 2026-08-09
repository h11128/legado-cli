//! Single-URL live repair: gate → hunt-before-disable → MCP oneshot.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use source_gate::{classify_one_l0, load_rules};
use source_mcp::{FsChannelPort, McpClient, McpEndpoint, McpSourceRepository};
use source_ports::{ChannelPort, SourceRepository};
use source_types::{GateAction, SourceKey};

use super::oneshot_finish::run_after_gate;
use super::oneshot_hunt::{
    apply_hunt_migrate, depth_exceeded_outcome, hunt_depth_ok, resolve_hunt_gate,
};
use super::repair_outcome::RepairOneOutcome;

#[cfg(feature = "gate_full")]
use source_gate::{classify_one, ClassifyOpts};

fn default_rules_path() -> PathBuf {
    let candidates = [
        PathBuf::from("config/verify_skip_rules.json"),
        PathBuf::from("../config/verify_skip_rules.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json"),
    ];
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json")
}

#[allow(clippy::too_many_arguments)]
pub fn repair_one_url(
    url: &str,
    rules: Option<PathBuf>,
    l0_only: bool,
    tcp_timeout: f64,
    l2_timeout: f64,
    dry_run: bool,
    no_verify: bool,
    html: Option<PathBuf>,
    prefetch: bool,
    key: &str,
    skip_diagnose: bool,
) -> ExitCode {
    repair_one_outcome(
        url,
        rules,
        l0_only,
        tcp_timeout,
        l2_timeout,
        dry_run,
        no_verify,
        html,
        prefetch,
        key,
        skip_diagnose,
    )
    .exit
}

#[allow(clippy::too_many_arguments)]
pub fn repair_one_outcome(
    url: &str,
    rules: Option<PathBuf>,
    l0_only: bool,
    tcp_timeout: f64,
    l2_timeout: f64,
    dry_run: bool,
    no_verify: bool,
    html: Option<PathBuf>,
    prefetch: bool,
    key: &str,
    skip_diagnose: bool,
) -> RepairOneOutcome {
    repair_one_outcome_depth(
        url,
        rules,
        l0_only,
        tcp_timeout,
        l2_timeout,
        dry_run,
        no_verify,
        html,
        prefetch,
        key,
        skip_diagnose,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn repair_one_outcome_depth(
    url: &str,
    rules: Option<PathBuf>,
    l0_only: bool,
    tcp_timeout: f64,
    l2_timeout: f64,
    dry_run: bool,
    no_verify: bool,
    html: Option<PathBuf>,
    prefetch: bool,
    key: &str,
    skip_diagnose: bool,
    hunt_depth: u8,
) -> RepairOneOutcome {
    if let Ok(paths) = source_closeout::CloseoutPaths::from_repo() {
        let _ = source_closeout::claim_active_entry(
            &paths,
            url,
            "repair oneshot",
            source_closeout::ClaimEntry::Oneshot,
        );
    }
    let path = rules.unwrap_or_else(default_rules_path);
    let rules = match load_rules(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("repair: load rules: {e}");
            return RepairOneOutcome::err(4, &e.to_string());
        }
    };

    let mut gate = if l0_only {
        classify_one_l0(url, &rules)
    } else {
        #[cfg(feature = "gate_full")]
        {
            classify_one(
                url,
                &rules,
                &ClassifyOpts {
                    tcp_timeout_s: tcp_timeout,
                    l2_timeout_s: l2_timeout,
                },
            )
        }
        #[cfg(not(feature = "gate_full"))]
        {
            let _ = (tcp_timeout, l2_timeout);
            classify_one_l0(url, &rules)
        }
    };

    let mut hunt_migrate_to: Option<String> = None;
    if gate.action == GateAction::Hunt {
        let (g, mig) = resolve_hunt_gate(url, &rules, l2_timeout, !l0_only, gate);
        gate = g;
        hunt_migrate_to = mig;
    }

    let ep = match McpEndpoint::load_defaults() {
        Ok(e) => e,
        Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
    };
    let client = Arc::new(McpClient::new(ep).with_client_name("source_cli_repair"));
    if let Err(e) = client.ensure_session() {
        return RepairOneOutcome::err(2, &e.to_string());
    }

    let channel = match FsChannelPort::from_repo() {
        Ok(c) => c,
        Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
    };
    if let Err(e) = channel.assert_idle_for_repair() {
        return RepairOneOutcome::err(5, &e.to_string());
    }
    let _guard = match channel.acquire_repair() {
        Ok(g) => g,
        Err(e) => return RepairOneOutcome::err(5, &e.to_string()),
    };

    let source_key = SourceKey::new(url.trim());
    let repo = McpSourceRepository::new(Arc::clone(&client));
    let source = match repo.get(&source_key) {
        Ok(s) => s,
        Err(e) => return RepairOneOutcome::err(2, &e.to_string()),
    };

    if let Some(to_url) = hunt_migrate_to.as_ref() {
        if dry_run {
            eprintln!("repair: hunt migrate dry-run {url} → {to_url}");
        } else if !hunt_depth_ok(hunt_depth) {
            return depth_exceeded_outcome(url, to_url);
        } else {
            match apply_hunt_migrate(&repo, url, to_url, &source) {
                Ok(()) => {
                    eprintln!("repair: hunt migrate {url} → {to_url}; re-enter repair");
                    drop(_guard);
                    return repair_one_outcome_depth(
                        to_url,
                        Some(path),
                        l0_only,
                        tcp_timeout,
                        l2_timeout,
                        false,
                        no_verify,
                        None,
                        prefetch,
                        key,
                        skip_diagnose,
                        hunt_depth + 1,
                    );
                }
                Err(e) => {
                    eprintln!("repair: hunt migrate failed: {e}; disable");
                    gate.action = GateAction::Disable;
                    gate.reason = format!("hunt_migrate_failed:{e}");
                    gate.migrate_to = None;
                }
            }
        }
    }

    run_after_gate(
        url,
        client,
        &channel,
        &repo,
        source,
        source_key,
        gate,
        dry_run,
        no_verify,
        html,
        prefetch,
        key,
        skip_diagnose,
    )
}
