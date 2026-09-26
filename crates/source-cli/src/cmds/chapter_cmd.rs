//! Chapter utilities: find chapter in novel TOC & inspect chapter content for anti-theft / corruption.

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use regex::Regex;
use serde_json::{json, Value};
use url::Url;

use crate::cli_subs::ChapterSub;

pub fn run_chapter(cmd: ChapterSub) -> ExitCode {
    match cmd {
        ChapterSub::Find {
            url,
            html,
            num,
            title,
            context,
        } => match find_chapter(url.as_deref(), html.as_deref(), num, title.as_deref(), context) {
            Ok(val) => {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("chapter find error: {e}");
                ExitCode::FAILURE
            }
        },
        ChapterSub::Inspect {
            url,
            html,
            out,
            lines,
        } => match inspect_chapter(url.as_deref(), html.as_deref(), out.as_deref(), lines) {
            Ok(val) => {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("chapter inspect error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn fetch_html(url: Option<&str>, file: Option<&Path>) -> Result<(String, String), String> {
    if let Some(f) = file {
        let raw = fs::read_to_string(f)
            .map_err(|e| format!("read file {}: {e}", f.display()))?;
        let text = raw.trim_start_matches('\u{feff}').to_string();
        let base = url.unwrap_or("http://localhost").to_string();
        return Ok((text, base));
    }
    let Some(u) = url else {
        return Err("either --url or --html must be provided".into());
    };
    let resp = ureq::get(u)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(12))
        .call()
        .map_err(|e| format!("http fetch {u}: {e}"))?;

    let content_type = resp.content_type().to_string();
    let mut reader = resp.into_reader();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .map_err(|e| format!("read response body: {e}"))?;

    let mut is_gbk = content_type.to_lowercase().contains("gbk") || content_type.to_lowercase().contains("gb2312");
    if !is_gbk && bytes.len() > 1000 {
        let head = String::from_utf8_lossy(&bytes[..1000.min(bytes.len())]).to_lowercase();
        if head.contains("charset=gbk") || head.contains("charset=\"gbk\"") || head.contains("charset=gb2312") {
            is_gbk = true;
        }
    }
    let text = if is_gbk {
        let (cow, _, _) = encoding_rs::GBK.decode(&bytes);
        cow.to_string()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    let text = text.trim_start_matches('\u{feff}').to_string();
    Ok((text, u.to_string()))
}

#[derive(Clone, Debug)]
pub struct ChapterEntry {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub num: Option<u32>,
}

impl ChapterEntry {
    pub fn to_json(&self) -> Value {
        json!({
            "index": self.index,
            "title": self.title,
            "url": self.url,
            "num": self.num,
        })
    }
}

pub fn parse_toc(html: &str, base_url: &str) -> Vec<ChapterEntry> {
    let re = Regex::new(r#"(?is)<a\s+[^>]*href=["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
    let tag_re = Regex::new(r#"<[^>]+>"#).unwrap();
    let num_re = Regex::new(r#"(?:第|\s|^)(\d+)(?:章|节|回|\s|\.|\))"#).unwrap();
    let base = Url::parse(base_url).ok();

    let mut entries = Vec::new();
    let mut idx = 0;
    for cap in re.captures_iter(html) {
        let raw_href = cap[1].trim();
        let raw_title = tag_re.replace_all(&cap[2], "").trim().to_string();
        if raw_title.is_empty() || raw_href.is_empty() || raw_href.starts_with("javascript:") {
            continue;
        }
        if raw_title.contains("返回")
            || raw_title.contains("首页")
            || raw_title.contains("上一页")
            || raw_title.contains("下一页")
            || raw_title.contains("加入书架")
        {
            continue;
        }
        let is_chapter_like = raw_title.contains('章')
            || raw_title.contains('节')
            || (raw_title.contains('第') && raw_title.contains('回'))
            || num_re.is_match(&raw_title);
        if !is_chapter_like {
            continue;
        }
        let full_url = if let Some(b) = &base {
            b.join(raw_href).map(|u| u.to_string()).unwrap_or_else(|_| raw_href.to_string())
        } else {
            raw_href.to_string()
        };
        let chapter_num = num_re.captures(&raw_title).and_then(|c| c[1].parse::<u32>().ok());
        entries.push(ChapterEntry {
            index: idx,
            title: raw_title,
            url: full_url,
            num: chapter_num,
        });
        idx += 1;
    }
    entries
}

fn find_chapter(
    url: Option<&str>,
    html: Option<&Path>,
    num: Option<u32>,
    title: Option<&str>,
    context: usize,
) -> Result<Value, String> {
    let (content, base_url) = fetch_html(url, html)?;
    let chapters = parse_toc(&content, &base_url);
    if chapters.is_empty() {
        return Ok(json!({
            "found": false,
            "total_chapters": 0,
            "message": "No chapter links parsed from TOC HTML",
        }));
    }

    let match_idx = chapters.iter().position(|c| {
        if let Some(target_num) = num {
            if c.num == Some(target_num) {
                return true;
            }
        }
        if let Some(t) = title {
            if c.title.contains(t) {
                return true;
            }
        }
        false
    });

    let Some(idx) = match_idx else {
        return Ok(json!({
            "found": false,
            "total_chapters": chapters.len(),
            "target_num": num,
            "target_title": title,
            "first_chapter": chapters.first().map(|c| c.to_json()),
            "last_chapter": chapters.last().map(|c| c.to_json()),
            "message": "Target chapter not found in TOC",
        }));
    };

    let target = &chapters[idx];
    let start = idx.saturating_sub(context);
    let end = (idx + context + 1).min(chapters.len());
    let surrounding: Vec<Value> = chapters[start..end].iter().map(|c| c.to_json()).collect();

    Ok(json!({
        "found": true,
        "matched": target.to_json(),
        "index": idx,
        "total_chapters": chapters.len(),
        "context": surrounding,
    }))
}

pub fn extract_content_text(html: &str) -> (String, Vec<String>) {
    let content_re = Regex::new(r#"(?is)<div[^>]+id=["']content["'][^>]*>(.*?)</div>"#).unwrap();
    let class_content_re = Regex::new(r#"(?is)<div[^>]+class=["'][^"']*content[^"']*["'][^>]*>(.*?)</div>"#).unwrap();
    let body_html = if let Some(m) = content_re.captures(html) {
        m[1].to_string()
    } else if let Some(m) = class_content_re.captures(html) {
        m[1].to_string()
    } else {
        html.to_string()
    };

    let script_re = Regex::new(r#"(?is)<script[^>]*>.*?</script>"#).unwrap();
    let style_re = Regex::new(r#"(?is)<style[^>]*>.*?</style>"#).unwrap();
    let comment_re = Regex::new(r#"(?is)<!--.*?-->"#).unwrap();
    let br_re = Regex::new(r#"(?i)<br\s*/?>"#).unwrap();
    let p_re = Regex::new(r#"(?i)</?p[^>]*>"#).unwrap();
    let tag_re = Regex::new(r#"<[^>]+>"#).unwrap();

    let cleaned = script_re.replace_all(&body_html, "");
    let cleaned = style_re.replace_all(&cleaned, "");
    let cleaned = comment_re.replace_all(&cleaned, "");
    let cleaned = br_re.replace_all(&cleaned, "\n");
    let cleaned = p_re.replace_all(&cleaned, "\n");
    let cleaned = tag_re.replace_all(&cleaned, "");

    let unescaped = cleaned
        .replace("&nbsp;", " ")
        .replace("&emsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"");

    let mut lines = Vec::new();
    for l in unescaped.lines() {
        let t = l.trim();
        if !t.is_empty() {
            lines.push(t.to_string());
        }
    }
    let full_text = lines.join("\n");
    (full_text, lines)
}

pub fn detect_anti_theft_smells(text: &str) -> Vec<String> {
    let mut smells = Vec::new();
    // 1. Antonym substitution patterns (typical Qidian VIP encryption)
    let antonym_patterns = [
        "热却期开始",
        "都没了小幅",
        "斩出是到",
        "是禁露出",
        "等我走退",
        "是斯对",
        "有没真正见识",
        "差距是小",
        "大觑这些",
        "多则十年,少则百年",
    ];
    for p in antonym_patterns {
        if text.contains(p) {
            smells.push(format!("antonym_antitheft: found suspicious inverted phrase '{p}'"));
            break;
        }
    }

    // 2. Ads / Watermarks
    let watermark_patterns = [
        "loadAdv(",
        "天天看小说",
        "记住本站域名",
        "防采集",
        "转码失败",
        "本站提供",
    ];
    for w in watermark_patterns {
        if text.contains(w) {
            smells.push(format!("watermark: found watermark phrase '{w}'"));
            break;
        }
    }

    // 3. Truncation
    if text.chars().count() < 300 {
        smells.push(format!("truncated: length only {} chars (<300)", text.chars().count()));
    }

    smells
}

fn inspect_chapter(
    url: Option<&str>,
    html: Option<&Path>,
    out: Option<&Path>,
    lines_limit: usize,
) -> Result<Value, String> {
    let (content, base_url) = fetch_html(url, html)?;
    let title_re = Regex::new(r#"(?is)<title>(.*?)</title>"#).unwrap();
    let title = title_re
        .captures(&content)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    let (full_text, lines) = extract_content_text(&content);
    let smells = detect_anti_theft_smells(&full_text);

    let next_page_re = Regex::new(r#"id=["']next_url["'][^>]*href=["']([^"']+)["']"#).unwrap();
    let next_page = next_page_re.captures(&content).and_then(|c| {
        let raw = c[1].trim();
        if raw.contains('_') && (raw.ends_with(".html") || raw.ends_with(".htm")) {
            Url::parse(&base_url).ok().and_then(|b| b.join(raw).ok()).map(|u| u.to_string())
        } else {
            None
        }
    });

    if let Some(out_path) = out {
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(out_path, &full_text)
            .map_err(|e| format!("write output file {}: {e}", out_path.display()))?;
    }

    let sample: Vec<String> = lines.into_iter().take(lines_limit).collect();

    Ok(json!({
        "title": title,
        "char_count": full_text.chars().count(),
        "line_count": sample.len(),
        "has_antonym_smell": smells.iter().any(|s| s.contains("antonym_antitheft")),
        "has_watermark_smell": smells.iter().any(|s| s.contains("watermark")),
        "is_truncated": smells.iter().any(|s| s.contains("truncated")),
        "has_next_page": next_page.is_some(),
        "next_page_url": next_page,
        "smells": smells,
        "sample": sample,
        "saved_to": out.map(|p| p.display().to_string()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toc_basic() {
        let html = r#"
        <div class="list">
            <a href="/book/1/101.html">第101章 启程</a>
            <a href="/book/1/102.html">第102章 试炼</a>
            <a href="/index.html">返回首页</a>
        </div>
        "#;
        let entries = parse_toc(html, "https://example.com/toc/");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].num, Some(101));
        assert_eq!(entries[0].url, "https://example.com/book/1/101.html");
        assert_eq!(entries[1].num, Some(102));
    }

    #[test]
    fn test_anti_theft_smell_detection() {
        let bad_text = "万界交易画册，热却期开始了。丁源握了握拳，都没了小幅的增长。";
        let smells = detect_anti_theft_smells(bad_text);
        assert!(smells.iter().any(|s| s.contains("antonym_antitheft")));
    }

    #[test]
    fn test_clean_text_extraction() {
        let html = r#"
        <div id="content">
            <script>alert(1);</script>
            <p>第一段文本内容。</p>
            <p>第二段文本内容。<br/>换行文本。</p>
        </div>
        "#;
        let (_, lines) = extract_content_text(html);
        assert_eq!(lines, vec!["第一段文本内容。", "第二段文本内容。", "换行文本。"]);
    }
}
