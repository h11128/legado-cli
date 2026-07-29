//! Deep-diagnose active URL claim — blocks progress next until sealed.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::paths::{norm_url, CloseoutPaths};

const STALE_SEALED_S: u64 = 6 * 3600;
const WARN_UNSEALED_S: u64 = 300; // 5 min hard budget signal

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn active_path(paths: &CloseoutPaths) -> PathBuf {
    paths.root.join("temp/full_fix/deep_active.json")
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
    let url = norm_url(url);
    if url.is_empty() {
        return Err("claim: empty url".into());
    }
    let ts = now_epoch();
    let v = json!({
        "schema_version": "1",
        "url": url,
        "claimed_at": ts,
        "heartbeat_at": ts,
        "pid": std::process::id(),
        "sealed": false,
        "status": Value::Null,
        "note": note.chars().take(120).collect::<String>(),
    });
    write_active(paths, &v)?;
    Ok(v)
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
    if !cur.is_empty() && !url.is_empty() && cur != url {
        // Sealing a different URL still clears the stall — write sealed for requested.
    }
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
        // Auto-clear old sealed markers so file does not linger forever.
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
    let mut errs = vec![format!(
        "deep_active unsealed for {url:?} (age {age}s) — finish close-out: \
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
        claim_active(&p, "https://a.test/", "diag").unwrap();
        assert!(gate_active_unsealed(&p).is_err());
        seal_active(&p, "https://a.test/", "fixed").unwrap();
        assert!(gate_active_unsealed(&p).is_ok());
    }
}
