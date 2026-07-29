//! Retro append + ledger seal for terminal fail/skip.

use std::fs::OpenOptions;
use std::io::Write;

use chrono::Utc;
use serde_json::{json, Value};

use crate::improve::gate_script_fix;
use crate::paths::{norm_url, CloseoutPaths};
use crate::trap::gate_trap;

#[derive(Debug, Clone)]
pub struct RetroAppendOpts {
    pub url: String,
    pub status: String,
    pub msg: String,
    pub name: String,
    pub respond_time: Option<i64>,
    pub waste_s: f64,
    pub trap: String,
    pub harness: String,
    pub script_fix: String,
    pub skill_fix: bool,
    pub seal: bool,
}

#[derive(Debug, Clone)]
pub struct RetroRow {
    pub row: Value,
    pub sealed: Option<Value>,
}

pub fn append_retro(paths: &CloseoutPaths, opts: RetroAppendOpts) -> Result<RetroRow, Vec<String>> {
    let trap = opts.trap.trim();
    if !trap.is_empty() {
        gate_trap(paths, trap, opts.skill_fix, &[], false)?;
    }
    gate_script_fix(&opts.script_fix, opts.skill_fix)?;

    let row = json!({
        "ts": Utc::now().to_rfc3339(),
        "url": opts.url,
        "name": opts.name,
        "status": opts.status,
        "msg": opts.msg.chars().take(200).collect::<String>(),
        "respondTime": opts.respond_time,
        "waste_s": opts.waste_s,
        "trap": opts.trap,
        "harness": opts.harness,
        "script_fix": opts.script_fix,
        "skill_fix": opts.skill_fix,
    });

    if let Some(parent) = paths.retro.parent() {
        std::fs::create_dir_all(parent).map_err(|e| vec![e.to_string()])?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.retro)
        .map_err(|e| vec![e.to_string()])?;
    writeln!(f, "{}", row).map_err(|e| vec![e.to_string()])?;

    let sealed = if opts.seal {
        seal_ledger(paths, &row)?
    } else {
        None
    };
    // Any terminal retro seals deep_active so progress next is not stuck.
    let status = opts.status.trim().to_lowercase();
    if matches!(status.as_str(), "fixed" | "fail" | "skip") {
        let _ = crate::active::seal_active(paths, &opts.url, &status);
    }
    Ok(RetroRow { row, sealed })
}

fn seal_ledger(paths: &CloseoutPaths, retro: &Value) -> Result<Option<Value>, Vec<String>> {
    let status = retro
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if status != "fail" && status != "skip" {
        return Ok(None);
    }
    let url = norm_url(retro.get("url").and_then(|v| v.as_str()).unwrap_or(""));
    if url.is_empty() {
        return Ok(None);
    }
    let reason = retro
        .get("trap")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| retro.get("msg").and_then(|v| v.as_str()))
        .unwrap_or(&status)
        .replace('\n', " ");
    let reason: String = reason.chars().take(80).collect();
    let ledger_row = json!({
        "ts": Utc::now().to_rfc3339(),
        "url": url,
        "step": if status == "skip" { "skip" } else { "check" },
        "result": format!("{status}:{}", if reason.is_empty() { status.clone() } else { reason.clone() }),
        "note": "sealed by source-cli retro",
        "waste": "",
        "final": true,
    });
    if let Some(parent) = paths.ledger.parent() {
        std::fs::create_dir_all(parent).map_err(|e| vec![e.to_string()])?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.ledger)
        .map_err(|e| vec![format!("seal ledger open: {e}")])?;
    writeln!(f, "{ledger_row}").map_err(|e| vec![format!("seal ledger write: {e}")])?;
    Ok(Some(ledger_row))
}
