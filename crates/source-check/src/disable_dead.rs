//! Disable/tag dead sources from precheck JSON — parity with `disable_dead_sources.py`.
//! Live MCP writes go through `SourceRepository` (do not invent a second client).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use source_gate::is_alias_book_source_url;
use source_ports::SourceRepository;
use source_types::{BookSource, PortError, SourceKey};
use thiserror::Error;

pub const DEAD_TAG: &str = "网站失效";

#[derive(Debug, Error)]
pub enum DisableDeadError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Msg(String),
    #[error("port: {0}")]
    Port(String),
}

pub type DisableResult<T> = Result<T, DisableDeadError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisableDeadOpts {
    pub disable: bool,
    pub tag: bool,
    /// 0 = all
    pub limit: usize,
}

impl DisableDeadOpts {
    pub fn validate(&self) -> DisableResult<()> {
        if !self.disable && !self.tag {
            return Err(DisableDeadError::Msg("need disable and/or tag".into()));
        }
        Ok(())
    }
}

/// Read `dead_urls` from precheck JSON.
pub fn load_dead_urls(path: &Path) -> DisableResult<Vec<String>> {
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let urls = data
        .get("dead_urls")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(urls)
}

/// Drop alias `bookSourceUrl` labels — L1/precheck on them is a false dead signal.
pub fn filter_alias_dead_urls(urls: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut keep = Vec::new();
    let mut skipped = Vec::new();
    for u in urls {
        if is_alias_book_source_url(&u) {
            skipped.push(u);
        } else {
            keep.push(u);
        }
    }
    (keep, skipped)
}

pub fn ensure_tag(group: Option<&str>, tag: &str) -> String {
    let mut parts: Vec<String> = group
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if !parts.iter().any(|p| p == tag) {
        parts.push(tag.to_string());
    }
    parts.join(",")
}

/// Remove `tag` from a comma group (revive false 「网站失效」).
pub fn strip_tag(group: Option<&str>, tag: &str) -> String {
    group
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != tag)
        .collect::<Vec<_>>()
        .join(",")
}

pub fn apply_limit(mut urls: Vec<String>, limit: usize) -> Vec<String> {
    if limit > 0 && urls.len() > limit {
        urls.truncate(limit);
    }
    urls
}

/// Dry-run plan: no MCP I/O. Alias URLs are listed under `skipped_alias`, not applied.
pub fn plan_disable_dead(precheck_json: &Path, opts: &DisableDeadOpts) -> DisableResult<Value> {
    opts.validate()?;
    let (filtered, skipped_alias) = filter_alias_dead_urls(load_dead_urls(precheck_json)?);
    let dead = apply_limit(filtered, opts.limit);
    Ok(json!({
        "dry_run": true,
        "total": dead.len(),
        "disable": opts.disable,
        "tag": opts.tag,
        "dead_tag": DEAD_TAG,
        "urls": dead,
        "skipped_alias": skipped_alias,
        "skip_reason": "alias_bookSourceUrl_false_dead",
    }))
}

fn mutate_source(mut value: Value, opts: &DisableDeadOpts) -> Value {
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    if opts.disable {
        obj.insert("enabled".into(), json!(false));
    }
    if opts.tag {
        let group = obj
            .get("bookSourceGroup")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        obj.insert(
            "bookSourceGroup".into(),
            json!(ensure_tag(group.as_deref(), DEAD_TAG)),
        );
    }
    value
}

/// Apply disable and/or tag via existing repository (get → mutate → save).
pub fn apply_disable_dead<R: SourceRepository + ?Sized>(
    repo: &R,
    urls: &[String],
    opts: &DisableDeadOpts,
) -> DisableResult<Value> {
    opts.validate()?;
    let (dead, skipped_alias) = filter_alias_dead_urls(urls.to_vec());
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    for url in &dead {
        match apply_one(repo, url, opts) {
            Ok(()) => ok.push(json!({"url": url})),
            Err(e) => failed.push(json!({"url": url, "error": e.to_string()})),
        }
    }
    Ok(json!({
        "dry_run": false,
        "total": dead.len(),
        "ok": ok,
        "failed": failed,
        "skipped_alias": skipped_alias,
        "skip_reason": "alias_bookSourceUrl_false_dead",
    }))
}

fn apply_one<R: SourceRepository + ?Sized>(
    repo: &R,
    url: &str,
    opts: &DisableDeadOpts,
) -> Result<(), DisableDeadError> {
    let key = SourceKey::new(url);
    // Prefer disable() when both flags match repository semantics.
    if opts.disable && opts.tag {
        repo.disable(&key)
            .map_err(|e: PortError| DisableDeadError::Port(e.to_string()))?;
        return Ok(());
    }
    let src = repo
        .get(&key)
        .map_err(|e| DisableDeadError::Port(e.to_string()))?;
    let mutated = mutate_source(src.into_value(), opts);
    let book = BookSource::new(mutated);
    repo.save(&book)
        .map_err(|e| DisableDeadError::Port(e.to_string()))?;
    Ok(())
}

