//! Detect alias `bookSourceUrl` values that are not real HTTP hosts.
//!
//! Some Legado sources use labels like `QQ浏览器` / `DragonQuestQBall` as
//! `bookSourceUrl` while `searchUrl` points at absolute `https://…` APIs.
//! Treating the label as a dead host (L1 / MCP GET / bulk 404) false-tags
//! 「网站失效」— see 松鹤阅读 / QB 书源 (2026-08-01).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Machine reason when gate skips L1/L2 on an alias URL.
pub const ALIAS_GATE_REASON: &str = "alias_bookSourceUrl_skip_host_probe";

const LOCAL_HOSTS: &[&str] = &["localhost", "local", "broadcasthost"];

/// Strip fragment / comment suffix used in Legado URLs.
pub fn book_url_base(url: &str) -> &str {
    url.split("##")
        .next()
        .unwrap_or(url)
        .split('#')
        .next()
        .unwrap_or(url)
        .trim()
}

fn hostish_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^[a-z0-9.-]+\.[a-z0-9.-]+$").expect("hostish"))
}

fn abs_http_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"https?://[^\s"'`]+"#).expect("abs http"))
}

/// Host without port; supports `host:port` and `[ipv6]:port`.
fn host_without_port(host: &str) -> &str {
    if let Some(rest) = host.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &host[..=end + 1]; // include brackets
        }
        return host;
    }
    if let Some((h, port)) = host.rsplit_once(':') {
        if !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return h;
        }
    }
    host
}

fn parse_ip(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    bare.parse::<std::net::IpAddr>().is_ok()
}

fn looks_like_app_alias(host: &str) -> bool {
    if host.chars().any(|c| !c.is_ascii()) {
        return true;
    }
    if host.chars().any(|c| c.is_ascii_uppercase()) {
        return true;
    }
    let lower = host.to_ascii_lowercase();
    lower.contains("dragonquest")
        || lower.contains("qqbrowser")
        || lower.starts_with("sjsw")
        || lower.contains("西瓜")
}

/// True when `bookSourceUrl` is an app/alias label, not a real host.
///
/// Also true for padded forms like `http://QQ浏览器/` / `http://DragonQuestQBkd1/`
/// (gate `ensure_scheme`), so diagnose tips still fire after URL normalization.
pub fn is_alias_book_source_url(url: &str) -> bool {
    let base = book_url_base(url);
    if base.is_empty() {
        return false;
    }
    let lower = base.to_ascii_lowercase();
    let (had_scheme, without_scheme) = if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        // Preserve original host casing from `base` (same byte offsets as lower for ASCII schemes).
        let prefix_len = base.len() - rest.len();
        (true, &base[prefix_len..])
    } else {
        (false, base)
    };
    let host_raw = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host_raw = host_raw.split('@').next_back().unwrap_or(host_raw);
    let host = host_without_port(host_raw);
    if host.is_empty() {
        return false;
    }
    if parse_ip(host) {
        return false;
    }
    let host_cmp = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    if LOCAL_HOSTS.iter().any(|h| host_cmp.eq_ignore_ascii_case(h)) {
        return false;
    }
    if hostish_re().is_match(host_cmp) {
        return false;
    }
    if !had_scheme {
        // Bare `QQ浏览器` / `DragonQuestQBall` / `sjsw:youke`.
        return true;
    }
    // Padded ensure_scheme on alias labels — not every single-label host.
    looks_like_app_alias(host_cmp)
}

/// First absolute http(s) URL found inside searchUrl (incl. @js / JSON options).
pub fn first_absolute_http(search_url: &str) -> Option<String> {
    abs_http_re().find(search_url).map(|m| {
        m.as_str()
            .trim_end_matches([',', ')', ']', ';', '"', '\''])
            .to_string()
    })
}

/// Prefer probing/searching this URL instead of alias bookSourceUrl.
pub fn effective_probe_hint(book_source_url: &str, search_url: &str) -> Option<String> {
    if !is_alias_book_source_url(book_source_url) {
        return None;
    }
    first_absolute_http(search_url)
}

/// Do not auto-tag 「网站失效」 from bookSourceUrl reachability alone.
pub fn refuse_dead_tag_reason(book_source_url: &str, search_url: &str) -> Option<&'static str> {
    if !is_alias_book_source_url(book_source_url) {
        return None;
    }
    if first_absolute_http(search_url).is_some() {
        Some("alias_bookSourceUrl_with_absolute_searchUrl")
    } else {
        Some("alias_bookSourceUrl_probe_searchUrl_or_debug_first")
    }
}

pub fn classify_alias_row(book_source_url: &str, search_url: &str, name: &str) -> Value {
    let alias = is_alias_book_source_url(book_source_url);
    let abs = first_absolute_http(search_url);
    json!({
        "bookSourceUrl": book_source_url,
        "bookSourceName": name,
        "alias": alias,
        "absolute_search": abs.is_some(),
        "probe_hint": abs,
        "refuse_dead_tag": refuse_dead_tag_reason(book_source_url, search_url),
        "advice": if alias && abs.is_some() {
            "Do NOT tag 网站失效 from bookSourceUrl L1/404 — debug_source / check with this key; \
             verify search→toc→content. False dead trap: alias_booksourceurl_false_dead"
        } else if alias {
            "Alias URL without absolute searchUrl — inspect loginUrl/@js before disable"
        } else {
            "normal http(s) bookSourceUrl"
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_qq_alias() {
        assert!(is_alias_book_source_url("QQ浏览器"));
        assert!(is_alias_book_source_url("DragonQuestQBall"));
        assert!(is_alias_book_source_url("黑岩阅读"));
        assert!(is_alias_book_source_url("sjsw:youke"));
        assert!(is_alias_book_source_url("http://QQ浏览器/"));
        assert!(is_alias_book_source_url("http://DragonQuestQBkd1/"));
        assert!(is_alias_book_source_url("http://dragonquestqball/"));
        assert!(!is_alias_book_source_url("https://so.html5.qq.com"));
        assert!(!is_alias_book_source_url("HTTP://so.html5.qq.com"));
        assert!(!is_alias_book_source_url("www.ibiquxs.org"));
        assert!(!is_alias_book_source_url("http://www.kanunu8.com"));
        assert!(!is_alias_book_source_url("http://1.2.3.4/"));
        assert!(!is_alias_book_source_url("http://[::1]/"));
        assert!(!is_alias_book_source_url("https://a/"));
        assert!(!is_alias_book_source_url("http://localhost/"));
        assert!(!is_alias_book_source_url("http://localhost:8080/"));
        assert!(!is_alias_book_source_url("localhost"));
    }

    #[test]
    fn extracts_abs_search() {
        let su = "https://so.html5.qq.com/ajax/real/search_result?tabId=360&q={{key}}";
        assert_eq!(
            first_absolute_http(su).as_deref(),
            Some("https://so.html5.qq.com/ajax/real/search_result?tabId=360&q={{key}}")
        );
        let js = "@js:\n`http://s1.ftn178.com/s?q={{key}}`";
        assert!(first_absolute_http(js)
            .unwrap()
            .starts_with("http://s1.ftn178.com"));
        assert!(refuse_dead_tag_reason("QQ浏览器", su).is_some());
        assert!(refuse_dead_tag_reason("https://a.com", su).is_none());
    }
}
