//! Scaffold a 笔趣阁-family BookSource draft from a known-good template (15u).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::json;

/// Minimal MVP: GET search + CookieJar off + longest-list TOC JS + content-body pattern.
/// Agents must still fetch real HTML and rewrite selectors before claiming success.
pub fn run_scaffold(host_url: &str, name: Option<&str>, out: &Path) -> ExitCode {
    let base = match normalize_base(host_url) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("scaffold: {e}");
            return ExitCode::from(1);
        }
    };
    let host = match host_of(&base) {
        Some(h) => h,
        None => {
            eprintln!("scaffold: cannot parse host from {base}");
            return ExitCode::from(1);
        }
    };
    let display = name
        .map(str::to_string)
        .unwrap_or_else(|| format!("笔趣阁系-{host}"));
    let host_esc = regex_escape_host(&host);
    let draft = json!({
        "bookSourceName": display,
        "bookSourceUrl": base,
        "bookSourceGroup": "小说",
        "bookSourceType": 0,
        "bookSourceComment": "scaffold MVP 笔趣阁系 — REWRITE selectors from live HTML; \
    GET search; enabledCookieJar=false; clear-cookies on 搜索间隔; TOC pick longest list",
        "enabled": true,
        "enabledExplore": false,
        "enabledCookieJar": false,
        "concurrentRate": "2000",
        "header": format!(
            "{{\"User-Agent\": \"Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36\", \"Referer\": \"{base}/\"}}"
        ),
        "bookUrlPattern": format!("https?://(www\\.)?{host_esc}/"),
        "searchUrl": format!(
            "@js:\ncookie.removeCookie('{base}');\n'/search.php?q='+encodeURIComponent(key)"
        ),
        "ruleSearch": {
            "bookList": ".result-item, .bookbox, #hotcontent .item, id.bookcon@tag.tr!0",
            "name": "tag.a.0@text||h3@text||.s2@a@text",
            "author": ".author@text||tag.td.2@text||.s4@text",
            "bookUrl": "tag.a.0@href||h3@a@href||.s2@a@href",
            "checkKeyWord": "我的"
        },
        "ruleBookInfo": {
            "name": "h1@text||#info h1@text",
            "author": "#info p.0@text||.author@text||h3@tag.a@text",
            "intro": "#intro@text||.intro@text||tag.p.0@text"
        },
        "ruleToc": {
            "chapterList": "@js:\nvar doc=org.jsoup.Jsoup.parse(result);\nvar cands=doc.select(\
    'ul.list-group.list-charts, #list dl, .listmain dl, .chapter-list, #chapterlist');\
    nvar best=null,max=0;\nfor(var i=0;i<cands.size();i++){\n var n=cands.get(i).select('a').size();\n \
    if(n>max){max=n;best=cands.get(i);}\n}\nbest?best.select('a'):doc.select('#list a, .chapter-list a');\n",
            "chapterName": "text",
            "chapterUrl": "href"
        },
        "ruleContent": {
            "content": "#content@html||#chaptercontent@html||.content-body@html||#BookText@html",
            "nextContentUrl": "id.next_url@href||a:contains(下一页)@href||#next_url@href",
            "replaceRegex": "##请收藏本站.*|本章未完.*|笔趣阁.*"
        }
    });

    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(
        out,
        serde_json::to_string_pretty(&draft).unwrap_or_default(),
    ) {
        Ok(()) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&draft).unwrap_or_default()
            );
            eprintln!(
                "scaffold: wrote {} — fetch HTML, fix selectors, then:\n\
                 source-cli source push --file {}\n\
                 (MVP only; device 校验成功 required before claim created)",
                out.display(),
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("scaffold: write {}: {e}", out.display());
            ExitCode::from(1)
        }
    }
}

fn normalize_base(raw: &str) -> Result<String, String> {
    let t = raw.trim().trim_end_matches('/');
    if t.is_empty() {
        return Err("empty url".into());
    }
    let with_scheme = if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("http://{t}")
    };
    let u = url::Url::parse(&with_scheme).map_err(|e| e.to_string())?;
    let host = u.host_str().ok_or_else(|| "no host".to_string())?;
    Ok(format!("{}://{}", u.scheme(), host))
}

fn host_of(base: &str) -> Option<String> {
    url::Url::parse(base)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .map(|h| h.strip_prefix("www.").unwrap_or(&h).to_string())
}

fn regex_escape_host(host: &str) -> String {
    host.replace('.', "\\.")
}

/// Default out path under cache/new_sources.
pub fn default_out_for(host_url: &str) -> PathBuf {
    let host = normalize_base(host_url)
        .ok()
        .and_then(|b| host_of(&b))
        .unwrap_or_else(|| "site".into())
        .replace('.', "_");
    PathBuf::from(format!(
        "temp/full_fix/cache/new_sources/{host}_scaffold.json"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn normalize_adds_scheme() {
        assert_eq!(normalize_base("www.15u.cc").unwrap(), "http://www.15u.cc");
    }

    #[test]
    fn draft_has_cookie_jar_off() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("x.json");
        assert!(run_scaffold("http://www.example.com", None, &out) == ExitCode::SUCCESS);
        let v: Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(v["enabledCookieJar"], false);
        assert!(v["searchUrl"].as_str().unwrap().contains("removeCookie"));
        let pat = v["bookUrlPattern"].as_str().unwrap();
        // JSON string should contain a single-backslash regex escape for dots.
        assert!(pat.contains(r"(www\.)?example\.com"), "pat={pat}");
    }
}
