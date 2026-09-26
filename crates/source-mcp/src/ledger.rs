//! Ledger adapters: SQLite (`source_db`) and JSONL (`repair_session_log.append_row`).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use source_db::Db;
use source_ports::LedgerPort;
use source_types::{LedgerRow, PortError};

use crate::root::repo_root;

/// Default path matching `repair_session_log.DEFAULT`.
pub fn default_jsonl_path() -> Result<PathBuf, PortError> {
    Ok(repo_root()?.join("temp/full_fix/repair_session_ledger.jsonl"))
}

/// Default SQLite repair state (`config/repair_db_defaults.json` db_path).
pub fn default_sqlite_path() -> Result<PathBuf, PortError> {
    let root = repo_root()?;
    let cfg_path = root.join("config/repair_db_defaults.json");
    if cfg_path.is_file() {
        if let Ok(raw) = std::fs::read_to_string(&cfg_path) {
            let raw = raw.trim_start_matches('\u{feff}');
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
                if let Some(p) = v.get("db_path").and_then(|x| x.as_str()) {
                    let path = PathBuf::from(p);
                    return Ok(if path.is_absolute() {
                        path
                    } else {
                        root.join(path)
                    });
                }
            }
        }
    }
    Ok(root.join("temp/full_fix/repair_state.sqlite"))
}

/// Append-only JSONL ledger (Python `append_row` shape via `LedgerRow` serde).
pub struct JsonlLedgerPort {
    path: PathBuf,
}

impl JsonlLedgerPort {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn from_defaults() -> Result<Self, PortError> {
        Ok(Self::new(default_jsonl_path()?))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl LedgerPort for JsonlLedgerPort {
    fn append(&self, row: &LedgerRow) -> Result<(), PortError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PortError::Permanent(format!("mkdir {}: {e}", parent.display())))?;
        }
        let line = serde_json::to_string(row)
            .map_err(|e| PortError::ContractViolation(format!("ledger json: {e}")))?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| PortError::Permanent(format!("open ledger: {e}")))?;
        writeln!(f, "{line}").map_err(|e| PortError::Permanent(format!("write ledger: {e}")))?;
        Ok(())
    }
}

/// SQLite ledger via `source_db::Db::append_ledger`.
pub struct SqliteLedgerPort {
    db: Mutex<Db>,
}

impl SqliteLedgerPort {
    pub fn new(db: Db) -> Self {
        Self { db: Mutex::new(db) }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, PortError> {
        let db = Db::connect(path.as_ref())
            .map_err(|e| PortError::Permanent(format!("sqlite open: {e}")))?;
        Ok(Self::new(db))
    }
}

impl LedgerPort for SqliteLedgerPort {
    fn append(&self, row: &LedgerRow) -> Result<(), PortError> {
        let db = self
            .db
            .lock()
            .map_err(|_| PortError::Transient("sqlite ledger mutex poisoned".into()))?;
        db.append_ledger(row)
            .map_err(|e| PortError::Permanent(format!("sqlite append: {e}")))?;
        Ok(())
    }
}

/// Dual-write: JSONL then SQLite (best-effort SQLite after JSONL success).
pub struct DualLedgerPort {
    jsonl: JsonlLedgerPort,
    sqlite: SqliteLedgerPort,
}

impl DualLedgerPort {
    pub fn new(jsonl: JsonlLedgerPort, sqlite: SqliteLedgerPort) -> Self {
        Self { jsonl, sqlite }
    }

    pub fn from_defaults() -> Result<Self, PortError> {
        Ok(Self::new(
            JsonlLedgerPort::from_defaults()?,
            SqliteLedgerPort::open(default_sqlite_path()?)?,
        ))
    }
}

impl LedgerPort for DualLedgerPort {
    fn append(&self, row: &LedgerRow) -> Result<(), PortError> {
        self.jsonl.append(row)?;
        self.sqlite.append(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use source_types::{LedgerStep, Url};
    use tempfile::TempDir;

    fn sample_row() -> LedgerRow {
        LedgerRow::new(
            "2026-07-27T00:00:00Z",
            Url::new("https://a.example/").unwrap(),
            LedgerStep::Check,
            "ok",
        )
    }

    #[test]
    fn jsonl_append_writes_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ledger.jsonl");
        let port = JsonlLedgerPort::new(&path);
        port.append(&sample_row()).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("https://a.example/"));
        assert!(raw.contains("\"step\":\"check\""));
    }

    #[test]
    fn sqlite_append_ok() {
        let dir = TempDir::new().unwrap();
        let port = SqliteLedgerPort::open(dir.path().join("t.sqlite")).unwrap();
        port.append(&sample_row()).unwrap();
    }
}
