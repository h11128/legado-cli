//! Rank search-result HTML. Higher is better.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProbeScore {
    pub score: i32,
    pub reasons: Vec<String>,
    pub dead: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_list_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_url_hint: Option<String>,
}

fn pid_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)pid\s*:\s*(\d+)").unwrap())
}

fn bookish_href_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Do NOT match bare *.html (nav/sort links inflate scores on empty jieqi search).
        Regex::new(
            r#"(?i)href=["'][^"']*(?:/novel/\d|/book/\d|/txtbook/\d|/info/\d+|/xiaoshuo/)[^"']*["']"#,
        )
        .unwrap()
    })
}

fn zero_result_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"共有\s*(?:<[^>]*>\s*)*0\s*(?:<[^>]*>\s*)*条").unwrap())
}

fn jieqi_empty_contents_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)id\s*=\s*["']jieqi_page_contents["'][^>]*>\s*</(?:div|td)>"#).unwrap()
    })
}

/// Score search HTML (optional home HTML for fake-home penalty).
pub fn score_search_html(html: &str, query: &str, http_status: u16) -> ProbeScore {
    score_search_html_with_home(html, query, http_status, None)
}

pub fn score_search_html_with_home(
    html: &str,
    query: &str,
    http_status: u16,
    home_html: Option<&str>,
) -> ProbeScore {
    let mut out = ProbeScore::default();
    if http_status >= 500 {
        out.score = -100;
        out.reasons.push(format!("http_{http_status}"));
        out.dead = true;
        return out;
    }
    // 笔趣阁系 search throttle: alert(搜索间隔) / ss_search_delay — not a dead site.
    let lower_early = html.to_ascii_lowercase();
    for needle in [
        "搜索间隔",
        "ss_search_delay",
        "search interval",
        "搜索太频繁",
        "请稍后再搜索",
        "请稍后搜索",
    ] {
        if lower_early.contains(&needle.to_ascii_lowercase()) || html.contains(needle) {
            out.score = -50;
            out.reasons.push("search_rate_limit".into());
            out.dead = false;
            return out;
        }
    }
    // Cloudflare / empty error bodies often arrive as 200 with tiny payload
    if html.trim().len() < 80
        && (html.contains("error code")
            || html.contains("521")
            || html.contains("Just a moment")
            || html.is_empty())
    {
        out.score = -80;
        out.reasons.push("empty_or_cf_error".into());
        out.dead = true;
        return out;
    }

    let lower = lower_early;
    for (needle, w, tag, bl) in [
        ("id=\"sitebox\"", 5, "list_sitebox", Some("#sitebox dl")),
        ("id='sitebox'", 5, "list_sitebox", Some("#sitebox dl")),
        ("item fiction", 5, "list_xchina", Some(".item.fiction")),
        ("class=\"bookbox\"", 3, "list_bookbox", Some(".bookbox")),
        ("hot_sale", 3, "list_hot_sale", Some(".hot_sale")),
        ("result-list", 3, "list_result", Some(".result-list")),
        ("novelslist", 3, "list_novelslist", Some(".novelslist2 li")),
        ("class=\"bookname\"", 3, "list_bookname", None),
        ("txt-list", 2, "list_txt", Some(".txt-list li")),
        ("class=\"list\"", 2, "list_class_list", None),
        ("ss_box", 2, "list_ss_box", Some(".ss_box")),
    ] {
        if lower.contains(needle) {
            out.score += w;
            out.reasons.push(tag.into());
            if out.book_list_hint.is_none() {
                if let Some(h) = bl {
                    out.book_list_hint = Some(h.into());
                }
            }
        }
    }

    let bookish_n = bookish_href_re().find_iter(html).count();
    if bookish_n >= 2 {
        out.score += 3;
        out.reasons.push(format!("bookish_hrefs_{bookish_n}"));
    }

    if pid_re().is_match(html) {
        out.score += 4;
        out.reasons.push("pid_js".into());
        out.book_url_hint = Some("a@href##pid:\\s*(\\d+)##/novel/$1.html###".into());
    }

    if !query.is_empty() && html.contains(query) {
        out.score += 2;
        out.reasons.push("query_echo".into());
    }

    if lower.contains("search.php?q=") || lower.contains("xunsearch") {
        out.score += 4;
        out.reasons.push("xunsearch_shape".into());
    }

    if lower.contains("出错啦")
        || lower.contains("页面不存在")
        || lower.contains(">404<")
        || lower.contains("没找到你需要的页面")
    {
        out.score -= 8;
        out.reasons.push("error_page".into());
    }

    // Jieqi search shell with literally 0 hits (b483) — not a selector bug.
    if zero_result_re().is_match(html) || jieqi_empty_contents_re().is_match(html) {
        out.score -= 25;
        out.reasons.push("zero_search_hits".into());
    }

    let listish = out.reasons.iter().any(|r| r.starts_with("list_"));
    if !listish
        && (lower.contains("首页") || lower.contains(">home<") || lower.contains("nav-bar"))
        && html.len() < 8000
    {
        out.score -= 5;
        out.reasons.push("fake_home_penalty".into());
    }

    if let Some(home) = home_html {
        if let (Some(ht), Some(st)) = (page_title(home), page_title(html)) {
            if !ht.is_empty() && ht == st && !listish {
                out.score -= 6;
                out.reasons.push("same_title_as_home".into());
            }
        }
    }

    if lower.contains("验证码") || (lower.contains("password") && lower.contains("login")) {
        out.dead = true;
        out.reasons.push("wall".into());
    }

    // Prefer table list when class.list present
    if out.book_list_hint.is_none() && lower.contains("class=\"list\"") && lower.contains("<table")
    {
        out.book_list_hint = Some("class.list@table".into());
    }

    out
}

