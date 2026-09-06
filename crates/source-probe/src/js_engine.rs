//! Unified JS search shell probe — parity with `debugger/js_engine`.

use serde::{Deserialize, Serialize};

use crate::forms_js::{forms_from_js, searchish_script_srcs};
use crate::js_api::detect_js_search_api;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsEngineHit {
    pub kind: String,
    pub search_url: String,
    pub detail: String,
}

/// Probe homepage HTML (+ optional fetched script bodies) for JS/search shells.
pub fn probe_js_engine(
    base_url: &str,
    home_html: &str,
    script_bodies: &[String],
) -> Vec<JsEngineHit> {
    let mut hits = Vec::new();
    if let Some(j) = detect_js_search_api(home_html, base_url) {
        hits.push(JsEngineHit {
            kind: "data_api".into(),
            search_url: j.search_url,
            detail: j.api_path,
        });
    }
    for form in forms_from_js(home_html, base_url) {
        hits.push(JsEngineHit {
            kind: "form_js".into(),
            search_url: form.action,
            detail: format!("{} {}", form.method, form.fields),
        });
    }
    for body in script_bodies {
        for form in forms_from_js(body, base_url) {
            hits.push(JsEngineHit {
                kind: "script_form".into(),
                search_url: form.action,
                detail: format!("{} {}", form.method, form.fields),
            });
        }
        if let Some(j) = detect_js_search_api(body, base_url) {
            hits.push(JsEngineHit {
                kind: "script_data_api".into(),
                search_url: j.search_url,
                detail: j.api_path,
            });
        }
    }
    hits.dedup_by(|a, b| a.search_url == b.search_url && a.kind == b.kind);
    hits
}

/// Script `src` paths worth fetching for embedded search forms.
pub fn script_src_candidates(html: &str) -> Vec<String> {
    searchish_script_srcs(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_data_api_and_js_form() {
        let html = r#"<div data-api="/api/search"></div><script>action='/s.php'</script>"#;
        let hits = probe_js_engine("https://ex.com/", html, &[]);
        assert!(!hits.is_empty());
    }
}
