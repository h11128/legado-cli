//! Seed JSON load + candidate expansion.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

use crate::HuntError;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SeedEntry {
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub migrated_to: Option<String>,
    #[serde(default)]
    pub shutdown: bool,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HuntSeeds {
    #[serde(default)]
    pub seeds: HashMap<String, SeedEntry>,
}

impl HuntSeeds {
    pub fn load_path(path: &Path) -> Result<Self, HuntError> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn resolve_path(explicit: Option<&Path>) -> PathBuf {
        if let Some(p) = explicit {
            return p.to_path_buf();
        }
        if let Ok(env) = std::env::var("DOMAIN_HUNT_SEEDS") {
            return PathBuf::from(env);
        }
        PathBuf::from("config/domain_hunt_seeds.json")
    }

    pub fn lookup_host(&self, host: &str) -> Option<&SeedEntry> {
        let h = host.trim_end_matches('.').to_ascii_lowercase();
        if let Some(e) = self.seeds.get(&h) {
            return Some(e);
        }
        if let Some(rest) = h.strip_prefix("www.") {
            if let Some(e) = self.seeds.get(rest) {
                return Some(e);
            }
        }
        self.seeds.get(&format!("www.{h}"))
    }
}

pub fn hostname(raw: &str) -> Option<String> {
    let base = raw.split('#').next().unwrap_or(raw);
    let base = base.split("##").next().unwrap_or(base).trim();
    let with = if base.contains("://") {
        base.to_string()
    } else {
        format!("http://{base}")
    };
    Url::parse(&with)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
}

fn entry_candidates(entry: &SeedEntry) -> Vec<String> {
    if entry.shutdown {
        return Vec::new();
    }
    let mut v = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let push = |c: &str, v: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
        if seen.insert(c.to_string()) {
            v.push(c.to_string());
        }
    };
    if let Some(to) = &entry.migrated_to {
        push(to, &mut v, &mut seen);
    }
    for c in &entry.candidates {
        push(c, &mut v, &mut seen);
    }
    v
}

pub fn hunt_candidates(seeds: &HuntSeeds, book_source_url: &str) -> Vec<String> {
    let host = hostname(book_source_url).unwrap_or_default();
    if host.is_empty() {
        return Vec::new();
    }
    seeds
        .lookup_host(&host)
        .map(entry_candidates)
        .unwrap_or_default()
}

pub fn hunt_candidates_by_keyword(seeds: &HuntSeeds, keyword: &str) -> Vec<String> {
    let kw = keyword.to_ascii_lowercase();
    if kw.is_empty() {
        return Vec::new();
    }
    let mut ordered = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (k, entry) in &seeds.seeds {
        let note = entry.note.as_deref().unwrap_or("");
        let mig = entry.migrated_to.as_deref().unwrap_or("");
        let hay = format!("{k} {note} {mig}").to_ascii_lowercase();
        if !hay.contains(&kw) {
            continue;
        }
        for c in entry_candidates(entry) {
            if seen.insert(c.clone()) {
                ordered.push(c);
            }
        }
    }
    ordered
}

pub fn seed_candidates(
    seeds_path: Option<&Path>,
    dead_url: &str,
    keyword: Option<&str>,
) -> Result<Vec<String>, HuntError> {
    let path = HuntSeeds::resolve_path(seeds_path);
    let seeds = HuntSeeds::load_path(&path)?;
    Ok(match keyword {
        Some(kw) if !kw.is_empty() => hunt_candidates_by_keyword(&seeds, kw),
        _ => hunt_candidates(&seeds, dead_url),
    })
}
