//! Hunt policy decision after optional L2 probe.

use crate::seeds::{hostname, hunt_candidates, HuntSeeds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HuntAction {
    NoMirror,
    Migrate,
    WeakCandidate,
    OriginalAlive,
    NoneAlive,
    CandidatesOnly,
    Empty,
}

impl HuntAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoMirror => "no_mirror",
            Self::Migrate => "migrate",
            Self::WeakCandidate => "weak_candidate",
            Self::OriginalAlive => "original_alive",
            Self::NoneAlive => "none_alive",
            Self::CandidatesOnly => "candidates_only",
            Self::Empty => "empty",
        }
    }

    pub fn should_disable(self) -> bool {
        matches!(self, Self::NoMirror | Self::NoneAlive | Self::Empty)
    }

    pub fn should_migrate(self) -> bool {
        matches!(self, Self::Migrate)
    }
}

fn urls_same_host_path(a: &str, b: &str) -> bool {
    let norm = |u: &str| {
        u.split('#')
            .next()
            .unwrap_or(u)
            .trim_end_matches('/')
            .replace("https://", "http://")
            .to_ascii_lowercase()
    };
    norm(a) == norm(b)
}

pub fn decide_hunt_action(
    shutdown: bool,
    confidence: &str,
    best: Option<&str>,
    original_url: &str,
    probed: bool,
    had_candidates: bool,
) -> HuntAction {
    if shutdown {
        return HuntAction::NoMirror;
    }
    if !had_candidates {
        return HuntAction::Empty;
    }
    let Some(best) = best else {
        return if probed {
            HuntAction::NoneAlive
        } else {
            HuntAction::CandidatesOnly
        };
    };
    if urls_same_host_path(best, original_url) {
        return HuntAction::OriginalAlive;
    }
    if confidence == "low" {
        return HuntAction::WeakCandidate;
    }
    HuntAction::Migrate
}

#[derive(Debug, Clone)]
pub struct HuntResolve {
    pub url: String,
    pub host: String,
    pub note: Option<String>,
    pub shutdown: bool,
    pub confidence: String,
    pub candidates: Vec<String>,
    pub best_candidate: Option<String>,
    pub action: HuntAction,
}

impl HuntResolve {
    pub fn from_seeds(seeds: &HuntSeeds, url: &str) -> Self {
        let host = hostname(url).unwrap_or_default();
        let entry = seeds.lookup_host(&host);
        let shutdown = entry.map(|e| e.shutdown).unwrap_or(false);
        let confidence = entry
            .and_then(|e| e.confidence.as_deref())
            .unwrap_or("normal")
            .to_string();
        let note = entry.and_then(|e| e.note.clone());
        let candidates = hunt_candidates(seeds, url);
        let action = decide_hunt_action(
            shutdown,
            &confidence,
            None,
            url,
            false,
            !candidates.is_empty(),
        );
        Self {
            url: url.to_string(),
            host,
            note,
            shutdown,
            confidence,
            candidates,
            best_candidate: None,
            action,
        }
    }

    pub fn with_best_alive(mut self, best: Option<String>) -> Self {
        self.best_candidate = best;
        self.action = decide_hunt_action(
            self.shutdown,
            &self.confidence,
            self.best_candidate.as_deref(),
            &self.url,
            true,
            !self.candidates.is_empty(),
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_migrate_vs_no_mirror() {
        assert_eq!(
            decide_hunt_action(
                true,
                "normal",
                Some("https://x/"),
                "http://old/",
                true,
                true
            ),
            HuntAction::NoMirror
        );
        assert_eq!(
            decide_hunt_action(
                false,
                "normal",
                Some("https://www.zxcs.click/"),
                "http://www.zxcs.info/",
                true,
                true
            ),
            HuntAction::Migrate
        );
        assert_eq!(
            decide_hunt_action(false, "normal", None, "http://old/", true, true),
            HuntAction::NoneAlive
        );
        assert_eq!(
            decide_hunt_action(false, "normal", None, "http://old/", true, false),
            HuntAction::Empty
        );
        assert!(HuntAction::NoneAlive.should_disable());
        assert!(HuntAction::Migrate.should_migrate());
    }
}