pub fn write_report(out: &Path, report: &Value) -> DisableResult<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, serde_json::to_string_pretty(report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use tempfile::TempDir;

    struct FakeRepo {
        store: RefCell<HashMap<String, Value>>,
    }

    impl SourceRepository for FakeRepo {
        fn get(&self, key: &SourceKey) -> Result<BookSource, PortError> {
            self.store
                .borrow()
                .get(key.as_str())
                .cloned()
                .map(BookSource::new)
                .ok_or_else(|| PortError::Permanent("missing".into()))
        }
        fn save(&self, source: &BookSource) -> Result<(), PortError> {
            let v = source.as_value().clone();
            let url = v
                .get("bookSourceUrl")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            self.store.borrow_mut().insert(url, v);
            Ok(())
        }
        fn disable(&self, key: &SourceKey) -> Result<(), PortError> {
            let mut v = self.get(key)?.into_value();
            if let Some(obj) = v.as_object_mut() {
                obj.insert("enabled".into(), json!(false));
                obj.insert(
                    "bookSourceGroup".into(),
                    json!(ensure_tag(
                        obj.get("bookSourceGroup").and_then(|x| x.as_str()),
                        DEAD_TAG
                    )),
                );
            }
            self.save(&BookSource::new(v))
        }
        fn delete(&self, _keys: &[SourceKey]) -> Result<(), PortError> {
            Ok(())
        }
    }

    #[test]
    fn dry_run_loads_dead_urls() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pre.json");
        std::fs::write(&p, r#"{"dead_urls":["https://a/","https://b/"]}"#).unwrap();
        let plan = plan_disable_dead(
            &p,
            &DisableDeadOpts {
                disable: true,
                tag: true,
                limit: 1,
            },
        )
        .unwrap();
        assert_eq!(plan["total"], 1);
        assert_eq!(plan["dry_run"], true);
    }

    #[test]
    fn dry_run_skips_alias_urls() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pre.json");
        std::fs::write(
            &p,
            r#"{"dead_urls":["QQ浏览器","https://dead.example/"]}"#,
        )
        .unwrap();
        let plan = plan_disable_dead(
            &p,
            &DisableDeadOpts {
                disable: true,
                tag: true,
                limit: 0,
            },
        )
        .unwrap();
        assert_eq!(plan["total"], 1);
        assert_eq!(plan["urls"][0], "https://dead.example/");
        assert_eq!(plan["skipped_alias"][0], "QQ浏览器");
    }

    #[test]
    fn apply_skips_alias_without_touching_repo() {
        let repo = FakeRepo {
            store: RefCell::new(HashMap::from([(
                "QQ浏览器".into(),
                json!({
                    "bookSourceUrl": "QQ浏览器",
                    "enabled": true,
                    "bookSourceGroup": "小说"
                }),
            )])),
        };
        let report = apply_disable_dead(
            &repo,
            &["QQ浏览器".into()],
            &DisableDeadOpts {
                disable: true,
                tag: true,
                limit: 0,
            },
        )
        .unwrap();
        assert_eq!(report["total"], 0);
        assert_eq!(report["skipped_alias"][0], "QQ浏览器");
        assert_eq!(repo.store.borrow()["QQ浏览器"]["enabled"], true);
        assert_eq!(repo.store.borrow()["QQ浏览器"]["bookSourceGroup"], "小说");
    }

    #[test]
    fn apply_tag_only() {
        let repo = FakeRepo {
            store: RefCell::new(HashMap::from([(
                "https://a/".into(),
                json!({
                    "bookSourceUrl": "https://a/",
                    "enabled": true,
                    "bookSourceGroup": "小说"
                }),
            )])),
        };
        let report = apply_disable_dead(
            &repo,
            &["https://a/".into()],
            &DisableDeadOpts {
                disable: false,
                tag: true,
                limit: 0,
            },
        )
        .unwrap();
        assert_eq!(report["failed"].as_array().unwrap().len(), 0);
        let g = repo.store.borrow()["https://a/"]["bookSourceGroup"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(g.contains(DEAD_TAG));
        assert_eq!(repo.store.borrow()["https://a/"]["enabled"], true);
    }

    #[test]
    fn strip_tag_removes_dead() {
        assert_eq!(strip_tag(Some("小说,网站失效"), DEAD_TAG), "小说");
        assert_eq!(strip_tag(Some("网站失效"), DEAD_TAG), "");
    }

    #[test]
    fn dry_run_limit_after_alias_filter() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pre.json");
        std::fs::write(
            &p,
            r#"{"dead_urls":["QQ浏览器","DragonQuestQBall","https://dead1.example/","https://dead2.example/"]}"#,
        )
        .unwrap();
        let plan = plan_disable_dead(
            &p,
            &DisableDeadOpts {
                disable: true,
                tag: true,
                limit: 1,
            },
        )
        .unwrap();
        assert_eq!(plan["total"], 1);
        assert_eq!(plan["urls"][0], "https://dead1.example/");
        assert_eq!(plan["skipped_alias"].as_array().unwrap().len(), 2);
    }
}
