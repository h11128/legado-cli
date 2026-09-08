//! Lightweight docs/assets search — parity with `repair_knowledge.search_knowledge`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

const GLOBS: &[&str] = &["md", "txt"];
const MAX_HITS: usize = 8;
const MAX_SNIPPET: usize = 220;

fn preferred_rank(name: &str) -> i32 {
    match name {
        "essential-knowledge-summary.md" => 0,
        "toc-pagination-rules.md" => 1,
        "html-authenticity-checklist.md" => 2,
        "2026-07-28-source-repair-retrospective.md" => 3,
        _ => 50,
    }
}

fn iter_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for sub in ["docs", "assets"] {
        let dir = root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        let walk = walkdir_shallow(&dir);
        files.extend(walk);
    }
    files.sort_by(|a, b| {
        let ra = preferred_rank(a.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        let rb = preferred_rank(b.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        ra.cmp(&rb)
            .then_with(|| a.to_string_lossy().cmp(&b.to_string_lossy()))
    });
    files
}

fn walkdir_shallow(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&cur) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if GLOBS.iter().any(|g| g.eq_ignore_ascii_case(ext)) {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn split_tokens(query: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"[\s,/|]+").unwrap());
    re.split(query)
        .filter(|t| t.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

/// Search docs/assets under `root` for `query` (+ optional layer boost tokens).
pub fn search_knowledge(root: &Path, query: &str, layer: &str) -> Vec<Value> {
    let mut tokens = split_tokens(query);
    if !layer.is_empty() {
        tokens.push(layer.to_string());
    }
    let extra: &[&str] = match layer {
        "toc" => &["tocUrl", "目录", "catalog"],
        "content" => &["正文", "content"],
        "search" => &["searchUrl", "搜索"],
        _ => &[],
    };
    for e in extra {
        tokens.push((*e).to_string());
    }
    // dedupe preserve order, cap 12
    let mut seen = HashMap::new();
    let mut uniq = Vec::new();
    for t in tokens {
        if seen.insert(t.clone(), ()).is_none() {
            uniq.push(t);
        }
        if uniq.len() >= 12 {
            break;
        }
    }
    let mut hits = Vec::new();
    for path in iter_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        let mut score = 0i64;
        let mut matched = Vec::new();
        for tok in &uniq {
            let needle = tok.to_ascii_lowercase();
            let needle_short: String = needle.chars().take(40).collect();
            if lower.contains(&needle_short) || text.contains(tok.as_str()) {
                score += lower.matches(&needle_short).count() as i64;
                matched.push(tok.clone());
            }
        }
        if score <= 0 {
            continue;
        }
        let mut snippet: String = text
            .chars()
            .take(MAX_SNIPPET)
            .collect::<String>()
            .replace('\n', " ");
        for tok in &matched {
            if let Some(idx) = lower.find(&tok.to_ascii_lowercase()) {
                let start = idx.saturating_sub(60);
                snippet = text
                    .chars()
                    .skip(start)
                    .take(MAX_SNIPPET)
                    .collect::<String>()
                    .replace('\n', " ");
                break;
            }
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        hits.push(json!({
            "path": rel,
            "score": score,
            "matched": matched.into_iter().take(6).collect::<Vec<_>>(),
            "snippet": snippet,
        }));
        if hits.len() >= MAX_HITS * 3 {
            break;
        }
    }
    hits.sort_by(|a, b| {
        b.get("score")
            .and_then(|v| v.as_i64())
            .cmp(&a.get("score").and_then(|v| v.as_i64()))
    });
    hits.truncate(MAX_HITS);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn finds_token_in_docs() {
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ESSENTIAL_KNOWLEDGE_SUMMARY.md"),
            "tocUrl rules and 目录 pagination\n",
        )
        .unwrap();
        let hits = search_knowledge(dir.path(), "tocUrl", "toc");
        assert!(!hits.is_empty());
        assert!(hits[0]["path"].as_str().unwrap().contains("ESSENTIAL"));
    }
}
