//! `SourceRepository` over Legado MCP tools (get/save/disable/delete).

use std::sync::Arc;

use serde_json::{json, Value};
use source_db::{host_key, iso_now, norm_source_key, Db, SourceSnapshotRow};
use source_ports::SourceRepository;
use source_types::{BookSource, PortError, SourceKey};

use crate::client::McpClient;
use crate::root::repo_root;

const DISABLE_TAG: &str = "网站失效";

/// MCP-backed source CRUD. URL trim / fragment variants match `mcp_client.get_source`.
pub struct McpSourceRepository {
    client: Arc<McpClient>,
    ready: std::sync::OnceLock<()>,
    use_cache: bool,
    force_refresh: bool,
}

impl McpSourceRepository {
    pub fn new(client: Arc<McpClient>) -> Self {
        let skip = std::env::var("REPAIR_SKIP_PHONE_CACHE")
            .ok()
            .is_some_and(|v| v == "1");
        Self {
            client,
            ready: std::sync::OnceLock::new(),
            use_cache: !skip,
            force_refresh: false,
        }
    }

    pub fn with_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    pub fn with_force_refresh(mut self, force_refresh: bool) -> Self {
        self.force_refresh = force_refresh;
        self
    }

    fn ensure_ready(&self) -> Result<(), PortError> {
        if self.ready.get().is_some() {
            return Ok(());
        }
        self.client.ensure_session()?;
        let _ = self.ready.set(());
        Ok(())
    }

    fn try_cache(&self, url: &str) -> Option<BookSource> {
        if !self.use_cache || self.force_refresh {
            return None;
        }
        let root = repo_root().ok()?;
        let (db, cfg) = Db::connect_defaults(&root).ok()?;
        let key = norm_source_key(url);
        let payload = db
            .get_source_payload_fresh(&key, cfg.source_snapshot_ttl_s)
            .ok()??;
        let src = coerce_source(&payload)?;
        let mut v = src.as_value().clone();
        v["_cache_hit"] = json!(true);
        Some(BookSource::new(v))
    }

    fn cache_delete(&self, url: &str) -> Result<(), PortError> {
        if !self.use_cache {
            return Ok(());
        }
        let root = repo_root()?;
        let (db, _cfg) =
            Db::connect_defaults(&root).map_err(|e| PortError::Permanent(format!("db: {e}")))?;
        let key = norm_source_key(url);
        db.delete_source_snapshot(&key)
            .map_err(|e| PortError::Permanent(format!("snapshot delete: {e}")))?;
        Ok(())
    }

    fn cache_put(&self, url: &str, src: &BookSource) -> Result<(), PortError> {
        if !self.use_cache {
            return Ok(());
        }
        let root = repo_root()?;
        let (db, _cfg) =
            Db::connect_defaults(&root).map_err(|e| PortError::Permanent(format!("db: {e}")))?;
        let key = norm_source_key(url);
        let mut v = src.as_value().clone();
        strip_ephemeral(&mut v);
        let payload = serde_json::to_string(&v).map_err(|e| PortError::Permanent(e.to_string()))?;
        let row = SourceSnapshotRow {
            source_key: key.clone(),
            host_key: host_key(&key),
            name: v
                .get("bookSourceName")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            book_source_type: v
                .get("bookSourceType")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            enabled: v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
            group_name: v
                .get("bookSourceGroup")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            respond_time_ms: v.get("respondTime").and_then(|x| x.as_i64()),
            payload_json: payload,
            pulled_at: iso_now(),
        };
        db.upsert_source_snapshot(&row)
            .map_err(|e| PortError::Permanent(format!("snapshot upsert: {e}")))?;
        Ok(())
    }

    fn fetch_mcp(&self, url: &str) -> Result<BookSource, PortError> {
        self.ensure_ready()?;
        let mut last = String::new();
        for cand in url_candidates(url) {
            let result = self
                .client
                .tools_call("get_source", json!({ "url": cand }))?;
            let raw = McpClient::extract_text(&result);
            last = raw.clone();
            let data = McpClient::parse_json_text(&raw);
            if let Some(src) = coerce_source(&data) {
                let mut v = src.as_value().clone();
                v["_cache_hit"] = json!(false);
                let src = BookSource::new(v);
                let _ = self.cache_put(&cand, &src);
                return Ok(src);
            }
        }
        Err(PortError::Permanent(format!(
            "unexpected get_source payload: {}",
            trunc(&last, 300)
        )))
    }

    fn get_raw(&self, url: &str) -> Result<BookSource, PortError> {
        if !self.force_refresh {
            for cand in url_candidates(url) {
                if let Some(src) = self.try_cache(&cand) {
                    return Ok(src);
                }
            }
        }
        self.fetch_mcp(url)
    }
}

impl SourceRepository for McpSourceRepository {
    fn get(&self, key: &SourceKey) -> Result<BookSource, PortError> {
        self.get_raw(key.as_str())
    }

    fn save(&self, source: &BookSource) -> Result<(), PortError> {
        self.ensure_ready()?;
        let mut v = source.as_value().clone();
        strip_ephemeral(&mut v);
        let payload = serde_json::to_string(&v)
            .map_err(|e| PortError::ContractViolation(format!("serialize source: {e}")))?;
        let _ = self.client.tools_call(
            "save_source",
            json!({
                "source": payload,
                "preserveEnabled": true,
                "preserveGroup": true,
            }),
        )?;
        if let Some(u) = v
            .get("bookSourceUrl")
            .and_then(|x| x.as_str())
            .map(str::to_string)
        {
            let _ = self.cache_put(&u, &BookSource::new(v));
        }
        Ok(())
    }

