//! Deep-diagnose active URL claim — blocks progress next until sealed.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::paths::{norm_url, CloseoutPaths};

const STALE_SEALED_S: u64 = 6 * 3600;
const WARN_UNSEALED_S: u64 = 300; // 5 min hard budget signal
const DIAGNOSE_EVIDENCE_MAX_AGE_S: u64 = 6 * 3600;

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// How this deep dig was entered (recorded on `deep_active.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimEntry {
    Diagnose,
    Oneshot,
    McpFallback,
    Create,
}

impl ClaimEntry {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Diagnose => "diagnose",
            Self::Oneshot => "oneshot",
            Self::McpFallback => "mcp_fallback",
            Self::Create => "create",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "diagnose" => Some(Self::Diagnose),
            "oneshot" => Some(Self::Oneshot),
            "mcp_fallback" | "mcp" | "manual_mcp" => Some(Self::McpFallback),
            "create" | "create_push" | "push" => Some(Self::Create),
            _ => None,
        }
    }

    /// Infer from legacy claim notes when `--entry` was not passed.
    pub fn from_note(note: &str) -> Self {
        let n = note.trim().to_ascii_lowercase();
        if n == "diagnose" || n.starts_with("diagnose ") {
            Self::Diagnose
        } else if n.contains("oneshot") {
            Self::Oneshot
        } else if n.contains("create push") || n == "create" {
            Self::Create
        } else {
            Self::McpFallback
        }
    }
}

pub fn active_path(paths: &CloseoutPaths) -> PathBuf {
    paths.root.join("temp/full_fix/deep_active.json")
}

pub fn diagnose_dir(paths: &CloseoutPaths) -> PathBuf {
    paths.root.join("temp/full_fix/cache/diagnose")
}

/// Stable filename for a diagnose JSON artifact.
pub fn diagnose_artifact_path(paths: &CloseoutPaths, url: &str) -> PathBuf {
    let u = norm_url(url);
    let safe: String = u
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe: String = if safe.is_empty() {
        "unknown".into()
    } else {
        safe.chars().take(180).collect()
    };
    diagnose_dir(paths).join(format!("{safe}.json"))
}

pub fn read_active(paths: &CloseoutPaths) -> Option<Value> {
    let p = active_path(paths);
    let raw = fs::read_to_string(p).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_active(paths: &CloseoutPaths, v: &Value) -> Result<(), String> {
    let p = active_path(paths);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(p, serde_json::to_string_pretty(v).unwrap_or_default()).map_err(|e| e.to_string())
}

/// Claim (or refresh) the current deep-diagnose URL.
pub fn claim_active(paths: &CloseoutPaths, url: &str, note: &str) -> Result<Value, String> {
    claim_active_entry(paths, url, note, ClaimEntry::from_note(note))
}

pub fn claim_active_entry(
    paths: &CloseoutPaths,
    url: &str,
    note: &str,
    entry: ClaimEntry,
) -> Result<Value, String> {
    let url = norm_url(url);
    if url.is_empty() {
        return Err("claim: empty url".into());
    }
    let ts = now_epoch();
    let mut diagnose_at = Value::Null;
    // Keep prior diagnose_at when re-claiming same URL (e.g. MCP after diagnose).
    // Do NOT invent diagnose_at from entry alone — only mark_diagnose_done stamps it.
    if let Some(prev) = read_active(paths) {
        let prev_url = norm_url(prev.get("url").and_then(|x| x.as_str()).unwrap_or(""));
        if prev_url == url {
            if let Some(da) = prev.get("diagnose_at") {
                if !da.is_null() {
                    diagnose_at = da.clone();
                }
            }
        }
    }
    let v = json!({
        "schema_version": "1",
        "url": url,
        "claimed_at": ts,
        "heartbeat_at": ts,
        "pid": std::process::id(),
        "sealed": false,
        "status": Value::Null,
        "note": note.chars().take(120).collect::<String>(),
        "entry": entry.as_str(),
        "diagnose_at": diagnose_at,
    });
    write_active(paths, &v)?;
    Ok(v)
}

/// Record that `source-cli diagnose` produced evidence for this URL.
pub fn mark_diagnose_done(
    paths: &CloseoutPaths,
    url: &str,
    payload: &str,
) -> Result<PathBuf, String> {
    let url = norm_url(url);
    let path = diagnose_artifact_path(paths, &url);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, payload).map_err(|e| e.to_string())?;
    let ts = now_epoch();
    if let Some(mut v) = read_active(paths) {
        let cur = norm_url(v.get("url").and_then(|x| x.as_str()).unwrap_or(""));
        if cur.is_empty() || cur == url {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("url".into(), json!(url));
                obj.insert("diagnose_at".into(), json!(ts));
                obj.insert("heartbeat_at".into(), json!(ts));
                if obj
                    .get("entry")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .is_empty()
                {
                    obj.insert("entry".into(), json!(ClaimEntry::Diagnose.as_str()));
                }
            }
            write_active(paths, &v)?;
        }
    }
    Ok(path)
}

