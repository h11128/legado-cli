//! Ledger hard-block logic for progress next.

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn norm_url(u: &str) -> String {
    u.trim().trim_end_matches('/').to_string()
}

pub fn is_fixed_row(step: &str, result: &str) -> bool {
    step == "check"
        && (result.contains(source_types::LEDGER_VERIFY_OK)
            || result.starts_with("fixed:")
            || result.starts_with("fixed "))
}

pub fn is_attempt_closed(step: &str, result: &str) -> bool {
    step == "skip"
        || step == "migrate"
        || result.starts_with("skip:")
        || result.starts_with("repurposed:")
        || result.starts_with("disable:")
        || result.starts_with("fail:")
        || result.contains("migrated_to")
        || (step == "check" && (result.starts_with("disable") || result.starts_with("fail")))
}

pub fn is_retryable_reason(reason: &str) -> bool {
    let soft = reason.contains("no_patch")
        || reason.contains("搜索")
        || reason.contains("verify_fail")
        || reason.contains("校验失败");
    let hard = reason.starts_with("disable")
        || reason.starts_with("skip:l2")
        || reason.starts_with("skip:dead")
        || reason.starts_with("skip:wall")
        || reason.starts_with("skip:park")
        || reason.starts_with("skip:migrated")
        || reason.starts_with("repurposed:")
        || reason.starts_with("skip:jieqi")
        || reason.starts_with("skip:biquge")
        || reason.contains("migrated_to")
        || reason.contains("migrated to")
        || reason.contains("search_empty")
        || reason.contains("search_index_empty")
        || reason.contains("http_dead")
        || reason.contains("domain_repurposed");
    soft && !hard
}

pub fn default_ledger() -> Option<PathBuf> {
    [
        PathBuf::from("temp/full_fix/repair_session_ledger.jsonl"),
        PathBuf::from("../temp/full_fix/repair_session_ledger.jsonl"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

pub fn blocked_from_lines<'a>(lines: impl Iterator<Item = &'a str>) -> HashSet<String> {
    let mut fixed: HashSet<String> = HashSet::new();
    let mut forced: HashSet<String> = HashSet::new();
    let mut last_closed: HashMap<String, String> = HashMap::new();
    for line in lines {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let u = norm_url(row.get("url").and_then(|v| v.as_str()).unwrap_or(""));
        if u.is_empty() {
            continue;
        }
        let result = row.get("result").and_then(|v| v.as_str()).unwrap_or("");
        let step = row.get("step").and_then(|v| v.as_str()).unwrap_or("");
        if is_fixed_row(step, result) {
            fixed.insert(u.clone());
        }
        if row.get("final").and_then(|v| v.as_bool()).unwrap_or(false) {
            forced.insert(u.clone());
        }
        if is_attempt_closed(step, result) {
            last_closed.insert(u, result.to_string());
        }
    }
    let mut hard = HashSet::new();
    for (u, reason) in last_closed {
        if fixed.contains(&u) || is_retryable_reason(&reason) {
            continue;
        }
        hard.insert(u);
    }
    hard.extend(forced);
    hard.extend(fixed);
    hard
}

pub fn ledger_blocked() -> HashSet<String> {
    let Some(path) = default_ledger() else {
        return HashSet::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    blocked_from_lines(raw.lines())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(url: &str, step: &str, result: &str) -> String {
        json!({"url": url, "step": step, "result": result}).to_string()
    }

    #[test]
    fn gave_up_fail_blocks_repick() {
        let lines = [row(
            "https://ac.qq.com",
            "check",
            "fail:content_redirect_encrypted_chapter",
        )];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("https://ac.qq.com"));
    }

    #[test]
    fn transient_fail_stays_pickable() {
        let lines = [row("https://a.test", "check", "fail:verify_fail")];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.is_empty());
    }

    #[test]
    fn later_fix_overrides_earlier_fail() {
        let lines = [
            row("https://b.test", "check", "fail:toc_empty"),
            row("https://b.test", "check", "校验成功"),
        ];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("https://b.test"));
    }

    #[test]
    fn soft_skip_and_hard_skip_differ() {
        let lines = [
            row("https://soft.test", "skip", "skip:no_patch"),
            row("https://hard.test", "skip", "skip:dead_host"),
        ];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(!blocked.contains("https://soft.test"));
        assert!(blocked.contains("https://hard.test"));
    }

    #[test]
    fn final_flag_beats_soft_wording() {
        let line =
            json!({"url": "https://d.test", "step": "check", "result": "fail:校验失败", "final": true})
                .to_string();
        let blocked = blocked_from_lines(std::iter::once(line.as_str()));
        assert!(blocked.contains("https://d.test"));
    }

    #[test]
    fn trailing_slash_is_normalized() {
        let lines = [row("https://c.test/", "check", "fail:dead")];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("https://c.test"));
    }

    #[test]
    fn migrated_from_url_is_hard_blocked() {
        let lines = [row(
            "http://m.old.test/",
            "skip",
            "skip:migrated_to:http://i.new.test/",
        )];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("http://m.old.test"));
    }

    #[test]
    fn migrate_step_is_attempt_closed() {
        let lines = [row("http://m.old.test", "migrate", "migrated_to:http://i.new.test")];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("http://m.old.test"));
    }

    #[test]
    fn migrated_to_with_spaces_is_hard() {
        let lines = [row(
            "http://m.xinbanzhu.net/",
            "skip",
            "skip: migrated to i.xinbanzhu.net (phone deleted)",
        )];
        let blocked = blocked_from_lines(lines.iter().map(String::as_str));
        assert!(blocked.contains("http://m.xinbanzhu.net"));
    }
}
