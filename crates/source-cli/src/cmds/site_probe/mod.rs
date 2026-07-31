//! Batch site reachability for book-source discovery (`site-probe`).

mod load;

use load::{default_rules_path, load_preset, load_urls_file, preset_path, Cand};
use serde_json::{json, Value};
use source_gate::{classify_one, load_rules, ClassifyOpts};
use source_types::GateAction;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

pub struct SiteProbeArgs {
    pub preset: Option<String>,
    pub urls_file: Option<PathBuf>,
    pub url: Vec<String>,
    pub rules: Option<PathBuf>,
    pub concurrency: usize,
    pub tcp_timeout: f64,
    pub l2_timeout: f64,
    pub out: Option<PathBuf>,
    /// When true, also L1/L2-probe rows marked `status=sourced`.
    pub include_sourced: bool,
}

fn static_discovery(status: &str) -> Option<&'static str> {
    match status {
        "sourced" => Some("already_sourced"),
        "skip_waf" => Some("skip_waf"),
        "skip_vpn" => Some("skip_vpn"),
        "skip_app" => Some("skip_app"),
        "skip_pdf" => Some("skip_pdf"),
        "skip_catalog" => Some("skip_catalog"),
        "skip_dead" => Some("skip_dead"),
        _ => None,
    }
}

fn discovery_from_gate(action: GateAction, reason: &str) -> &'static str {
    let r = reason.to_ascii_lowercase();
    if r.contains("cf")
        || r.contains("challenge")
        || r.contains("just a moment")
        || r.contains("wall")
        || r.contains("password_or_db")
        || r.contains("bot_shell")
        || r.contains("waf")
        || r.contains("anti_bot")
        || r.contains("captcha")
    {
        return "skip_waf";
    }
    match action {
        GateAction::Verify | GateAction::Migrate => "make_candidate",
        GateAction::Video => "skip_dead",
        GateAction::Hunt | GateAction::Disable | GateAction::Skip => "skip_dead",
    }
}

fn static_row(c: &Cand, discovery: &str) -> Value {
    json!({
        "id": c.id, "url": c.url, "kind": c.kind, "note": c.note,
        "status": c.status, "discovery": discovery, "verify": false,
        "action": "skip", "reason": format!("preset_status:{}", c.status),
        "latency_ms": 0, "probed": false,
    })
}

fn collect_inputs(args: &SiteProbeArgs) -> Result<Vec<Cand>, String> {
    let mut cands: Vec<Cand> = Vec::new();
    if let Some(p) = args.preset.as_ref() {
        let path = if p.ends_with(".json") {
            PathBuf::from(p)
        } else {
            preset_path(p)
        };
        cands.extend(load_preset(&path)?);
    }
    if let Some(f) = args.urls_file.as_ref() {
        cands.extend(load_urls_file(f)?);
    }
    for u in &args.url {
        let u = u.trim();
        if !u.is_empty() {
            cands.push(Cand {
                id: u.to_string(),
                url: u.to_string(),
                kind: String::new(),
                note: String::new(),
                status: "candidate".into(),
            });
        }
    }
    if cands.is_empty() {
        return Err("need --preset publish and/or --urls-file / --url".into());
    }
    Ok(cands)
}

fn probe_batch(
    to_probe: Vec<Cand>,
    rules_path: PathBuf,
    opts: ClassifyOpts,
    concurrency: usize,
) -> Result<Vec<Value>, String> {
    if to_probe.is_empty() {
        return Ok(Vec::new());
    }
    let rules =
        load_rules(&rules_path).map_err(|e| format!("load rules {}: {e}", rules_path.display()))?;
    let conc = concurrency.max(1).min(32).min(to_probe.len());
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    let chunk = (to_probe.len() + conc - 1) / conc;
    for part in to_probe.chunks(chunk.max(1)) {
        let part = part.to_vec();
        let rules = rules.clone();
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            for c in part {
                let started = Instant::now();
                let gate = classify_one(&c.url, &rules, &opts);
                let ms = started.elapsed().as_millis() as u64;
                let discovery = discovery_from_gate(gate.action, &gate.reason);
                let migrate_to = gate.migrate_to.as_ref().map(|m| match m {
                    source_types::MigrateTarget::Url(u) => u.as_str().to_string(),
                    source_types::MigrateTarget::Host(h) => h.as_str().to_string(),
                });
                let _ = tx.send(json!({
                    "id": c.id, "url": c.url, "kind": c.kind, "note": c.note,
                    "status": c.status, "discovery": discovery,
                    "verify": gate.verify, "action": gate.action.as_str(),
                    "reason": gate.reason, "latency_ms": ms, "probed": true,
                    "migrate_to": migrate_to,
                    "l1_ok": gate.l1.as_ref().map(|x| x.ok),
                    "l2_ok": gate.l2.as_ref().map(|x| x.ok),
                    "l2_status": gate.l2.as_ref().and_then(|x| x.status),
                    "title": gate.l2.as_ref().and_then(|x| x.title.clone()),
                }));
            }
        }));
    }
    drop(tx);
    for h in handles {
        if let Err(e) = h.join() {
            return Err(format!("worker panic: {e:?}"));
        }
    }
    Ok(rx.into_iter().collect())
}

pub fn run_site_probe(args: SiteProbeArgs) -> ExitCode {
    let cands = match collect_inputs(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("site-probe: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut static_rows: Vec<Value> = Vec::new();
    let mut to_probe: Vec<Cand> = Vec::new();
    for c in cands {
        if let Some(d) = static_discovery(&c.status) {
            if c.status == "sourced" && args.include_sourced {
                to_probe.push(c);
            } else {
                static_rows.push(static_row(&c, d));
            }
        } else {
            to_probe.push(c);
        }
    }

    let t0 = Instant::now();
    let probed_rows = match probe_batch(
        to_probe,
        args.rules.clone().unwrap_or_else(default_rules_path),
        ClassifyOpts {
            tcp_timeout_s: args.tcp_timeout,
            l2_timeout_s: args.l2_timeout,
        },
        args.concurrency,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("site-probe: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut rows = static_rows;
    rows.extend(probed_rows);
    rows.sort_by(|a, b| {
        let rank = |r: &Value| -> u8 {
            match r.get("discovery").and_then(|x| x.as_str()).unwrap_or("") {
                "make_candidate" => 0,
                "already_sourced" => 1,
                _ => 2,
            }
        };
        match rank(a).cmp(&rank(b)) {
            Ordering::Equal => a
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .cmp(b.get("id").and_then(|x| x.as_str()).unwrap_or("")),
            o => o,
        }
    });

    let make_n = rows
        .iter()
        .filter(|r| r.get("discovery").and_then(|x| x.as_str()) == Some("make_candidate"))
        .count();
    let probed_n = rows
        .iter()
        .filter(|r| r.get("probed").and_then(|x| x.as_bool()) == Some(true))
        .count();
    let report = json!({
        "schema_version": 1,
        "elapsed_ms": t0.elapsed().as_millis() as u64,
        "concurrency": args.concurrency.max(1).min(32),
        "total": rows.len(),
        "probed": probed_n,
        "make_candidate": make_n,
        "items": rows,
    });

    let text = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
    if let Some(out) = args.out {
        if let Some(parent) = out.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&out, &text) {
            eprintln!("site-probe: write {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        eprintln!(
            "site-probe: wrote {} (probed={probed_n} make_candidate={make_n})",
            out.display()
        );
    }
    println!("{text}");
    ExitCode::SUCCESS
}