    fn disable(&self, key: &SourceKey) -> Result<(), PortError> {
        let mut value = self.fetch_mcp(key.as_str())?.into_value();
        strip_ephemeral(&mut value);
        apply_disable(&mut value, DISABLE_TAG);
        self.ensure_ready()?;
        let payload = serde_json::to_string(&value)
            .map_err(|e| PortError::ContractViolation(format!("serialize source: {e}")))?;
        let _ = self.client.tools_call(
            "save_source",
            json!({
                "source": payload,
                "preserveEnabled": false,
                "preserveGroup": false,
            }),
        )?;
        let disabled = BookSource::new(value);
        if let Some(u) = disabled
            .as_value()
            .get("bookSourceUrl")
            .and_then(|x| x.as_str())
        {
            let _ = self.cache_put(u, &disabled);
        }
        Ok(())
    }

    fn delete(&self, keys: &[SourceKey]) -> Result<(), PortError> {
        self.ensure_ready()?;
        let urls: Vec<&str> = keys.iter().map(|k| k.as_str()).collect();
        let _ = self
            .client
            .tools_call("delete_sources", json!({ "urls": urls }))?;
        for key in keys {
            for cand in url_candidates(key.as_str()) {
                let _ = self.cache_delete(&cand);
            }
        }
        Ok(())
    }
}

fn coerce_source(data: &Value) -> Option<BookSource> {
    if data.get("bookSourceUrl").is_some() {
        return Some(BookSource::new(data.clone()));
    }
    if let Some(inner) = data.get("data") {
        if inner.get("bookSourceUrl").is_some() {
            return Some(BookSource::new(inner.clone()));
        }
    }
    None
}

fn strip_ephemeral(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("_cache_hit");
    }
}

fn apply_disable(value: &mut Value, tag: &str) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    obj.insert("enabled".into(), json!(false));
    let group = obj
        .get("bookSourceGroup")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut parts: Vec<String> = group
        .replace('，', ",")
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if !parts.iter().any(|p| p == tag) {
        parts.push(tag.to_string());
    }
    obj.insert("bookSourceGroup".into(), json!(parts.join(",")));
}

/// Candidates matching Python `get_source` retry list.
pub fn url_candidates(book_source_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<String>, s: String| {
        if !s.is_empty() && !out.iter().any(|x| x == &s) {
            out.push(s);
        }
    };
    push(&mut out, book_source_url.to_string());
    let trimmed = book_source_url.trim();
    push(&mut out, trimmed.to_string());
    if !trimmed.is_empty() {
        push(&mut out, format!(" {trimmed}"));
    }
    let base = trimmed.split('#').next().unwrap_or(trimmed).trim();
    push(&mut out, base.to_string());
    let no_slash = base.trim_end_matches('/').to_string();
    push(&mut out, no_slash.clone());
    if !no_slash.is_empty() {
        push(&mut out, format!("{no_slash}/"));
    }
    out
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_candidates_trim_and_fragment() {
        let c = url_candidates("  https://a.example/#tag  ");
        assert!(c.iter().any(|x| x == "https://a.example/#tag"));
        assert!(c.iter().any(|x| x == "https://a.example/"));
        assert!(c.iter().any(|x| x == "https://a.example"));
        assert!(c.iter().any(|x| x == " https://a.example/#tag"));
    }

    #[test]
    fn apply_disable_sets_group_tag() {
        let mut v = json!({
            "bookSourceUrl": "https://a.example/",
            "enabled": true,
            "bookSourceGroup": "小说"
        });
        apply_disable(&mut v, DISABLE_TAG);
        assert_eq!(v["enabled"], false);
        assert_eq!(v["bookSourceGroup"], "小说,网站失效");
    }

    #[test]
    fn cache_hit_before_mcp() {
        use source_db::{iso_now, norm_source_key, Db, SourceSnapshotRow};
        use source_types::SourceKey;

        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_dir = dir.path().join("config");
        std::fs::create_dir_all(&cfg_dir).expect("config dir");
        std::fs::write(
            cfg_dir.join("mcp_defaults.json"),
            r#"{"mcp_url":"http://127.0.0.1:9/mcp","token":"test"}"#,
        )
        .expect("mcp defaults");
        std::fs::create_dir_all(dir.path().join("temp/full_fix")).expect("cache dir");

        let prev = std::env::var("LEGADO_SKILL_ROOT").ok();
        std::env::set_var("LEGADO_SKILL_ROOT", dir.path());

        let (db, _cfg) = Db::connect_defaults(dir.path()).expect("db");
        let url = "https://cache-hit.example/";
        let key = norm_source_key(url);
        let payload = json!({
            "bookSourceUrl": url,
            "bookSourceName": "CacheHit",
        });
        db.upsert_source_snapshot(&SourceSnapshotRow {
            source_key: key,
            host_key: source_db::host_key(url),
            name: Some("CacheHit".into()),
            book_source_type: 0,
            enabled: true,
            group_name: None,
            respond_time_ms: None,
            payload_json: payload.to_string(),
            pulled_at: iso_now(),
        })
        .expect("upsert");

        let ep = crate::McpEndpoint::load_defaults().expect("endpoint");
        let repo = McpSourceRepository::new(std::sync::Arc::new(crate::McpClient::new(ep)));
        let src = repo
            .get(&SourceKey::new(url))
            .expect("cache hit should not need MCP");
        assert_eq!(src.as_value()["_cache_hit"], json!(true));

        match prev {
            Some(p) => std::env::set_var("LEGADO_SKILL_ROOT", p),
            None => std::env::remove_var("LEGADO_SKILL_ROOT"),
        }
    }
}
