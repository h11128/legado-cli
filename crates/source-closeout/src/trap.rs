//! Trap slug vs SKILL Traps section gate.

use std::path::Path;

use crate::paths::CloseoutPaths;

pub fn load_skill_text(paths: &CloseoutPaths) -> String {
    if paths.skill_sot.is_file() {
        return std::fs::read_to_string(&paths.skill_sot).unwrap_or_default();
    }
    if paths.cursor_skill.is_file() {
        return std::fs::read_to_string(&paths.cursor_skill).unwrap_or_default();
    }
    String::new()
}

fn traps_section(skill_text: &str) -> &str {
    let start = skill_text.find("## Traps").unwrap_or(0);
    if start == 0 && !skill_text.starts_with("## Traps") {
        return skill_text;
    }
    let rest = &skill_text[start..];
    if let Some(end) = rest[8..].find("\n## ") {
        &rest[..8 + end]
    } else {
        rest
    }
}

pub fn trap_in_skill(trap: &str, skill_text: &str) -> bool {
    let trap = trap.trim();
    if trap.is_empty() || trap.starts_with("known:") {
        return true;
    }
    let section = traps_section(skill_text).to_lowercase();
    // Prefer the raw slug (underscores): SKILL rows are `**apex_no_a_try_m (…)**`.
    // Expanding `_`→space then requiring len≥4 tokens falsely misses short segments
    // (`no`/`a`/`m`/`try`) and fails `section.contains("apex no a try m")`.
    let raw = trap.to_lowercase();
    if section.contains(&raw) {
        return true;
    }
    let slug = raw.replace(['_', '-'], " ");
    if section.contains(&slug) {
        return true;
    }
    let first = slug.split_whitespace().next().unwrap_or("");
    if !first.is_empty()
        && (section.contains(&format!("({first}")) || section.contains(&format!("({slug}")))
    {
        return true;
    }
    let tokens: Vec<&str> = slug.split_whitespace().filter(|t| t.len() >= 4).collect();
    if tokens.len() >= 2 {
        for line in section.lines() {
            if line.starts_with('|') && tokens.iter().all(|t| line.contains(t)) {
                return true;
            }
        }
    }
    false
}

pub fn gate_trap(
    paths: &CloseoutPaths,
    trap: &str,
    skill_fix: bool,
    harness_files: &[&Path],
    require_harness: bool,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let skill_text = load_skill_text(paths);
    if skill_text.is_empty() {
        errors.push(format!("SKILL not found: {}", paths.skill_sot.display()));
        return Err(errors);
    }
    let novel = !trap_in_skill(trap, &skill_text);
    if novel && !skill_fix {
        errors.push(format!(
            "novel trap {trap:?} not in SKILL Traps — add row to SKILL.md \
             and retro --skill-fix 1 (or use known:… if repeating a playbook)"
        ));
    }
    if novel && skill_fix && !trap_in_skill(trap, &skill_text) {
        errors.push(format!(
            "--skill-fix 1 but trap {trap:?} still missing from SKILL Traps"
        ));
    }
    if require_harness && novel {
        let missing: Vec<_> = harness_files
            .iter()
            .filter(|p| !p.is_file())
            .map(|p| p.display().to_string())
            .collect();
        if !missing.is_empty() {
            errors.push(format!("novel trap requires harness file(s): {missing:?}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_trap_always_passes() {
        assert!(trap_in_skill("known:foo", ""));
    }

    #[test]
    fn underscored_slug_matches_skill_row_with_parenthetical() {
        let skill = r#"
## Traps

| Trap | Signal | Action |
|------|--------|--------|
| **apex_no_a_try_m (tongrenquan)** | apex no A; m. live | migrate |
| **dead_skip_without_hunt** | l1_unreachable disable | hunt first |

## Other
"#;
        assert!(trap_in_skill("apex_no_a_try_m", skill));
        assert!(trap_in_skill("dead_skip_without_hunt", skill));
        assert!(!trap_in_skill("totally_missing_trap_xyz", skill));
    }
}
