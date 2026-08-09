//! HTML deadish / wall / shell sniff — parity with `_sniff_dead_html`.

/// Parking / expired / sale hints (case-insensitive match on lowered blob).
pub const DEADISH_HINTS: &[&str] = &[
    "无法访问此网站",
    "域名到期",
    "域名已过期",
    "域名过期",
    "sitename suspended",
    // Bare nginx/title "404 Not Found" is NOT parked — site may still serve
    // /book/… + search (trap home_404_paths_alive). Leave as L2 http fail → Hunt.
    "this domain",
    "domain expired",
    "domain has expired",
    "expired domain",
    "for sale",
    "buy this domain",
    "hugedomains",
    "godaddy",
    "sedo.com",
    "dan.com",
    "afternic",
    "parked",
    "parking",
    "域名出售",
    "域名买卖",
    "此域名出售",
    "该域名",
    "出售域名",
    // Domain still resolves but is no longer a novel site (product/OEM shell).
    "专业生产厂家",
    "工业通风",
    "请输入您要查询的产品",
    // Empty hosting / panel shells (skill: 没有找到站点 / nginx).
    "没有找到站点",
    "welcome to nginx",
];

/// Soft walls: alive but not repairable without human.
pub const WALL_HINTS: &[&str] = &[
    "请输入密码",
    "输入密码访问",
    "password protected",
    "password required",
    "连接数据库失败",
    "数据库连接失败",
    "urldance.com",
    "safebrowse.io",
    "safebrowsing",
];

/// Bot / JS challenge shells (matched against lowered blob; hints already lower).
pub const SHELL_HINTS: &[&str] = &[
    "redirecting...",
    "<title>redirecting",
    "inte_base64:",
    "challenge-platform",
    "cf-browser-verification",
];

/// Search throttle shells (笔趣阁系 `ss_search_delay` / alert 间隔).
///
/// Not a dead/wall site — do **not** disable. Agent must clear cookies and/or
/// set `enabledCookieJar=false` before rewriting search selectors.
pub const SEARCH_RATE_LIMIT_HINTS: &[&str] = &[
    "搜索间隔",
    "ss_search_delay",
    "search interval",
    "搜索太频繁",
    "请稍后再搜索",
    "请稍后搜索",
];

/// True when HTML is a search-throttle stub (often tiny `<script>alert("搜索间隔…")`).
pub fn sniff_search_rate_limit(text: &str) -> Option<&'static str> {
    let low = text.to_lowercase();
    for h in SEARCH_RATE_LIMIT_HINTS {
        if low.contains(&h.to_lowercase()) {
            return Some(*h);
        }
    }
    None
}