fn artifact_fresh(path: &PathBuf) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(dur) = modified.duration_since(UNIX_EPOCH) else {
        return false;
    };
    now_epoch().saturating_sub(dur.as_secs()) <= DIAGNOSE_EVIDENCE_MAX_AGE_S
}

/// True when fixed close-out may proceed without mcp_fallback escape.
///
/// Requires a real diagnose artifact / diagnose_at stamp from `mark_diagnose_done`.
/// Bare `claim --entry diagnose|oneshot` alone is NOT enough.
pub fn has_diagnose_evidence(paths: &CloseoutPaths, url: &str) -> bool {
    let url = norm_url(url);
    if let Some(v) = read_active(paths) {
        let cur = norm_url(v.get("url").and_then(|x| x.as_str()).unwrap_or(""));
        if cur == url || cur.is_empty() {
            if let Some(da) = v.get("diagnose_at").and_then(|x| x.as_u64()) {
                if da > 0 && now_epoch().saturating_sub(da) <= DIAGNOSE_EVIDENCE_MAX_AGE_S {
                    return true;
                }
            }
        }
    }
    let art = diagnose_artifact_path(paths, &url);
    art.is_file() && artifact_fresh(&art)
}

/// Gate `retro append --status fixed`: require diagnose path or documented MCP fallback.
pub fn gate_fixed_diagnose_path(
    paths: &CloseoutPaths,
    url: &str,
    trap: &str,
    script_fix: &str,
) -> Result<(), Vec<String>> {
    let url = norm_url(url);
    let entry = read_active(paths)
        .and_then(|v| {
            let cur = norm_url(v.get("url").and_then(|x| x.as_str()).unwrap_or(""));
            if !cur.is_empty() && cur != url {
                return None;
            }
            v.get("entry")
                .and_then(|x| x.as_str())
                .and_then(ClaimEntry::parse)
        })
        .unwrap_or(ClaimEntry::McpFallback);

    if matches!(entry, ClaimEntry::Create) {
        return Ok(());
    }
    if has_diagnose_evidence(paths, &url) {
        return Ok(());
    }
    if entry == ClaimEntry::McpFallback {
        let trap_ok = trap.to_ascii_lowercase().contains("manual_mcp_bypass");
        let sf = script_fix.trim().to_ascii_lowercase();
        let script_ok =
            sf.starts_with("no_auto:diagnose_transport") || sf.starts_with("no_auto:user_");
        if trap_ok && script_ok {
            return Ok(());
        }
        return Err(vec![format!(
            "fixed via mcp_fallback requires trap containing 'manual_mcp_bypass' AND \
             --script-fix starting with no_auto:diagnose_transport… or no_auto:user_… \
             (got trap={trap:?} script_fix={script_fix:?}). Prefer: \
             source-cli diagnose → oneshot, or source-cli dig --url …"
        )]);
    }
    Err(vec![format!(
        "fixed requires diagnose evidence for {url:?} (entry={}, no artifact/diagnose_at). \
         Run `source-cli diagnose` / `source-cli dig` first (writes cache/diagnose/*.json), \
         or claim --entry mcp_fallback with manual_mcp_bypass + no_auto:diagnose_transport…",
        entry.as_str()
    )])
}

pub fn heartbeat_active(paths: &CloseoutPaths) -> Result<Value, String> {
    let mut v = read_active(paths).ok_or_else(|| "heartbeat: no deep_active.json".to_string())?;
    if v.get("sealed").and_then(|x| x.as_bool()).unwrap_or(false) {
        return Ok(v);
    }
    if let Some(obj) = v.as_object_mut() {
        obj.insert("heartbeat_at".into(), json!(now_epoch()));
        obj.insert("pid".into(), json!(std::process::id()));
    }
    write_active(paths, &v)?;
    Ok(v)
}

