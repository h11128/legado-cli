//! Offline + MCP diagnose CLI.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use source_contracts::validate_diagnose;
use source_diagnose::{diagnose_from_debug, diagnose_gate_skip, gate_blocks_diagnose};
use source_gate::{classify_one_l0, load_rules};
use source_mcp::{McpClient, McpEndpoint, McpSourceRepository};
use source_ports::SourceRepository;
use source_probe::probe_search_live;
use source_types::{Layer, SourceKey, Url};

use super::diagnose_http_log::enrich_http_log_on_empty_search;
use super::diagnose_tips::{enrich_with_live_probe, layer_tips};

#[cfg(feature = "gate_full")]
use source_gate::{classify_one, ClassifyOpts};

pub struct DiagnoseArgs {
    pub url: String,
    pub key: String,
    pub rules: Option<PathBuf>,
    pub l0_only: bool,
    pub tcp_timeout: f64,
    pub l2_timeout: f64,
    /// Skip MCP debug_source; diagnose from this debug log file.
    pub debug_file: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

fn write_out(args: &DiagnoseArgs, payload: &str) {
    if let Some(path) = &args.out {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, payload);
    }
}

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

fn fetch_text(url: &str) -> Option<(u16, String)> {
    let resp = ureq::get(url).call().ok()?;
    let status = resp.status();
    let mut buf = Vec::new();
    resp.into_reader().read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    Some((status, text))
}

fn attach_probe_tips(diag: &mut source_types::DiagnoseResult, key: &str) {
    diag.tips = layer_tips(diag);
    if diag.layer != Layer::Search && diag.fake_detail != Some(true) {
        return;
    }
    let base = diag.url.as_str().trim_end_matches('/').to_string() + "/";
    let Some((_, home)) = fetch_text(base.trim_end_matches('/')) else {
        diag.tips
            .push("probe: home fetch failed — tips are layer-only".into());
        return;
    };
    let live = probe_search_live(&home, &base, key, &fetch_text, 8);
    enrich_with_live_probe(diag, &live);
}

pub fn run_diagnose(args: DiagnoseArgs) -> ExitCode {
    let path = args.rules.clone().unwrap_or_else(default_rules_path);
    let rules = match load_rules(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("diagnose: load rules: {e}");
            return ExitCode::from(4);
        }
    };
    let gate = if args.l0_only {
        classify_one_l0(&args.url, &rules)
    } else {
        #[cfg(feature = "gate_full")]
        {
            let opts = ClassifyOpts {
                tcp_timeout_s: args.tcp_timeout,
                l2_timeout_s: args.l2_timeout,
            };
            classify_one(&args.url, &rules, &opts)
        }
        #[cfg(not(feature = "gate_full"))]
        {
            let _ = (args.tcp_timeout, args.l2_timeout);
            classify_one_l0(&args.url, &rules)
        }
    };

    let url = match Url::new(args.url.trim()) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("diagnose: bad url: {e}");
            return ExitCode::from(4);
        }
    };

    // Claim deep_active so progress next / turn end cannot skip close-out.
    if let Ok(paths) = source_closeout::CloseoutPaths::from_repo() {
        match source_closeout::claim_active(&paths, url.as_str(), "diagnose") {
            Ok(_) => {}
            Err(e) => eprintln!("diagnose: warn: deep_active claim: {e}"),
        }
    }

    if gate_blocks_diagnose(&gate) {
        let mut d = diagnose_gate_skip(url, gate);
        d.tips = layer_tips(&d);
        let v = serde_json::to_value(&d).unwrap_or_default();
        let _ = validate_diagnose(&v);
        let payload = serde_json::to_string_pretty(&d).unwrap_or_default();
        write_out(&args, &payload);
        println!("{payload}");
        return ExitCode::from(3);
    }

    let (debug_text, mcp_client) = if let Some(p) = &args.debug_file {
        match std::fs::read_to_string(p) {
            Ok(t) => (t, None),
            Err(e) => {
                eprintln!("diagnose: read debug file: {e}");
                return ExitCode::from(4);
            }
        }
    } else {
        let ep = match McpEndpoint::load_defaults() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("diagnose: mcp: {e}");
                return ExitCode::from(4);
            }
        };
        let client = Arc::new(McpClient::new(ep).with_client_name("source_cli_diagnose"));
        if let Err(e) = client.ensure_session() {
            eprintln!("diagnose: session: {e}");
            return ExitCode::from(2);
        }
        let repo = McpSourceRepository::new(Arc::clone(&client));
        let _ = repo.get(&SourceKey::new(args.url.trim()));
        let raw = match client.tools_call(
            "debug_source",
            serde_json::json!({"url": args.url, "key": args.key}),
        ) {
            Ok(v) => McpClient::extract_text(&v),
            Err(e) => {
                eprintln!("diagnose: debug_source: {e}");
                return ExitCode::from(2);
            }
        };
        (raw, Some(client))
    };

    let mut d = diagnose_from_debug(url, &debug_text, Some(gate), None);
    attach_probe_tips(&mut d, &args.key);
    if let Some(ref client) = mcp_client {
        enrich_http_log_on_empty_search(client, &mut d, args.url.trim(), &args.key, &debug_text);
    }
    let v = serde_json::to_value(&d).unwrap_or_default();
    if let Err(e) = validate_diagnose(&v) {
        eprintln!("diagnose: contract: {e}");
        return ExitCode::from(4);
    }
    let payload = serde_json::to_string_pretty(&d).unwrap_or_default();
    write_out(&args, &payload);
    println!("{payload}");
    ExitCode::SUCCESS
}