/// Return reason tag if HTML looks like parking / wall / bot-shell.
///
/// Tags: `wall:…` | `deadish:…` | `shell:…` | `deadish:tiny_sale_or_redirect`.
/// Search-throttle stubs (`搜索间隔` / `ss_search_delay`) return **None** — not dead.
pub fn sniff_dead_html(text: &str, final_url: &str, title: &str) -> Option<String> {
    // Throttle HTML must not fall through to tiny_sale / shell heuristics.
    if sniff_search_rate_limit(text).is_some() {
        return None;
    }
    let low = text.to_lowercase();
    let title_l = title.to_lowercase();
    let final_l = final_url.to_lowercase();
    let head: String = low.chars().take(8000).collect();
    let blob = format!("{title_l}\n{final_l}\n{head}");

    for h in WALL_HINTS {
        if blob.contains(&h.to_lowercase()) {
            return Some(format!("wall:{h}"));
        }
    }
    for h in DEADISH_HINTS {
        if blob.contains(&h.to_lowercase()) {
            return Some(format!("deadish:{h}"));
        }
    }
    for h in SHELL_HINTS {
        if blob.contains(h) {
            return Some(format!("shell:{h}"));
        }
    }
    if text.len() < 6000
        && (low.contains("for sale") || text.contains("出售") || title_l == "redirecting...")
    {
        return Some("deadish:tiny_sale_or_redirect".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_password_wall() {
        let html = "<html><title>Gate</title><body>请输入密码后继续</body></html>";
        let r = sniff_dead_html(html, "https://x.example/", "Gate").unwrap();
        assert!(r.starts_with("wall:"), "{r}");
        assert!(r.contains("请输入密码"));
    }

    #[test]
    fn sniff_db_wall() {
        let html = "<html>连接数据库失败，请稍后</html>";
        let r = sniff_dead_html(html, "", "").unwrap();
        assert_eq!(r, "wall:连接数据库失败");
    }

    #[test]
    fn sniff_domain_parked() {
        let html = "<html><body>This domain is for sale at HugeDomains</body></html>";
        let r = sniff_dead_html(html, "https://park.example/", "").unwrap();
        assert!(r.starts_with("deadish:"), "{r}");
    }

    #[test]
    fn sniff_expired_cn() {
        let html = "<title>提示</title><p>域名已过期，请联系管理员</p>";
        let r = sniff_dead_html(html, "", "提示").unwrap();
        assert_eq!(r, "deadish:域名已过期");
    }

    #[test]
    fn sniff_bot_shell() {
        let html = r#"<html><title>Just a moment...</title>
            <script src="https://challenges.cloudflare.com/cdn-cgi/challenge-platform/h/x"></script>"#;
        let r = sniff_dead_html(html, "https://x/", "Just a moment...").unwrap();
        assert_eq!(r, "shell:challenge-platform");
    }

    #[test]
    fn sniff_tiny_sale_or_redirect() {
        let html = "<html><title>x</title>短页 出售</html>";
        assert!(html.len() < 6000);
        let r = sniff_dead_html(html, "", "x").unwrap();
        assert_eq!(r, "deadish:tiny_sale_or_redirect");
    }

    #[test]
    fn sniff_clean_novel_page() {
        let html = format!(
            "<html><title>三体</title><body>{}</body></html>",
            "章节内容".repeat(500)
        );
        assert!(sniff_dead_html(&html, "https://novel.example/book/1", "三体").is_none());
    }

    #[test]
    fn sniff_no_site_shell() {
        let html = "<html><title>没有找到站点</title><body>站点不存在</body></html>";
        let r = sniff_dead_html(html, "http://199.33.126.51/", "没有找到站点").unwrap();
        assert_eq!(r, "deadish:没有找到站点");
    }

    #[test]
    fn sniff_industrial_repurpose() {
        let html = r#"<html><title>安盛风机 - 工业通风设备专业生产厂家</title>
            <body><form><input placeholder="请输入您要查询的产品"/></form></body></html>"#;
        let r = sniff_dead_html(html, "https://www.jinyongwang.net/", "安盛风机").unwrap();
        assert!(r.starts_with("deadish:"), "{r}");
        assert!(r.contains("专业生产厂家") || r.contains("工业通风") || r.contains("查询的产品"));
    }

    #[test]
    fn sniff_search_interval_alert() {
        let html = r#"<script>alert("搜索间隔: 30 秒");window.history.go(-1);</script>"#;
        assert_eq!(sniff_search_rate_limit(html), Some("搜索间隔"));
        assert!(sniff_dead_html(html, "http://www.15u.cc/searchb0.html", "").is_none());
    }

    #[test]
    fn sniff_search_interval_not_deadish_even_with_sale_word() {
        // Tiny page + 出售 would otherwise hit tiny_sale; throttle must win.
        let html = r#"<script>alert("搜索间隔: 30 秒");</script>出售"#;
        assert!(html.len() < 6000);
        assert_eq!(sniff_search_rate_limit(html), Some("搜索间隔"));
        assert!(sniff_dead_html(html, "http://www.15u.cc/s", "").is_none());
    }

    #[test]
    fn sniff_bare_404_title_not_deadish() {
        // home_404_paths_alive: nginx 404 home must not map to parked/Disable.
        let html = "<html><title>404 Not Found</title><body>404 Not Found</body></html>";
        assert!(
            sniff_dead_html(html, "https://www.dbxsn.com/", "404 Not Found").is_none(),
            "bare 404 must not be deadish"
        );
    }
}
