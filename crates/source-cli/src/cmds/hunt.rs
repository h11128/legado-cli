//! Domain hunt CLI from seeds JSON + optional L2 probe.

use std::path::PathBuf;
use std::process::ExitCode;

use source_gate::{classify_one, load_rules, ClassifyOpts};
use source_hunt::{HuntResolve, HuntSeeds};
use source_types::GateAction;

pub struct HuntArgs {
    pub url: String,
    pub seeds: Option<PathBuf>,
    pub probe: bool,
    pub l2_timeout: f64,
    pub rules: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

pub fn default_seeds() -> PathBuf {
    let candidates = [
        PathBuf::from("config/domain_hunt_seeds.json"),
        PathBuf::from("../config/domain_hunt_seeds.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/domain_hunt_seeds.json"),
    ];
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/domain_hunt_seeds.json")
}

fn default_rules() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json")
}

/// Probe seed candidates; first that passes L2 Verify **or** Migrate (live redirect).
pub fn probe_best_candidate(
    candidates: &[String],
    rules: &[source_gate::SkipRule],
    l2_timeout: f64,
) -> (Option<String>, Vec<serde_json::Value>) {
    let mut probes = Vec::new();
    let mut best: Option<String> = None;
    for c in candidates {
        let row = classify_one(
            c,
            rules,
            &ClassifyOpts {
                tcp_timeout_s: 1.5,
                l2_timeout_s: l2_timeout,
            },
        );
        let alive = matches!(row.action, GateAction::Verify | GateAction::Migrate)
            || row.l2.as_ref().is_some_and(|l| l.ok);
        let row_json = serde_json::to_value(&row).unwrap_or(serde_json::json!({}));
        if alive && best.is_none() {
            // Prefer migrate_to URL when candidate itself redirects.
            if row.action == GateAction::Migrate {
                if let Some(ref m) = row.migrate_to {
                    let to = match m {
                        source_types::MigrateTarget::Url(u) => u.as_str().to_string(),
                        source_types::MigrateTarget::Host(h) => format!("http://{}/", h.as_str()),
                    };
                    best = Some(to);
                } else {
                    best = Some(c.clone());
                }
            } else {
                best = Some(c.clone());
            }
        }
        probes.push(row_json);
    }
    (best, probes)
}

/// Full hunt resolve used by CLI + oneshot (always probes when rules available).
pub fn resolve_hunt(
    url: &str,
    seeds: &HuntSeeds,
    rules: &[source_gate::SkipRule],
    l2_timeout: f64,
    probe: bool,
) -> (HuntResolve, Vec<serde_json::Value>) {
    let base = HuntResolve::from_seeds(seeds, url);
    if !probe || base.shutdown || base.candidates.is_empty() {
        return (base, Vec::new());
    }
    let (best, probes) = probe_best_candidate(&base.candidates, rules, l2_timeout);
    (base.with_best_alive(best), probes)
}

fn hunt_one(
    url: &str,
    seeds: &HuntSeeds,
    rules: &[source_gate::SkipRule],
    l2_timeout: f64,
    probe: bool,
) -> serde_json::Value {
    let (resolved, probes) = resolve_hunt(url, seeds, rules, l2_timeout, probe);
    serde_json::json!({
        "schema_version": "1",
        "url": resolved.url,
        "host": resolved.host,
        "note": resolved.note,
        "shutdown": resolved.shutdown,
        "confidence": resolved.confidence,
        "candidates": resolved.candidates,
        "best_candidate": resolved.best_candidate,
        "probes": probes,
        "action": resolved.action.as_str(),
        "status": if resolved.candidates.is_empty() { "empty" } else { "hunted" },
    })
}

pub fn run_hunt(args: HuntArgs) -> ExitCode {
    let path = args.seeds.unwrap_or_else(default_seeds);
    let seeds = match HuntSeeds::load_path(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hunt: load seeds: {e}");
            return ExitCode::from(4);
        }
    };
    let rules_path = args.rules.unwrap_or_else(default_rules);
    let rules = match load_rules(&rules_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("hunt: rules: {e}");
            return ExitCode::from(4);
        }
    };
    let out = hunt_one(&args.url, &seeds, &rules, args.l2_timeout, args.probe);
    if let Some(path) = args.out {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(&out).unwrap_or_default(),
        );
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    ExitCode::SUCCESS
}
