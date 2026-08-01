//! Full L0→L1→L2 classify — parity with `repair_prefilter.classify_one`.

use source_types::{GateAction, GateResult, MigrateTarget};

use crate::l0::{match_l0, SkipRule};
use crate::l1::probe_l1;
use crate::l2::probe_l2;
use crate::url_util::to_gate_url;

/// Timeouts for L1 TCP and L2 HTTP (seconds).
#[derive(Debug, Clone, Copy)]
pub struct ClassifyOpts {
    pub tcp_timeout_s: f64,
    pub l2_timeout_s: f64,
}

impl Default for ClassifyOpts {
    fn default() -> Self {
        Self {
            tcp_timeout_s: 1.5,
            l2_timeout_s: 4.0,
        }
    }
}

/// L0 denylist → L1 DNS/TCP → L2 HTTP sniff. Always full classify when called.
pub fn classify_one(url: &str, rules: &[SkipRule], opts: &ClassifyOpts) -> GateResult {
    if let Some(hit) = match_l0(url, rules) {
        return GateResult::l0_deny(to_gate_url(url), hit);
    }

    // Alias labels (`QQ浏览器`, `DragonQuestQBall`, …) are not hosts — L1/L2 on
    // `http://{label}` false-hunts → false 「网站失效」. Defer to MCP debug/check.
    if crate::alias_url::is_alias_book_source_url(url) {
        return GateResult::new(
            to_gate_url(url),
            GateAction::Verify,
            crate::alias_url::ALIAS_GATE_REASON,
        );
    }

    let l1 = probe_l1(url, opts.tcp_timeout_s);
    if !l1.ok {
        // Policy: timeout/unreachable → Hunt before hard-disable (except L0
        // dead_site_shutdown_confirmed which never reaches here).
        let mut out = GateResult::new(to_gate_url(url), GateAction::Hunt, "l1_unreachable");
        out.verify = false;
        out.l1 = Some(l1);
        return out;
    }

    let l2 = probe_l2(url, opts.l2_timeout_s);
    if !l2.ok {
        let (reason, action) = deadish_reason_action(l2.deadish.as_deref());
        let mut out = GateResult::new(to_gate_url(url), action, reason);
        out.verify = false;
        out.l1 = Some(l1);
        out.l2 = Some(l2);
        return out;
    }

    let mut out = GateResult::new(to_gate_url(url), GateAction::Verify, "passed_l0_l1_l2");
    out.verify = true;
    if l2.host_migrated == Some(true) {
        let to = l2.to_host.clone();
        let to_l = to
            .as_ref()
            .map(|h| h.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        // Interstitial / security wrappers are not novel successors.
        if to_l.contains("safebrowse") || to_l.contains("safebrowsing") || to_l.contains("urldance")
        {
            out.action = GateAction::Disable;
            out.reason = format!("l2_hijack_redirect:{to_l}");
            out.verify = false;
        } else {
            out.action = GateAction::Migrate;
            out.reason = "l2_host_redirect".into();
            out.verify = false;
            if let Some(h) = to {
                out.migrate_to = Some(MigrateTarget::Host(h));
            }
        }
    }
    out.l1 = Some(l1);
    out.l2 = Some(l2);
    out
}

fn deadish_reason_action(dead: Option<&str>) -> (&'static str, GateAction) {
    let dead = dead.unwrap_or("");
    if dead.starts_with("wall:") {
        ("l2_password_or_db_wall", GateAction::Skip)
    } else if dead.starts_with("deadish:") {
        // Parked / ad-hijack / nginx shell — not a successor hunt target.
        ("l2_domain_parked_or_expired", GateAction::Disable)
    } else if dead.starts_with("shell:") {
        ("l2_bot_shell", GateAction::Skip)
    } else {
        // HTTP dead / timeout body — hunt seeds before disable.
        ("l2_http_dead", GateAction::Hunt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::load_rules;
    use crate::l2::probe_from_html_fixture;
    use std::path::PathBuf;

    fn rules() -> Vec<SkipRule> {
        let p =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/verify_skip_rules.json");
        load_rules(&p).expect("rules")
    }

    #[test]
    fn l0_short_circuits_before_network() {
        let r = classify_one(
            "https://www.qidian.com/book/1",
            &rules(),
            &ClassifyOpts::default(),
        );
        assert!(!r.verify);
        assert_eq!(r.reason, "waf_official");
        assert!(r.l1.is_none());
        assert!(r.l2.is_none());
    }

    #[test]
    fn alias_url_skips_host_probe() {
        let r = classify_one("QQ浏览器", &rules(), &ClassifyOpts::default());
        assert!(r.verify);
        assert_eq!(r.action, GateAction::Verify);
        assert_eq!(r.reason, crate::alias_url::ALIAS_GATE_REASON);
        assert!(r.l1.is_none());
        assert!(r.l2.is_none());
    }

    #[test]
    fn deadish_maps_reasons() {
        let wall = probe_from_html_fixture(200, "https://x/", "<html>请输入密码</html>");
        assert!(!wall.ok);
        let (reason, action) = deadish_reason_action(wall.deadish.as_deref());
        assert_eq!(reason, "l2_password_or_db_wall");
        assert_eq!(action, GateAction::Skip);

        let park = probe_from_html_fixture(200, "https://x/", "<html>domain has expired</html>");
        let (reason, action) = deadish_reason_action(park.deadish.as_deref());
        assert_eq!(reason, "l2_domain_parked_or_expired");
        assert_eq!(action, GateAction::Disable);

        let shell =
            probe_from_html_fixture(200, "https://x/", "<html>cf-browser-verification</html>");
        let (reason, action) = deadish_reason_action(shell.deadish.as_deref());
        assert_eq!(reason, "l2_bot_shell");
        assert_eq!(action, GateAction::Skip);

        let http_dead = deadish_reason_action(None);
        assert_eq!(http_dead.0, "l2_http_dead");
        assert_eq!(http_dead.1, GateAction::Hunt);
    }
}
