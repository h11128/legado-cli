//! Offline probe CLI (forms + rank from HTML file or string).

use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;
use source_probe::{probe_js_engine, probe_search, script_src_candidates};

pub struct ProbeArgs {
    pub base_url: String,
    pub html: Option<String>,
    pub html_file: Option<PathBuf>,
    pub key: String,
    pub js_engine: bool,
}

pub fn run_probe(args: ProbeArgs) -> ExitCode {
    let html = if let Some(f) = args.html_file {
        match std::fs::read_to_string(&f) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("probe: {e}");
                return ExitCode::from(4);
            }
        }
    } else {
        args.html.unwrap_or_default()
    };
    let mut out = json!({
        "probe": probe_search(&html, &args.base_url, &args.key),
    });
    if args.js_engine {
        let mut bodies = Vec::new();
        for src in script_src_candidates(&html) {
            let url = if src.starts_with("http") {
                src.clone()
            } else {
                format!(
                    "{}{}",
                    args.base_url.trim_end_matches('/'),
                    if src.starts_with('/') {
                        src
                    } else {
                        format!("/{src}")
                    }
                )
            };
            if let Ok(resp) = ureq::get(&url).call() {
                if let Ok(body) = resp.into_string() {
                    bodies.push(body);
                }
            }
        }
        out["js_engine"] = json!(probe_js_engine(&args.base_url, &html, &bodies));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    ExitCode::SUCCESS
}
