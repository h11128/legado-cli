//! Load `mcp_url` + `token` from `config/mcp_defaults.json`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use source_types::PortError;

use crate::root::repo_root;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpEndpointRecord {
    pub mcp_url: String,
    #[serde(default = "default_token")]
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn default_token() -> String {
    "1234".into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpEndpoint {
    pub mcp_url: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    #[serde(default)]
    pub mcp_url: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub web_api: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<McpEndpointRecord>,
    #[serde(default)]
    pub http_timeout_s: Option<f64>,
    #[serde(default)]
    pub debug_timeout_s: Option<f64>,
    #[serde(default)]
    pub verify_timeout_ms: Option<u64>,
    #[serde(default)]
    pub verify_max_wait_s: Option<f64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub discovered_via: Option<String>,
}

impl McpEndpoint {
    pub fn new(mcp_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            mcp_url: mcp_url.into(),
            token: token.into(),
        }
    }

    /// Read shared SOT at `<repo>/config/mcp_defaults.json`.
    pub fn load_defaults() -> Result<Self, PortError> {
        let path = repo_root()?.join("config/mcp_defaults.json");
        Self::load_path(&path)
    }

    pub fn load_path(path: &Path) -> Result<Self, PortError> {
        let cfg = Self::load_config(path)?;
        let token = cfg
            .token
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "1234".into());
        if cfg.mcp_url.trim().is_empty() {
            return Err(PortError::Permanent("mcp_url empty in defaults".into()));
        }
        Ok(Self::new(cfg.mcp_url.trim(), token))
    }

    pub fn load_config(path: &Path) -> Result<DefaultsConfig, PortError> {
        let raw = fs::read_to_string(path)
            .map_err(|e| PortError::Permanent(format!("read {}: {e}", path.display())))?;
        let data: DefaultsConfig = serde_json::from_str(&raw)
            .map_err(|e| PortError::ContractViolation(format!("mcp_defaults.json: {e}")))?;
        Ok(data)
    }

    pub fn save_config(path: &Path, config: &DefaultsConfig) -> Result<(), PortError> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let pretty = serde_json::to_string_pretty(config)
            .map_err(|e| PortError::Permanent(format!("serialize defaults: {e}")))?;
        fs::write(path, pretty + "\n")
            .map_err(|e| PortError::Permanent(format!("write {}: {e}", path.display())))?;
        Ok(())
    }

    pub fn load_endpoints(path: &Path) -> Vec<McpEndpointRecord> {
        Self::load_config(path)
            .map(|c| c.endpoints)
            .unwrap_or_default()
    }

    pub fn add_or_update_endpoint(
        path: &Path,
        record: McpEndpointRecord,
        set_active: bool,
    ) -> Result<(), PortError> {
        let mut cfg = Self::load_config(path).unwrap_or_default();
        let mut found = false;
        for ep in &mut cfg.endpoints {
            if ep.mcp_url == record.mcp_url {
                ep.token = record.token.clone();
                if record.web_api.is_some() {
                    ep.web_api = record.web_api.clone();
                }
                if record.last_seen.is_some() {
                    ep.last_seen = record.last_seen.clone();
                }
                if record.note.is_some() {
                    ep.note = record.note.clone();
                }
                found = true;
                break;
            }
        }
        if !found {
            cfg.endpoints.push(record.clone());
        }
        if set_active {
            cfg.mcp_url = record.mcp_url.clone();
            cfg.token = Some(record.token.clone());
            if let Some(web_api) = &record.web_api {
                cfg.web_api = Some(web_api.clone());
            }
            cfg.updated = Some(chrono::Utc::now().format("%Y-%m-%d").to_string());
        }
        Self::save_config(path, &cfg)
    }

    pub fn switch_active(path: &Path, mcp_url: &str) -> Result<Self, PortError> {
        let mut cfg = Self::load_config(path)?;
        let mcp_url = mcp_url.trim();
        let target = cfg
            .endpoints
            .iter()
            .find(|e| e.mcp_url == mcp_url)
            .cloned();
        let (token, web_api) = if let Some(rec) = target {
            (rec.token, rec.web_api)
        } else {
            let token = cfg.token.clone().unwrap_or_else(|| "1234".into());
            let host = mcp_url
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(':')
                .next()
                .unwrap_or("127.0.0.1");
            let web_api = Some(format!("http://{host}:1122"));
            cfg.endpoints.push(McpEndpointRecord {
                mcp_url: mcp_url.to_string(),
                token: token.clone(),
                web_api: web_api.clone(),
                last_seen: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                note: Some("manual switch".into()),
            });
            (token, web_api)
        };
        cfg.mcp_url = mcp_url.to_string();
        cfg.token = Some(token.clone());
        if let Some(w) = web_api {
            cfg.web_api = Some(w);
        }
        cfg.updated = Some(chrono::Utc::now().format("%Y-%m-%d").to_string());
        Self::save_config(path, &cfg)?;
        Ok(Self::new(mcp_url, token))
    }

    pub fn remove_endpoint(path: &Path, mcp_url: &str) -> Result<bool, PortError> {
        let mut cfg = Self::load_config(path)?;
        let before_len = cfg.endpoints.len();
        cfg.endpoints.retain(|e| e.mcp_url != mcp_url.trim());
        let removed = cfg.endpoints.len() < before_len;
        if removed {
            Self::save_config(path, &cfg)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_repo_defaults() {
        let ep = McpEndpoint::load_defaults().expect("defaults");
        assert!(ep.mcp_url.starts_with("http"));
        assert!(!ep.token.is_empty());
    }

    #[test]
    fn test_multi_endpoint_management() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mcp_defaults.json");
        let initial = DefaultsConfig {
            mcp_url: "http://10.0.0.44:1236/mcp".into(),
            token: Some("1234".into()),
            web_api: Some("http://10.0.0.44:1122".into()),
            endpoints: vec![McpEndpointRecord {
                mcp_url: "http://10.0.0.44:1236/mcp".into(),
                token: "1234".into(),
                web_api: Some("http://10.0.0.44:1122".into()),
                last_seen: Some("2026-09-06".into()),
                note: Some("phone1".into()),
            }],
            ..Default::default()
        };
        McpEndpoint::save_config(&path, &initial).expect("save");

        // Add second endpoint
        McpEndpoint::add_or_update_endpoint(
            &path,
            McpEndpointRecord {
                mcp_url: "http://10.0.0.55:1236/mcp".into(),
                token: "5678".into(),
                web_api: Some("http://10.0.0.55:1122".into()),
                last_seen: Some("2026-09-06".into()),
                note: Some("phone2".into()),
            },
            false,
        )
        .expect("add");

        let endpoints = McpEndpoint::load_endpoints(&path);
        assert_eq!(endpoints.len(), 2);

        // Switch to second endpoint
        let switched = McpEndpoint::switch_active(&path, "http://10.0.0.55:1236/mcp").expect("switch");
        assert_eq!(switched.mcp_url, "http://10.0.0.55:1236/mcp");
        assert_eq!(switched.token, "5678");

        let active = McpEndpoint::load_path(&path).expect("load active");
        assert_eq!(active.mcp_url, "http://10.0.0.55:1236/mcp");

        // Remove first endpoint
        let removed = McpEndpoint::remove_endpoint(&path, "http://10.0.0.44:1236/mcp").expect("remove");
        assert!(removed);
        let endpoints = McpEndpoint::load_endpoints(&path);
        assert_eq!(endpoints.len(), 1);
    }
}
