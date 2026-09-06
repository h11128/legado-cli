//! Filesystem MCP channel lock — mirrors `scripts/mcp_channel.py`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use source_ports::{ChannelGuard, ChannelPort};
use source_types::PortError;

use crate::channel_pid::pid_alive;
use crate::root::repo_root;

/// Repair lock max age before treated stale (even if pid look-alive fails).
pub const REPAIR_STALE_S: f64 = 15.0 * 60.0;
/// Bulk runner lock max age.
pub const BULK_STALE_S: f64 = 2.0 * 3600.0;

/// Exclusive repair vs bulk lock under `<root>/temp/`.
pub struct FsChannelPort {
    root: PathBuf,
}

impl FsChannelPort {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_repo() -> Result<Self, PortError> {
        Ok(Self::new(repo_root()?))
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join("temp/mcp_channel.lock")
    }

    /// Hold bulk runner lock (`temp/full_check/runner.lock`) for batch MCP check.
    pub fn acquire_bulk(&self) -> Result<FsChannelGuard, PortError> {
        let snap = status(&self.root)?;
        if snap.get("idle").and_then(|v| v.as_bool()) != Some(true) {
            return Err(PortError::ChannelBusy(format!(
                "MCP channel busy: {}",
                snap.get("holders").unwrap_or(&json!([]))
            )));
        }
        acquire_lock(
            &self.root,
            "bulk",
            &self.root.join("temp/full_check/runner.lock"),
        )
    }
}

/// Drop releases repair lock when pid matches.
pub struct FsChannelGuard {
    root: PathBuf,
    owner: String,
    pid: u32,
}

impl ChannelGuard for FsChannelGuard {}

impl Drop for FsChannelGuard {
    fn drop(&mut self) {
        let _ = release(&self.root, &self.owner, self.pid);
    }
}

impl ChannelPort for FsChannelPort {
    type Guard = FsChannelGuard;

    fn assert_idle_for_repair(&self) -> Result<(), PortError> {
        let snap = status(&self.root)?;
        for h in snap
            .get("holders")
            .and_then(|h| h.as_array())
            .into_iter()
            .flatten()
        {
            let owner = h.get("owner").and_then(|o| o.as_str()).unwrap_or("");
            let path = h.get("path").and_then(|p| p.as_str()).unwrap_or("");
            if owner == "bulk" || path.contains("runner.lock") {
                return Err(PortError::ChannelBusy(format!(
                    "Refuse repair/verify while bulk holds MCP: {h}. \
                     Stop full_check_runner / batch_check_mcp first."
                )));
            }
        }
        Ok(())
    }

    fn acquire_repair(&self) -> Result<Self::Guard, PortError> {
        self.assert_idle_for_repair()?;
        let snap = status(&self.root)?;
        if snap.get("idle").and_then(|v| v.as_bool()) != Some(true) {
            return Err(PortError::ChannelBusy(format!(
                "MCP channel busy: {}",
                snap.get("holders").unwrap_or(&json!([]))
            )));
        }
        acquire_lock(&self.root, "repair", &self.lock_path())
    }
}

fn acquire_lock(root: &Path, owner: &str, lock: &Path) -> Result<FsChannelGuard, PortError> {
    let pid = std::process::id();
    if let Some(parent) = lock.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| PortError::Permanent(format!("mkdir {}: {e}", parent.display())))?;
    }
    let payload = json!({
        "owner": owner,
        "role": owner,
        "pid": pid,
        "mtime": now_s(),
    });
    fs::write(lock, payload.to_string())
        .map_err(|e| PortError::Permanent(format!("write lock: {e}")))?;
    Ok(FsChannelGuard {
        root: root.to_path_buf(),
        owner: owner.into(),
        pid,
    })
}

fn now_s() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub fn status(root: &Path) -> Result<Value, PortError> {
    let cleared = clear_stale_locks(root)?;
    let paths = [
        (root.join("temp/mcp_channel.lock"), "repair"),
        (root.join("temp/full_check/runner.lock"), "bulk"),
    ];
    let mut holders = Vec::new();
    for (path, default_owner) in paths {
        let Some(mut info) = read_lock(&path)? else {
            continue;
        };
        if info.get("owner").is_none() {
            if let Some(obj) = info.as_object_mut() {
                obj.insert("owner".into(), json!(default_owner));
            }
        }
        holders.push(json!({
            "path": path.display().to_string(),
            "owner": info.get("owner").cloned().unwrap_or(json!(default_owner)),
            "pid": info.get("pid").cloned().unwrap_or(json!(null)),
            "role": info.get("role").cloned().unwrap_or(json!(null)),
        }));
    }
    Ok(json!({
        "idle": holders.is_empty(),
        "holders": holders,
        "cleared_stale": cleared,
        "repair_stale_s": REPAIR_STALE_S,
        "bulk_stale_s": BULK_STALE_S,
    }))
}

