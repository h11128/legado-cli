//! Improve gate: skill_fix requires a real harness claim (script_fix).

const HARNESS_NEEDLES: &[&str] = &[
    "source_patch",
    "source-patch",
    "source_probe",
    "source-probe",
    "source_check",
    "source-check",
    "source_cli",
    "source-cli",
    "source_closeout",
    "source-closeout",
    "source_queue",
    "source-queue",
    "source_adapters",
    "source-adapters",
    "source_spine",
    "source-spine",
    "source_types",
    "source-types",
    "diagnose_tips",
    "crates/",
    "crates\\",
    // Repo workflow scripts (e.g. shelf stale-tag triage) count as harness.
    "scripts/",
    "scripts\\",
];

/// True when `script_fix` is an acceptable Improve claim.
///
/// Accepts:
/// - `no_auto:<reason>` (≥8 chars of reason) — intentional no code change
/// - text mentioning a harness crate / diagnose_tips / crates/…
pub fn script_fix_ok(script_fix: &str) -> bool {
    let s = script_fix.trim();
    if s.is_empty() {
        return false;
    }
    if let Some(rest) = s.strip_prefix("no_auto:") {
        return rest.trim().chars().count() >= 8;
    }
    let lower = s.to_ascii_lowercase();
    HARNESS_NEEDLES
        .iter()
        .any(|n| lower.contains(&n.to_ascii_lowercase()))
}

/// When `skill_fix=1`, require a non-empty valid `script_fix`.
///
/// This closes the loophole where agents only add a SKILL Traps row and pass
/// close-out without touching diagnose/patch harness.
pub fn gate_script_fix(script_fix: &str, skill_fix: bool) -> Result<(), Vec<String>> {
    if !skill_fix {
        return Ok(());
    }
    if script_fix_ok(script_fix) {
        return Ok(());
    }
    Err(vec![format!(
        "skill_fix=1 requires --script-fix pointing at harness \
         (e.g. source_patch/smells.rs or diagnose_tips) or \
         no_auto:<reason≥8chars> — got {script_fix:?}"
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_crate_path() {
        assert!(script_fix_ok(
            "source_patch/smells.rs:17mb_empty_index_tocUrl"
        ));
        assert!(script_fix_ok("diagnose_tips TOC tip"));
        assert!(script_fix_ok("crates/source-probe live.rs"));
        assert!(script_fix_ok(
            "scripts/shelf-stale-tag-triage.py:hunt_probe_promote"
        ));
    }

    #[test]
    fn accepts_no_auto() {
        assert!(script_fix_ok(
            "no_auto: site-specific CSS only, not reusable"
        ));
        assert!(!script_fix_ok("no_auto: short"));
        assert!(!script_fix_ok("no_auto:"));
    }

    #[test]
    fn rejects_mcp_only_when_skill_fix() {
        assert!(!script_fix_ok("MCP save_source tocUrl"));
        assert!(gate_script_fix("MCP save_source", true).is_err());
        assert!(gate_script_fix("MCP save_source", false).is_ok());
        assert!(gate_script_fix("source_patch/smells.rs", true).is_ok());
    }
}
