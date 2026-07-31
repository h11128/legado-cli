//! Hunt-before-disable for live oneshot (policy: dead/timeout → seed probe).

use source_gate::SkipRule;
use source_hunt::HuntSeeds;
use source_migrate::migrate_book_source;
use source_ports::SourceRepository;
use source_types::{GateAction, GateResult, MigrateTarget, SourceKey, Url};

use super::hunt::{default_seeds, resolve_hunt};
use super::repair_outcome::RepairOneOutcome;

const HUNT_MIGRATE_DEPTH_MAX: u8 = 2;

/// Rewrite a Hunt gate after seed probe. Sets migrate URL when action is migrate.
pub fn resolve_hunt_gate(
    url: &str,
    rules: &[SkipRule],
    l2_timeout: f64,
    probe: bool,
    mut gate: GateResult,
) -> (GateResult, Option<String>) {
    let seeds_path = default_seeds();
    let seeds = match HuntSeeds::load_path(&seeds_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "repair: hunt seeds load warn ({e}); path={}",
                seeds_path.display()
            );
            HuntSeeds::default()
        }
    };
    let (resolved, _) = resolve_hunt(url, &seeds, rules, l2_timeout, probe);
    eprintln!(
        "repair: hunt action={} best={:?} cands={} seeds={}",
        resolved.action.as_str(),
        resolved.best_candidate,
        resolved.candidates.len(),
        seeds_path.display()
    );
    if resolved.action.should_migrate() {
        if let Some(to) = resolved.best_candidate.clone() {
            return match Url::new(to.trim()) {
                Ok(u) => {
                    gate.action = GateAction::Migrate;
                    gate.reason = format!("hunt_{}", resolved.action.as_str());
                    gate.migrate_to = Some(MigrateTarget::Url(u));
                    (gate, Some(to))
                }
                Err(e) => {
                    eprintln!("repair: hunt migrate_to bad url {to:?}: {e}");
                    gate.action = GateAction::Disable;
                    gate.reason = format!("hunt_bad_migrate_url:{e}");
                    gate.migrate_to = None;
                    (gate, None)
                }
            };
        }
    }
    if resolved.action.should_disable() {
        gate.action = GateAction::Disable;
        gate.reason = format!("hunt_{}", resolved.action.as_str());
        return (gate, None);
    }
    gate.action = GateAction::Skip;
    gate.reason = format!("hunt_{}", resolved.action.as_str());
    (gate, None)
}

/// Apply MCP migrate (caller must drop channel guard before re-entering oneshot).
pub fn apply_hunt_migrate<R: SourceRepository>(
    repo: &R,
    from_url: &str,
    to_url: &str,
    source: &source_types::BookSource,
) -> Result<(), String> {
    let mut migrated = migrate_book_source(source, from_url, to_url).map_err(|e| e.to_string())?;
    let mut v = migrated.into_value();
    v["enabled"] = serde_json::json!(true);
    migrated = source_types::BookSource::new(v);
    repo.save(&migrated).map_err(|e| e.to_string())?;
    let _ = repo.delete(&[SourceKey::new(from_url.trim())]);
    super::migrate::seal_migrated_from(from_url, to_url);
    match source_queue::refresh_phone_index(None) {
        Ok(r) => eprintln!("repair: hunt migrate refreshed index total={}", r.total),
        Err(e) => eprintln!("repair: hunt migrate index warn: {e}"),
    }
    Ok(())
}

pub fn hunt_depth_ok(depth: u8) -> bool {
    depth < HUNT_MIGRATE_DEPTH_MAX
}

pub fn depth_exceeded_outcome(url: &str, to: &str) -> RepairOneOutcome {
    RepairOneOutcome::err(
        4,
        &format!("hunt migrate depth exceeded ({url} → {to}); possible seed cycle"),
    )
}