/// Remove locks whose owner PID is dead or older than stale threshold.
pub fn clear_stale_locks(root: &Path) -> Result<usize, PortError> {
    let paths = [
        (root.join("temp/mcp_channel.lock"), REPAIR_STALE_S),
        (root.join("temp/full_check/runner.lock"), BULK_STALE_S),
    ];
    let mut n = 0;
    for (path, stale_s) in paths {
        let Some(info) = read_lock(&path)? else {
            continue;
        };
        if is_stale(&info, &path, stale_s) {
            let _ = fs::remove_file(&path);
            n += 1;
        }
    }
    Ok(n)
}

/// Force-remove repair + bulk locks (agent recovery after hung live PID).
pub fn force_clear_locks(root: &Path) -> Result<usize, PortError> {
    let paths = [
        root.join("temp/mcp_channel.lock"),
        root.join("temp/full_check/runner.lock"),
    ];
    let mut n = 0;
    for path in paths {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| PortError::Permanent(format!("remove {}: {e}", path.display())))?;
            n += 1;
        }
    }
    Ok(n)
}

fn read_lock(path: &Path) -> Result<Option<Value>, PortError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| PortError::Permanent(format!("read {}: {e}", path.display())))?;
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(Some(json!({
                "owner": "unknown",
                "pid": 0,
                "mtime": mtime_s(path),
            }))),
        }
    } else {
        let pid: u64 = trimmed.parse().unwrap_or(0);
        Ok(Some(json!({
            "owner": "bulk",
            "pid": pid,
            "mtime": mtime_s(path),
        })))
    }
}

fn mtime_s(path: &Path) -> f64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn is_stale(info: &Value, path: &Path, stale_s: f64) -> bool {
    let mtime = info
        .get("mtime")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| mtime_s(path));
    if now_s() - mtime > stale_s {
        return true;
    }
    let pid = info.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    pid != 0 && !pid_alive(pid)
}

fn release(root: &Path, owner: &str, pid: u32) -> Result<(), PortError> {
    let paths: Vec<PathBuf> = if owner == "bulk" {
        vec![
            root.join("temp/mcp_channel.lock"),
            root.join("temp/full_check/runner.lock"),
        ]
    } else {
        vec![root.join("temp/mcp_channel.lock")]
    };
    for path in paths {
        let Some(info) = read_lock(&path)? else {
            continue;
        };
        let file_pid = info.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        if file_pid == pid || owner == "bulk" {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_and_drop_releases() {
        let dir = TempDir::new().unwrap();
        let port = FsChannelPort::new(dir.path());
        port.assert_idle_for_repair().unwrap();
        {
            let _g = port.acquire_repair().unwrap();
            assert!(port.lock_path().exists());
            let busy = port.acquire_repair();
            assert!(matches!(busy, Err(PortError::ChannelBusy(_))));
        }
        assert!(!port.lock_path().exists());
    }

    #[test]
    fn bulk_runner_blocks_repair() {
        let dir = TempDir::new().unwrap();
        let bulk = dir.path().join("temp/full_check/runner.lock");
        fs::create_dir_all(bulk.parent().unwrap()).unwrap();
        fs::write(&bulk, format!("{}", std::process::id())).unwrap();
        let port = FsChannelPort::new(dir.path());
        let err = port.assert_idle_for_repair().unwrap_err();
        assert!(matches!(err, PortError::ChannelBusy(_)));
    }

    #[test]
    fn clear_stale_removes_dead_or_aged_lock() {
        let dir = TempDir::new().unwrap();
        let lock = dir.path().join("temp/mcp_channel.lock");
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(
            &lock,
            json!({"owner":"repair","pid":1u32,"mtime": now_s() - REPAIR_STALE_S - 10.0})
                .to_string(),
        )
        .unwrap();
        let n = clear_stale_locks(dir.path()).unwrap();
        assert!(n >= 1);
        assert!(!lock.exists());
    }

    #[test]
    fn force_clear_removes_live_lock() {
        let dir = TempDir::new().unwrap();
        let lock = dir.path().join("temp/mcp_channel.lock");
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(
            &lock,
            json!({"owner":"repair","pid": std::process::id(),"mtime": now_s()}).to_string(),
        )
        .unwrap();
        let n = force_clear_locks(dir.path()).unwrap();
        assert_eq!(n, 1);
        assert!(!lock.exists());
    }
}
