//! Pending close-out gate before progress next.

use serde_json::{json, Value};

use crate::active::{gate_active_unsealed, read_active};
use crate::improve::gate_script_fix;
use crate::jsonl::read_jsonl;
use crate::ledger_gate::ledger_result_blocked;
use crate::paths::{norm_url, CloseoutPaths};
use crate::skill::{skill_in_sync, sync_skill_to_cursor};
use crate::trap::gate_trap;

const TERMINAL_STEPS: &[&str] = &["check", "skip"];

#[derive(Debug, Clone)]
pub struct PendingDetail {
    pub ok: bool,
    pub reason: Option<String>,
    pub url: Option<String>,
    pub extra: Value,
}

pub fn last_terminal_ledger(paths: &CloseoutPaths) -> Option<Value> {
    read_jsonl(&paths.ledger).into_iter().rev().find(|row| {
        let url = norm_url(row.get("url").and_then(|v| v.as_str()).unwrap_or(""));
        let step = row.get("step").and_then(|v| v.as_str()).unwrap_or("");
        !url.is_empty() && TERMINAL_STEPS.contains(&step)
    })
}

pub fn latest_retro_for_url(paths: &CloseoutPaths, url: &str) -> Option<Value> {
    let target = norm_url(url);
    read_jsonl(&paths.retro)
        .into_iter()
        .rev()
        .find(|row| norm_url(row.get("url").and_then(|v| v.as_str()).unwrap_or("")) == target)
}

pub fn pending_closeout(paths: &CloseoutPaths) -> (bool, Vec<String>, PendingDetail) {
    let mut errors = Vec::new();

    // Hard gate: unsealed deep_active blocks next pick (agent_turn_stall).
    if let Err(active_errs) = gate_active_unsealed(paths) {
        errors.extend(active_errs);
        let active = read_active(paths).unwrap_or(json!({}));
        let url = norm_url(active.get("url").and_then(|v| v.as_str()).unwrap_or(""));
        return (
            false,
            errors,
            PendingDetail {
                ok: false,
                reason: Some("deep_active_unsealed".into()),
                url: if url.is_empty() { None } else { Some(url) },
                extra: json!({
                    "ok": false,
                    "missing": "deep_active_seal",
                    "active": active,
                }),
            },
        );
    }

    let Some(last) = last_terminal_ledger(paths) else {
        return (
            true,
            errors,
            PendingDetail {
                ok: true,
                reason: Some("no_terminal_ledger".into()),
                url: None,
                extra: json!({
                    "active": read_active(paths),
                }),
            },
        );
    };

    let url = norm_url(last.get("url").and_then(|v| v.as_str()).unwrap_or(""));
    let ledger_result = last
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut extra = json!({
        "url": url,
        "ledger_step": last.get("step"),
        "ledger_result": ledger_result.clone(),
        "active": read_active(paths),
    });

    if let Some(msg) = ledger_result_blocked(&ledger_result) {
        errors.push(format!(
            "last terminal ledger result blocked for {url:?}: {msg}"
        ));
        extra["ok"] = json!(false);
        extra["missing"] = json!("honest_ledger_result");
        return (
            false,
            errors,
            PendingDetail {
                ok: false,
                reason: Some("hedged_ledger".into()),
                url: Some(url),
                extra,
            },
        );
    }

    let Some(retro) = latest_retro_for_url(paths, &url) else {
        errors.push(format!(
            "close-out incomplete for {url:?}: ledger {:?} but no \
             retro row — run source-cli retro append --url … --status …",
            last.get("step")
        ));
        extra["ok"] = json!(false);
        extra["missing"] = json!("retro");
        return (
            false,
            errors,
            PendingDetail {
                ok: false,
                reason: None,
                url: Some(url),
                extra,
            },
        );
    };

    extra["retro_status"] = retro.get("status").cloned().unwrap_or(Value::Null);
    let trap = retro
        .get("trap")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    extra["trap"] = json!(trap);
    let skill_fix = retro
        .get("skill_fix")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    extra["skill_fix"] = json!(skill_fix);

    if !trap.is_empty() {
        match gate_trap(paths, &trap, skill_fix, &[], false) {
            Ok(()) => extra["gate"] = json!("pass"),
            Err(gate_errs) => {
                errors.extend(gate_errs);
                extra["ok"] = json!(false);
                extra["gate"] = json!("fail");
                return (
                    false,
                    errors,
                    PendingDetail {
                        ok: false,
                        reason: None,
                        url: Some(url),
                        extra,
                    },
                );
            }
        }
    }

    let script_fix = retro
        .get("script_fix")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    extra["script_fix"] = json!(script_fix);
    if let Err(sf_errs) = gate_script_fix(&script_fix, skill_fix) {
        errors.extend(sf_errs);
        extra["ok"] = json!(false);
        extra["improve_gate"] = json!("fail");
        return (
            false,
            errors,
            PendingDetail {
                ok: false,
                reason: None,
                url: Some(url),
                extra,
            },
        );
    }
    if skill_fix {
        extra["improve_gate"] = json!("pass");
    }

    if skill_fix && !skill_in_sync(paths) {
        match sync_skill_to_cursor(paths) {
            Ok(msg) => extra["skill_sync"] = json!(msg),
            Err(msg) => {
                extra["skill_sync"] = json!(format!("fail: {msg}"));
                errors.push(format!("skill_fix=1 but sync failed: {msg}"));
                extra["ok"] = json!(false);
                return (
                    false,
                    errors,
                    PendingDetail {
                        ok: false,
                        reason: None,
                        url: Some(url),
                        extra,
                    },
                );
            }
        }
    }

    extra["ok"] = json!(true);
    (
        true,
        errors,
        PendingDetail {
            ok: true,
            reason: None,
            url: Some(url),
            extra,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_jsonl(path: &std::path::Path, lines: &[&str]) {
        let mut f = std::fs::File::create(path).unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
    }

    #[test]
    fn missing_retro_blocks() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let skill = root.join("skills/legado-book-source-repair/SKILL.md");
        std::fs::create_dir_all(skill.parent().unwrap()).unwrap();
        std::fs::write(&skill, "## Traps\n| foo | bar |\n").unwrap();
        let ledger = root.join("ledger.jsonl");
        write_jsonl(
            &ledger,
            &[r#"{"url":"https://a.test","step":"check","result":"校验成功"}"#],
        );
        let paths = CloseoutPaths {
            root: root.to_path_buf(),
            skill_sot: skill,
            cursor_skill: root.join("cursor/SKILL.md"),
            ledger,
            retro: root.join("retro.jsonl"),
        };
        let (ok, _, _) = pending_closeout(&paths);
        assert!(!ok);
    }
}