fn page_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let rest = &html[start..];
    let end = rest.to_ascii_lowercase().find("</title>")?;
    let inner = rest.get(7..end)?.trim();
    Some(inner.chars().take(80).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_xx_dead() {
        let s = score_search_html("<html></html>", "q", 503);
        assert!(s.dead);
        assert!(s.score < 0);
    }

    #[test]
    fn sitebox_boost() {
        let html = r#"<div id="sitebox"><dl><dt>书</dt></dl><dl><dt>书2</dt></dl></div>"#;
        let s = score_search_html(html, "书", 200);
        assert!(s.score >= 5);
        assert_eq!(s.book_list_hint.as_deref(), Some("#sitebox dl"));
    }

    #[test]
    fn error_page_penalty() {
        let s = score_search_html("<title>没找到你需要的页面--必读居</title>", "x", 200);
        assert!(s.score < 0);
    }

    #[test]
    fn jieqi_zero_hits_penalized() {
        let html = r#"<div class="c_nav">搜索关键词“雪山”，共有<b class="hot"> 0 </b>条结果</div>
            <div id="jieqi_page_contents"></div>
            <a href="/sort/1/1.html">玄幻</a>"#;
        let s = score_search_html(html, "雪山", 200);
        assert!(
            s.reasons.iter().any(|r| r == "zero_search_hits"),
            "{:?}",
            s.reasons
        );
        assert!(s.score < 2, "score={}", s.score);
    }

    #[test]
    fn nav_html_not_bookish() {
        let html = r#"<a href="/sort/1/1.html">玄幻</a><a href="/top/allvisit/1.html">榜</a>"#;
        let s = score_search_html(html, "x", 200);
        assert!(!s.reasons.iter().any(|r| r.starts_with("bookish_hrefs")));
    }

    #[test]
    fn search_interval_alert_not_dead() {
        let html = r#"<script>alert("搜索间隔: 30 秒");window.history.go(-1);</script>"#;
        let s = score_search_html(html, "收徒", 200);
        assert!(!s.dead);
        assert!(s.reasons.iter().any(|r| r == "search_rate_limit"));
        assert!(s.score < 0);
    }
}
