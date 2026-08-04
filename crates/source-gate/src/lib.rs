//! PC gate: L0 denylist + L1 DNS/TCP + L2 HTTP sniff (Phase B/C).

mod alias_url;
#[cfg(feature = "l2")]
mod classify;
mod error;
mod l0;
#[cfg(feature = "l1")]
mod l1;
#[cfg(feature = "l2")]
mod l2;
#[cfg(feature = "l2")]
mod sniff;
#[cfg(any(feature = "l1", feature = "l2"))]
mod url_util;

pub use alias_url::{
    book_url_base, classify_alias_row, effective_probe_hint, first_absolute_http,
    is_alias_book_source_url, refuse_dead_tag_reason, ALIAS_GATE_REASON,
};
pub use error::GateError;
pub use l0::{classify_l0, load_rules, match_l0, SkipRule};

#[cfg(feature = "l1")]
pub use l1::probe_l1;

#[cfg(feature = "l2")]
pub use classify::{classify_one, ClassifyOpts};
#[cfg(feature = "l2")]
pub use l2::probe_l2;
#[cfg(feature = "l2")]
pub use sniff::{sniff_dead_html, sniff_search_rate_limit, SEARCH_RATE_LIMIT_HINTS};

use source_types::GateResult;

pub type Result<T> = std::result::Result<T, GateError>;

/// L0-only classify: denylist hit → `verify: false`; else `passed_l0`.
pub fn classify_one_l0(url: &str, rules: &[SkipRule]) -> GateResult {
    classify_l0(url, rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use source_types::GateAction;
    use std::path::PathBuf;

    fn rules_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json")
    }

    fn load_default() -> Vec<SkipRule> {
        load_rules(rules_path()).expect("load verify_skip_rules.json")
    }

    #[test]
    fn qidian_skip_golden() {
        let rules = load_default();
        let r = classify_l0("https://www.qidian.com/book/123", &rules);
        assert!(!r.verify);
        assert_eq!(r.action, GateAction::Skip);
        assert_eq!(r.reason, "waf_official");
        let l0 = r.l0.expect("l0");
        assert_eq!(l0.rule_id, "qidian");
        assert_eq!(l0.action, GateAction::Skip);
    }

    #[test]
    fn jjwxc_and_qq_case_insensitive() {
        let rules = load_default();
        let a = classify_l0("HTTPS://WWW.JJWXC.NET/onebook.php?novelid=1", &rules);
        assert_eq!(a.l0.as_ref().unwrap().rule_id, "jjwxc");
        let b = classify_l0("https://Book.QQ.Com/book-detail/1", &rules);
        assert_eq!(b.l0.as_ref().unwrap().rule_id, "qq_read");
        assert_eq!(b.action, GateAction::Skip);
    }

    #[test]
    fn video_and_hunt_actions() {
        let rules = load_default();
        let v = classify_l0("https://www.taopianzy.com/index.php", &rules);
        assert!(!v.verify);
        assert_eq!(v.action, GateAction::Video);
        assert_eq!(v.l0.as_ref().unwrap().rule_id, "taopian");

        let h = classify_l0("http://www.dddw.net/search", &rules);
        assert_eq!(h.action, GateAction::Hunt);
        assert_eq!(h.l0.as_ref().unwrap().rule_id, "timeout_cluster");
    }

    #[test]
    fn trxs_alt_tld_and_disable() {
        let rules = load_default();
        assert_eq!(
            classify_l0("https://www.trxs.cc/foo", &rules)
                .l0
                .unwrap()
                .rule_id,
            "trxs"
        );
        let d = classify_l0("https://www.tiexue.net/", &rules);
        assert_eq!(d.action, GateAction::Disable);
        assert_eq!(d.reason, "dead_site_shutdown_confirmed");
    }

    #[test]
    fn unknown_host_passes_l0() {
        let rules = load_default();
        let r = classify_l0("https://benign-novel.example/search?q=1", &rules);
        assert!(r.verify);
        assert_eq!(r.action, GateAction::Verify);
        assert_eq!(r.reason, "passed_l0");
        assert!(r.l0.is_none());
    }

    #[test]
    fn classify_one_l0_json_shape() {
        let rules = load_default();
        let r = classify_one_l0("https://qidian.com/x", &rules);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["schema_version"], "1");
        assert_eq!(v["verify"], false);
        assert_eq!(v["action"], "skip");
        assert_eq!(v["reason"], "waf_official");
        assert_eq!(v["l0"]["rule_id"], "qidian");
    }
}
