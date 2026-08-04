//! Timeouts from `config/mcp_defaults.json` (shared SOT with URL/token).

use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::root::repo_root;

/// Default HTTP / verify / debug budgets for phone MCP.
#[derive(Debug, Clone, PartialEq)]
pub struct McpTimeouts {
    /// ureq timeout for most `tools/call` (get/save/check…).
    pub http_timeout_s: f64,
    /// Shorter ureq timeout for `debug_source` (often the hang point).
    pub debug_timeout_s: f64,
    /// Device check `timeoutMs` passed to `start_check_sources`.
    pub verify_timeout_ms: u64,
    /// Max wall time polling `get_check_progress`.
    pub verify_max_wait_s: f64,
}

impl Default for McpTimeouts {
    fn default() -> Self {
        Self {
            http_timeout_s: 90.0,
            debug_timeout_s: 90.0,
            verify_timeout_ms: 90_000,
            verify_max_wait_s: 120.0,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct TimeoutsFile {
    #[serde(default)]
    http_timeout_s: Option<f64>,
    #[serde(default)]
    debug_timeout_s: Option<f64>,
    #[serde(default)]
    verify_timeout_ms: Option<u64>,
    #[serde(default)]
    verify_max_wait_s: Option<f64>,
}

impl McpTimeouts {
    pub fn load_defaults() -> Self {
        match repo_root() {
            Ok(root) => Self::load_path(&root.join("config/mcp_defaults.json")),
            Err(_) => Self::default(),
        }
    }

    pub fn load_path(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(data) = serde_json::from_str::<TimeoutsFile>(&raw) else {
            return Self::default();
        };
        let d = Self::default();
        Self {
            http_timeout_s: data
                .http_timeout_s
                .filter(|v| *v > 0.0)
                .unwrap_or(d.http_timeout_s),
            debug_timeout_s: data
                .debug_timeout_s
                .filter(|v| *v > 0.0)
                .unwrap_or(d.debug_timeout_s),
            verify_timeout_ms: data
                .verify_timeout_ms
                .filter(|v| *v > 0)
                .unwrap_or(d.verify_timeout_ms),
            verify_max_wait_s: data
                .verify_max_wait_s
                .filter(|v| *v > 0.0)
                .unwrap_or(d.verify_max_wait_s),
        }
    }

    /// Merge timeout keys into a defaults JSON object (discover rewrite).
    pub fn merge_into_defaults_json(existing: &Value, timeouts: &Self) -> Value {
        let mut out = existing.clone();
        let obj = out.as_object_mut().expect("defaults object");
        obj.insert("http_timeout_s".into(), json!(timeouts.http_timeout_s));
        obj.insert("debug_timeout_s".into(), json!(timeouts.debug_timeout_s));
        obj.insert(
            "verify_timeout_ms".into(),
            json!(timeouts.verify_timeout_ms),
        );
        obj.insert(
            "verify_max_wait_s".into(),
            json!(timeouts.verify_max_wait_s),
        );
        out
    }
}

/// Preserve timeout fields from an existing defaults file when rewriting URL.
pub fn timeouts_from_existing_defaults(path: &Path) -> McpTimeouts {
    if path.is_file() {
        McpTimeouts::load_path(path)
    } else {
        McpTimeouts::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_file_uses_defaults() {
        let t = McpTimeouts::load_path(Path::new("/no/such/mcp_defaults.json"));
        assert_eq!(t, McpTimeouts::default());
    }

    #[test]
    fn loads_partial_overrides() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mcp_defaults.json");
        fs::write(
            &path,
            r#"{"mcp_url":"http://x/mcp","http_timeout_s":60,"debug_timeout_s":30}"#,
        )
        .unwrap();
        let t = McpTimeouts::load_path(&path);
        assert_eq!(t.http_timeout_s, 60.0);
        assert_eq!(t.debug_timeout_s, 30.0);
        assert_eq!(t.verify_timeout_ms, 90_000);
        assert_eq!(t.verify_max_wait_s, 120.0);
    }

    #[test]
    fn merge_preserves_other_keys() {
        let existing = json!({"mcp_url":"http://a","token":"t","note":"keep"});
        let t = McpTimeouts {
            http_timeout_s: 70.0,
            ..McpTimeouts::default()
        };
        let merged = McpTimeouts::merge_into_defaults_json(&existing, &t);
        assert_eq!(merged["mcp_url"], "http://a");
        assert_eq!(merged["note"], "keep");
        assert_eq!(merged["http_timeout_s"], 70.0);
    }
}
