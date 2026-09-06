//! Cluster remaining/failing sources for batch-eligibility (§ batch ROI gate).

use std::collections::HashMap;

use serde::Serialize;
use source_types::RepairConfig;

use crate::cluster::ClusterSample;
use crate::fields::search_url;
use crate::hash::{normalize_search_url_shape, structural_hash_from_source};

/// One remain bucket: exact structural_hash or a cheap trap id.
#[derive(Debug, Clone, Serialize)]
pub struct RemainBucket {
    pub bucket_key: String,
    pub kind: String,
    pub n: usize,
    pub batch_ok: bool,
    pub structural_hash: Option<String>,
    pub urls: Vec<String>,
    pub shape: Option<String>,
}

/// Summary for `queue cluster` CLI.
#[derive(Debug, Clone, Serialize)]
pub struct RemainClusterReport {
    pub schema_version: String,
    pub min_size: u32,
    pub input_n: usize,
    pub hashed_n: usize,
    pub batch_ok_buckets: usize,
    pub oneshot_n: usize,
    pub buckets: Vec<RemainBucket>,
}

/// Group samples by `structural_hash` (+ optional cheap trap overlays).
///
/// `batch_ok` when `n >= cluster_min_size`. URLs in sub-min buckets count as oneshot.
pub fn cluster_remain(samples: &[ClusterSample], config: &RepairConfig) -> RemainClusterReport {
    let min = config.cluster_min_size as usize;
    let mut by_hash: HashMap<String, Vec<&ClusterSample>> = HashMap::new();
    let mut hashed = 0usize;
    for s in samples {
        let h = structural_hash_from_source(&s.source);
        hashed += 1;
        by_hash.entry(h).or_default().push(s);
    }

    let mut buckets = Vec::new();
    for (hash, members) in &by_hash {
        let shape = members
            .first()
            .and_then(|m| search_url(&m.source))
            .map(|su| normalize_search_url_shape(&su));
        let mut urls: Vec<String> = members.iter().map(|m| m.url.as_str().to_string()).collect();
        urls.sort();
        let n = members.len();
        buckets.push(RemainBucket {
            bucket_key: format!("hash:{hash}"),
            kind: "structural_hash".into(),
            n,
            batch_ok: n >= min,
            structural_hash: Some(hash.clone()),
            urls,
            shape,
        });
    }

    // Cheap trap overlays (may overlap hash buckets; listed separately for operators).
    let mut by_trap: HashMap<String, Vec<&ClusterSample>> = HashMap::new();
    for s in samples {
        if let Some(trap) = cheap_trap_id(s) {
            by_trap.entry(trap).or_default().push(s);
        }
    }
    for (trap, members) in by_trap {
        let mut urls: Vec<String> = members.iter().map(|m| m.url.as_str().to_string()).collect();
        urls.sort();
        let n = members.len();
        buckets.push(RemainBucket {
            bucket_key: format!("trap:{trap}"),
            kind: "cheap_trap".into(),
            n,
            batch_ok: n >= min,
            structural_hash: None,
            urls,
            shape: None,
        });
    }

    buckets.sort_by(|a, b| {
        b.batch_ok
            .cmp(&a.batch_ok)
            .then(b.n.cmp(&a.n))
            .then(a.bucket_key.cmp(&b.bucket_key))
    });

    let batch_ok_buckets = buckets.iter().filter(|b| b.batch_ok).count();
    // Oneshot = URLs that appear in no batch_ok structural_hash bucket.
    let mut in_batch = std::collections::HashSet::new();
    for b in buckets
        .iter()
        .filter(|b| b.batch_ok && b.kind == "structural_hash")
    {
        for u in &b.urls {
            in_batch.insert(u.clone());
        }
    }
    let oneshot_n = samples
        .iter()
        .map(|s| s.url.as_str().to_string())
        .filter(|u| !in_batch.contains(u))
        .count();

    RemainClusterReport {
        schema_version: source_types::SCHEMA_VERSION.to_string(),
        min_size: config.cluster_min_size,
        input_n: samples.len(),
        hashed_n: hashed,
        batch_ok_buckets,
        oneshot_n,
        buckets,
    }
}

fn cheap_trap_id(sample: &ClusterSample) -> Option<String> {
    let su = search_url(&sample.source)?.to_ascii_lowercase();
    if su.contains("/e/search") || su.contains("keyboard={{key}}") {
        return Some("empire_cms_keyboard".into());
    }
    if su.contains("search.php?q=") || su.contains("xunsearch") {
        return Some("xunsearch_q".into());
    }
    if su.contains("/sa") && su.contains("post") {
        return Some("post_sa_search".into());
    }
    let toc = sample
        .source
        .as_value()
        .get("ruleToc")
        .and_then(|t| t.get("tocUrl"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if toc.is_empty() {
        // Empty tocUrl is common; only flag when chapterList looks detail-page.
        if let Some(cl) = crate::fields::chapter_list(&sample.source) {
            if cl.contains("id") || cl.contains("chapter") || cl.contains("list") {
                return Some("empty_tocUrl".into());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use source_types::{BookSource, Url};

    fn samp(url: &str, book_list: &str) -> ClusterSample {
        ClusterSample {
            url: Url::new(url).unwrap(),
            source: BookSource::new(json!({
                "bookSourceUrl": url,
                "searchUrl": "https://h/search.php?q={{key}}",
                "ruleSearch": { "bookList": book_list },
                "ruleToc": { "chapterList": ".ch", "tocUrl": "" },
                "ruleContent": { "content": "#c" },
                "bookSourceType": 0
            })),
            verify_ok: false,
        }
    }

    #[test]
    fn batch_ok_when_three_same_hash() {
        let samples = vec![
            samp("https://a.example/", ".item"),
            samp("https://b.example/", ".item"),
            samp("https://c.example/", ".item"),
            samp("https://d.example/", ".other"),
        ];
        let report = cluster_remain(&samples, &RepairConfig::default());
        assert!(report.batch_ok_buckets >= 1);
        assert_eq!(report.oneshot_n, 1);
        let hash_ok: Vec<_> = report
            .buckets
            .iter()
            .filter(|b| b.batch_ok && b.kind == "structural_hash")
            .collect();
        assert_eq!(hash_ok.len(), 1);
        assert_eq!(hash_ok[0].n, 3);
    }
}