/// Mark current claim sealed (fixed/skip/fail) so progress next may continue.
pub fn seal_active(paths: &CloseoutPaths, url: &str, status: &str) -> Result<Value, String> {
    let url = norm_url(url);
    let mut v = match read_active(paths) {
        Some(v) => v,
        None => json!({
            "schema_version": "1",
            "url": url,
            "claimed_at": now_epoch(),
        }),
    };
    let cur = norm_url(v.get("url").and_then(|x| x.as_str()).unwrap_or(""));
    if let Some(obj) = v.as_object_mut() {
        obj.insert("url".into(), json!(if url.is_empty() { cur } else { url }));
        obj.insert("sealed".into(), json!(true));
        obj.insert("status".into(), json!(status));
        obj.insert("heartbeat_at".into(), json!(now_epoch()));
        obj.insert("sealed_at".into(), json!(now_epoch()));
    }
    write_active(paths, &v)?;
    Ok(v)
}

pub fn clear_active(paths: &CloseoutPaths) -> Result<(), String> {
    let p = active_path(paths);
    if p.is_file() {
        fs::remove_file(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Block progress next when an unsealed deep claim exists.
pub fn gate_active_unsealed(paths: &CloseoutPaths) -> Result<(), Vec<String>> {
    let Some(v) = read_active(paths) else {
        return Ok(());
    };
    let sealed = v.get("sealed").and_then(|x| x.as_bool()).unwrap_or(false);
    if sealed {
        let sealed_at = v
            .get("sealed_at")
            .or_else(|| v.get("heartbeat_at"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        if now_epoch().saturating_sub(sealed_at) > STALE_SEALED_S {
            let _ = clear_active(paths);
        }
        return Ok(());
    }
    let url = norm_url(v.get("url").and_then(|x| x.as_str()).unwrap_or(""));
    let claimed = v.get("claimed_at").and_then(|x| x.as_u64()).unwrap_or(0);
    let age = now_epoch().saturating_sub(claimed);
    let entry = v.get("entry").and_then(|x| x.as_str()).unwrap_or("?");
    let mut errs = vec![format!(
        "deep_active unsealed for {url:?} entry={entry} (age {age}s) — finish close-out: \
         ledger + retro append, or `source-cli closeout release --url … --status skip|fail|fixed`"
    )];
    if age >= WARN_UNSEALED_S {
        errs.push(format!(
            "hard budget exceeded ({WARN_UNSEALED_S}s): seal/skip this URL before next pick \
             (trap agent_turn_stall / discipline §21)"
        ));
    }
    Err(errs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn paths(tmp: &TempDir) -> CloseoutPaths {
        CloseoutPaths::under(tmp.path())
    }

    #[test]
    fn claim_blocks_until_seal() {
        let tmp = TempDir::new().unwrap();
        let p = paths(&tmp);
        claim_active_entry(&p, "https://a.test/", "diag", ClaimEntry::Diagnose).unwrap();
        assert!(gate_active_unsealed(&p).is_err());
        seal_active(&p, "https://a.test/", "fixed").unwrap();
        assert!(gate_active_unsealed(&p).is_ok());
    }

    #[test]
    fn fixed_requires_diagnose_or_documented_mcp_fallback() {
        let tmp = TempDir::new().unwrap();
        let p = paths(&tmp);
        claim_active_entry(
            &p,
            "https://b.test/",
            "legado_mcp debug",
            ClaimEntry::McpFallback,
        )
        .unwrap();
        assert!(gate_fixed_diagnose_path(&p, "https://b.test/", "x", "y").is_err());
        assert!(gate_fixed_diagnose_path(
            &p,
            "https://b.test/",
            "manual_mcp_bypass",
            "no_auto:diagnose_transport_fail_10060",
        )
        .is_ok());

        // Bare diagnose claim without artifact must NOT unlock fixed.
        claim_active_entry(&p, "https://c.test/", "diagnose", ClaimEntry::Diagnose).unwrap();
        assert!(gate_fixed_diagnose_path(&p, "https://c.test/", "", "").is_err());
        mark_diagnose_done(&p, "https://c.test/", "{\"layer\":\"search\"}").unwrap();
        assert!(gate_fixed_diagnose_path(&p, "https://c.test/", "", "").is_ok());
    }

    #[test]
    fn diagnose_artifact_counts_as_evidence() {
        let tmp = TempDir::new().unwrap();
        let p = paths(&tmp);
        claim_active_entry(&p, "https://d.test/", "legado_mcp", ClaimEntry::McpFallback).unwrap();
        mark_diagnose_done(&p, "https://d.test/", "{\"layer\":\"search\"}").unwrap();
        assert!(has_diagnose_evidence(&p, "https://d.test/"));
        assert!(gate_fixed_diagnose_path(&p, "https://d.test/", "", "").is_ok());
    }
}
