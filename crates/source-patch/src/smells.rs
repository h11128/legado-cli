//! Safe rule-smell fixes (Python `repair_rule_smells.py`).

use regex::Regex;
use serde_json::Value;
use source_types::BookSource;
use std::sync::OnceLock;

fn webview_single_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\{\s*'webView'\s*:\s*(true|false)\s*\}").unwrap())
}

fn webview_double_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)\{\s*"webView"\s*:\s*(true|false)\s*\}"#).unwrap())
}

fn js_baseurl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^@?js:baseUrl$").unwrap())
}

fn class_space_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^(class\.\w+)\s+a(?:\.(\d+))?(@href)$").unwrap())
}

/// Legado URL options must be JSON with double quotes: `{"webView":true}`.
pub fn fix_webview_quotes(text: &str) -> (String, bool) {
    if !text.contains("webView") {
        return (text.to_string(), false);
    }
    let new2 = webview_single_re().replace_all(text, r#"{"webView":$1}"#);
    let new2 = webview_double_re().replace_all(new2.as_ref(), |caps: &regex::Captures| {
        format!(
            r#"{{"webView":{}}}"#,
            caps.get(1)
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_else(|| "true".to_string())
        )
    });
    let changed = new2.as_ref() != text;
    (new2.into_owned(), changed)
}

/// `class.X a@href` → `class.X@tag.a@href`; drop `||@js:baseUrl`.
pub fn fix_bookurl_class_space(book_url: &str) -> (String, bool) {
    if book_url.is_empty() {
        return (book_url.to_string(), false);
    }
    let parts: Vec<&str> = book_url
        .split("||")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let mut cleaned = Vec::new();
    let mut changed = false;
    for p in parts {
        if js_baseurl_re().is_match(p) {
            changed = true;
            continue;
        }
        if let Some(caps) = class_space_re().captures(p) {
            let idx = caps
                .get(2)
                .map(|m| format!(".{}", m.as_str()))
                .unwrap_or_default();
            cleaned.push(format!("{}@tag.a{}{}", &caps[1], idx, &caps[3]));
            changed = true;
            continue;
        }
        cleaned.push(p.to_string());
    }
    if cleaned.is_empty() {
        return (book_url.to_string(), false);
    }
    let new = cleaned.join("||");
    let changed = changed || new != book_url;
    (new, changed)
}

const WEBVIEW_FIELDS: &[(&str, &[&str])] = &[
    ("ruleSearch", &["bookUrl"]),
    ("ruleBookInfo", &["tocUrl", "init"]),
    ("ruleToc", &["chapterUrl", "chapterList"]),
    ("ruleContent", &["nextContentUrl", "content"]),
];

/// 17mb / xinbanzhu: 「查看目录」→ empty `…/index.html`; real catalog is `/html/{dir}/{id}_1/`.
pub const TOC_17MB_STATIC: &str =
    r#"@js:m=baseUrl.match(/(\d+)_(\d+)/);result='/html/'+m[1]+'/'+m[2]+'_1/';"#;

/// True when tocUrl looks like 17mb "查看目录" / index.html shell (not already static `/html/…_1/`).
pub fn is_17mb_empty_index_toc(toc_url: &str) -> bool {
    let t = toc_url.trim();
    if t.is_empty() {
        return false;
    }
    if t.contains("/html/") && t.contains("_1/") {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    t.contains("查看目录")
        || (lower.contains("index.html")
            && (t.contains("text.") || t.contains("@href") || t.contains("##")))
}

/// Rewrite empty-index tocUrl to static `/html/{dir}/{id}_1/` @js.
pub fn fix_17mb_empty_index_toc(toc_url: &str) -> (String, bool) {
    if !is_17mb_empty_index_toc(toc_url) {
        return (toc_url.to_string(), false);
    }
    (TOC_17MB_STATIC.to_string(), true)
}

/// Apply non-destructive learned fixes. Mutates `source`; returns change labels.
pub fn apply_safe_rule_fixes(source: &mut BookSource) -> Vec<String> {
    let mut changes = Vec::new();
    let root = source.as_value_mut();
    for &(rule_key, field_keys) in WEBVIEW_FIELDS {
        let Some(rule) = root.get_mut(rule_key) else {
            continue;
        };
        let Some(obj) = rule.as_object_mut() else {
            continue;
        };
        for fk in field_keys {
            let Some(Value::String(val)) = obj.get(*fk) else {
                continue;
            };
            if !val.contains("webView") {
                continue;
            }
            let (fixed, changed) = fix_webview_quotes(val);
            if changed {
                obj.insert((*fk).to_string(), Value::String(fixed));
                changes.push(format!("webview_quotes:{rule_key}.{fk}"));
            }
        }
    }
    if let Some(Value::String(su)) = root.get("searchUrl") {
        if su.contains("webView") {
            let (fixed, changed) = fix_webview_quotes(su);
            if changed {
                root["searchUrl"] = Value::String(fixed);
                changes.push("webview_quotes:searchUrl".into());
            }
        }
    }
    if let Some(rs) = root.get_mut("ruleSearch").and_then(|v| v.as_object_mut()) {
        if let Some(bu) = rs.get("bookUrl").and_then(|v| v.as_str()) {
            let (fixed_bu, bu_changed) = fix_bookurl_class_space(bu);
            if bu_changed {
                rs.insert("bookUrl".into(), Value::String(fixed_bu));
                changes.push("bookUrl_class_space".into());
            }
        }
    }
    if let Some(info) = root.get_mut("ruleBookInfo").and_then(|v| v.as_object_mut()) {
        if let Some(toc) = info.get("tocUrl").and_then(|v| v.as_str()) {
            let (fixed_toc, toc_changed) = fix_17mb_empty_index_toc(toc);
            if toc_changed {
                info.insert("tocUrl".into(), Value::String(fixed_toc));
                changes.push("17mb_empty_index_tocUrl".into());
            }
        }
    }
    changes
}

/// Static rule smell hints for triage (Python `repair_helpers.smell_rules`).
pub fn smell_rules(source: &BookSource) -> Vec<serde_json::Value> {
    use serde_json::json;

    fn broad_a_href_re() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"a@href##").unwrap())
    }

    fn read_link_re() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"(?i)(在线阅读|开始阅读|全文)").unwrap())
    }

    let mut smells = Vec::new();
    let v = source.as_value();
    let info = v.get("ruleBookInfo").and_then(|x| x.as_object());
    let toc = info
        .and_then(|i| i.get("tocUrl"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let name = info
        .and_then(|i| i.get("name"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if broad_a_href_re().is_match(toc) && !toc.contains("text.") {
        smells.push(json!({
            "field": "ruleBookInfo.tocUrl",
            "issue": "broad_a_href_regex",
            "hint": "May resolve first unmatched link to homepage; narrow selector",
        }));
    }
    if name.contains("||") && name.contains("##") {
        smells.push(json!({
            "field": "ruleBookInfo.name",
            "issue": "fallback_mixed_with_regex",
            "hint": "Do not mix || fallback with ## replace on same field",
        }));
    }
    if read_link_re().is_match(toc) && toc.to_ascii_lowercase().contains("read") {
        smells.push(json!({
            "field": "ruleBookInfo.tocUrl",
            "issue": "maybe_content_not_catalog",
            "hint": "tocUrl may point at content page; clear or retarget catalog",
        }));
    }
    if is_17mb_empty_index_toc(toc) {
        smells.push(json!({
            "field": "ruleBookInfo.tocUrl",
            "issue": "17mb_empty_index_unapproved",
            "hint": "「查看目录」/index.html often empty; use /html/{dir}/{id}_1/; verify key≠我的 if first hit 未经审核",
        }));
    }
    smells
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn webview_single_quotes() {
        let (fixed, changed) = fix_webview_quotes("https://x.com/,{'webView': true}");
        assert!(changed);
        assert!(fixed.contains(r#"{"webView":true}"#));
        assert!(!fixed.contains("'webView'"));
    }

    #[test]
    fn apply_safe_fixes_search_url() {
        let mut src = BookSource::new(json!({
            "bookSourceUrl": "https://ex.com",
            "searchUrl": "/s,{'webView':true}"
        }));
        let ch = apply_safe_rule_fixes(&mut src);
        assert!(ch.iter().any(|c| c.contains("webview_quotes")));
        assert!(src.search_url().unwrap().contains(r#"{"webView":true}"#));
    }

    #[test]
    fn bookurl_class_space() {
        let (fixed, changed) = fix_bookurl_class_space("class.item a@href||@js:baseUrl");
        assert!(changed);
        assert_eq!(fixed, "class.item@tag.a@href");
    }

    #[test]
    fn seventeen_mb_empty_index_toc() {
        assert!(is_17mb_empty_index_toc("text.查看目录@href"));
        assert!(!is_17mb_empty_index_toc(TOC_17MB_STATIC));
        let mut src = BookSource::new(json!({
            "bookSourceUrl": "http://i.xinbanzhu.net/",
            "ruleBookInfo": { "tocUrl": "text.查看目录@href" }
        }));
        let ch = apply_safe_rule_fixes(&mut src);
        assert!(ch.iter().any(|c| c == "17mb_empty_index_tocUrl"));
        assert_eq!(
            src.as_value()["ruleBookInfo"]["tocUrl"].as_str(),
            Some(TOC_17MB_STATIC)
        );
        let smells = smell_rules(&src);
        assert!(
            !smells.iter().any(|s| s["issue"] == "17mb_empty_index_unapproved"),
            "after fix smell should clear"
        );
        let mut dirty = BookSource::new(json!({
            "bookSourceUrl": "http://m.xinbanzhu.net/",
            "ruleBookInfo": { "tocUrl": "text.查看目录@href" }
        }));
        assert!(smell_rules(&dirty)
            .iter()
            .any(|s| s["issue"] == "17mb_empty_index_unapproved"));
        let _ = &mut dirty;
    }
}
