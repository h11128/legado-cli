//! Post-gate oneshot: prep → diagnose → search plan → spine.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use source_adapters::{AdapterRegistry, RegistryRepairPlugin};
use source_cache::{cooldown_for, note_rate_limit, note_verify, CachePaths};
use source_diagnose::ParseDiagnosePort;
use source_mcp::{DualLedgerPort, FsChannelPort, McpClient, McpSourceRepository, McpVerifyPort};
use source_ports::{Clock, DiagnosePort, HtmlFetchPort};
use source_spine::{run_repair_oneshot, DiagnoseInput, GateInput, PlanOrPlugin, RepairPorts};
use source_types::{
    BookSource, FetchResult, GateResult, HeaderMap, Layer, PortError, SourceKey, Url,
};

use super::repair_outcome::RepairOneOutcome;
use super::search_plan::{build_search_layer_plan, SearchPlanOutcome};

struct UtcClock;
impl Clock for UtcClock {
    fn now_utc(&self) -> chrono::DateTime<Utc> {
        Utc::now()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

struct UreqFetch;
impl HtmlFetchPort for UreqFetch {
    fn fetch(&self, url: &Url, headers: &HeaderMap) -> Result<FetchResult, PortError> {
        let mut req = ureq::get(url.as_str());
        for (k, v) in headers.iter() {
            req = req.set(k, v);
        }
        let resp = req
            .call()
            .map_err(|e| PortError::Transient(format!("fetch {url}: {e}")))?;
        let status = resp.status();
        let mut body = Vec::new();
        resp.into_reader()
            .read_to_end(&mut body)
            .map_err(|e| PortError::Transient(format!("read body: {e}")))?;
        Ok(FetchResult::new(status, url.clone(), body))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_after_gate(
    url: &str,
    client: Arc<McpClient>,
    channel: &FsChannelPort,
    repo: &McpSourceRepository,
    mut source: BookSource,
    source_key: SourceKey,
    gate: GateResult,
    dry_run: bool,
    no_verify: bool,
    html: Option<PathBuf>,
    prefetch: bool,
    key: &str,
    skip_diagnose: bool,
) -> RepairOneOutcome {
    let prep_notes = super::oneshot_prep::prep_source_before_repair(&mut source, repo, dry_run);
    if !prep_notes.is_empty() {
        eprintln!("repair: prep {}", prep_notes.join(","));
    }
    let source_for_diag = source.clone();

    let cache_paths = source_mcp::repo_root().ok().map(CachePaths::from_root);
    let concurrent_rate = source
        .as_value()
        .get("concurrentRate")
        .and_then(|v| v.as_str());
    let cooldown_s = if no_verify {
        0.0
    } else {
        cache_paths
            .as_ref()
            .and_then(|p| cooldown_for(p, url.trim(), concurrent_rate).ok())
            .unwrap_or(0.0)
    };
    if cooldown_s > 0.0 {
        eprintln!("repair: cooldown {cooldown_s:.1}s");
        thread::sleep(Duration::from_secs_f64(cooldown_s));
    }

    let debug_text = if skip_diagnose {
        String::new()
    } else {
        match client.tools_call("debug_source", serde_json::json!({"url": url, "key": key})) {
            Ok(v) => McpClient::extract_text(&v),
            Err(e) => {
                eprintln!("repair: debug_source warn: {e}");
                String::new()
            }
        }
    };
    if !skip_diagnose && !debug_text.is_empty() {
        if let Ok(paths) = source_closeout::CloseoutPaths::from_repo() {
            let stub = serde_json::json!({
                "url": url.trim(),
                "via": "repair_oneshot",
                "debug_chars": debug_text.chars().count(),
            });
            let _ = source_closeout::mark_diagnose_done(&paths, url.trim(), &stub.to_string());
        }
    }

    let mut builder = source_spine::RepairContext::builder(source_key, source)
        .gate(gate.clone())
        .dry_run(dry_run)
        .no_verify(no_verify);

    if let Some(html_path) = &html {
        match std::fs::read(html_path) {
            Ok(body) => match Url::new(url.trim()) {
                Ok(u) => builder = builder.insert_html(u, body),
                Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
            },
            Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
        }
    } else if prefetch {
        match Url::new(url.trim()) {
            Ok(u) => match UreqFetch.fetch(&u, &HeaderMap::new()) {
                Ok(fr) => builder = builder.insert_html(u, fr.body),
                Err(e) => eprintln!("repair: prefetch warn: {e}"),
            },
            Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
        }
    }

    let ctx = builder.build();
    let verify = McpVerifyPort::new(Arc::clone(&client));
    let ledger = match DualLedgerPort::from_defaults() {
        Ok(l) => l,
        Err(e) => return RepairOneOutcome::err(4, &e.to_string()),
    };
    let clock = UtcClock;
    let ports = RepairPorts {
        repo,
        verify: &verify,
        ledger: &ledger,
        channel,
        clock: &clock,
    };
    let reg = AdapterRegistry::with_seed_families();
    let plugin = RegistryRepairPlugin(&reg);
    let diag_port = ParseDiagnosePort;

    let layer_preview = if !skip_diagnose && !debug_text.is_empty() {
        Url::new(url.trim()).ok().map(|u| {
            diag_port
                .diagnose(u, &source_for_diag, &debug_text, None)
                .layer
        })
    } else {
        None
    };

    let mut search_plan = None;
    let mut search_endpoint_dead = false;
    if layer_preview == Some(Layer::Search) {
        let home = ctx.html_text();
        if !home.trim().is_empty() {
            let fam = source_types::SiteFamily::new(source_types::SiteFamily::GENERIC_FORM);
            match build_search_layer_plan(url.trim(), &home, key, fam) {
                SearchPlanOutcome::Plan(p) => {
                    eprintln!(
                        "repair: search-layer plan ops={} rationale={}",
                        p.ops.len(),
                        p.rationale
                    );
                    search_plan = Some(p);
                }
                SearchPlanOutcome::EndpointDead => {
                    search_endpoint_dead = true;
                    eprintln!("repair: search_endpoint_dead → skip");
                }
                SearchPlanOutcome::None => {}
            }
        }
    }

    if search_endpoint_dead {
        return super::search_dead::report_search_endpoint_dead(url.trim(), &ledger, &clock);
    }

    let diagnose = if skip_diagnose || debug_text.is_empty() {
        DiagnoseInput::None
    } else {
        DiagnoseInput::DebugText {
            text: &debug_text,
            fail_msg: None,
            port: &diag_port,
        }
    };

    let plan_or = match search_plan {
        Some(p) => PlanOrPlugin::Plan(p),
        None => PlanOrPlugin::Plugin(&plugin),
    };

    let result = match run_repair_oneshot(
        ctx,
        &ports,
        plan_or,
        GateInput::Injected(gate),
        Some(&reg),
        diagnose,
        None,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("repair: spine: {e}");
            return RepairOneOutcome::err(2, &e.to_string());
        }
    };
    if let (Some(paths), Some(apply)) = (cache_paths.as_ref(), result.apply.as_ref()) {
        if let Some(vr) = &apply.verify {
            let _ = note_verify(
                paths,
                url.trim(),
                vr.success,
                vr.duration_ms.unwrap_or(0),
                cooldown_s,
            );
            if !vr.success && (vr.message.contains("403") || vr.message.contains("429")) {
                let _ = note_rate_limit(paths, url.trim(), (cooldown_s + 5.0).max(20.0));
            }
        }
    }
    RepairOneOutcome::from_spine(result)
}
