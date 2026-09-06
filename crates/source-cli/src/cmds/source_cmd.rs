//! `repair_source.py` umbrella — triage / fetch / verify / log / index / channel.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};
use source_cache::{cooldown_for, note_rate_limit, note_verify, CachePaths};
use source_closeout::{append_index, assert_fixed_allowed, load_check_json};
use source_mcp::{
    channel_status, force_clear_locks, repo_root, FsChannelPort, McpClient, McpEndpoint,
    McpSourceRepository, McpVerifyPort,
};
use source_patch::smell_rules;
use source_ports::{ChannelPort, SourceRepository, VerifyPort};
use source_types::{CheckOpts, SourceKey};

use super::fetch_cmd::{run_fetch, FetchArgs};

pub enum SourceCmd {
    Triage {
        url: String,
        fail_msg: Option<String>,
        out: Option<PathBuf>,
    },
    Fetch(FetchArgs),
    Verify {
        url: String,
        keyword: String,
        timeout_ms: u64,
        auto_cooldown: bool,
        cooldown: f64,
        out: Option<PathBuf>,
    },
    Log {
        url: String,
        name: Option<String>,
        status: String,
        keyword: String,
        root_cause: Option<String>,
        check_json: Option<PathBuf>,
        out: PathBuf,
        index: Option<PathBuf>,
        agent: Option<String>,
    },
    Index {
        from_log: PathBuf,
        index: PathBuf,
    },
    Channel {
        clear_stale: bool,
        force_clear: bool,
    },
}

pub fn run_source(cmd: SourceCmd) -> ExitCode {
    match cmd {
        SourceCmd::Triage { url, fail_msg, out } => run_triage(&url, fail_msg.as_deref(), out),
        SourceCmd::Fetch(args) => run_fetch(args),
        SourceCmd::Verify {
            url,
            keyword,
            timeout_ms,
            auto_cooldown,
            cooldown,
            out,
        } => run_verify(&url, &keyword, timeout_ms, auto_cooldown, cooldown, out),
        SourceCmd::Log {
            url,
            name,
            status,
            keyword,
            root_cause,
            check_json,
            out,
            index,
            agent,
        } => run_log(
            &url,
            name.as_deref(),
            &status,
            &keyword,
            root_cause.as_deref(),
            check_json.as_deref(),
            &out,
            index.as_deref(),
            agent.as_deref(),
        ),
        SourceCmd::Index { from_log, index } => run_index(&from_log, &index),
        SourceCmd::Channel {
            clear_stale,
            force_clear,
        } => {
            if force_clear {
                match repo_root().and_then(|r| force_clear_locks(&r)) {
                    Ok(n) => eprintln!("source channel: force_clear removed {n} lock(s)"),
                    Err(e) => {
                        eprintln!("source channel: force_clear: {e}");
                        return ExitCode::from(1);
                    }
                }
            } else if clear_stale {
                eprintln!("source channel: clear_stale (also runs on plain status)");
            }
            match channel_status() {
                Ok(v) => {
                    println!("{}", v);
                    if v.get("idle") == Some(&json!(true)) {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("source channel: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

fn run_triage(url: &str, fail_msg: Option<&str>, out: Option<PathBuf>) -> ExitCode {
    let ep = match McpEndpoint::load_defaults() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("source triage: {e}");
            return ExitCode::from(4);
        }
    };
    let client = Arc::new(McpClient::new(ep).with_client_name("source_triage"));
    if let Err(e) = client.ensure_session() {
        eprintln!("source triage: {e}");
        return ExitCode::from(2);
    }
    let repo = McpSourceRepository::new(client);
    let source = match repo.get(&SourceKey::new(url)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("source triage: {e}");
            return ExitCode::from(2);
        }
    };
    let fail = fail_msg.unwrap_or("");
    let layer = source_queue::layer_for_fail(fail);
    let v = source.as_value();
    let info = v.get("ruleBookInfo").and_then(|x| x.as_object());
    let report = json!({
        "url": url,
        "name": v.get("bookSourceName"),
        "group": v.get("bookSourceGroup"),
        "fail_msg": fail,
        "layer": layer,
        "action": if layer == "skip" {
            json!("skip")
        } else {
            json!(format!("fix_{layer}"))
        },
        "smells": smell_rules(&source),
        "concurrentRate": v.get("concurrentRate"),
        "tocUrl": info.and_then(|i| i.get("tocUrl")),
    });
    write_optional(out.as_ref(), &report);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    ExitCode::SUCCESS
}

fn run_verify(
    url: &str,
    keyword: &str,
    timeout_ms: u64,
    auto_cooldown: bool,
    cooldown: f64,
    out: Option<PathBuf>,
) -> ExitCode {
    let channel = match FsChannelPort::from_repo() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("source verify: {e}");
            return ExitCode::from(4);
        }
    };
    if let Err(e) = channel.assert_idle_for_repair() {
        eprintln!("source verify: {e}");
        return ExitCode::from(5);
    }
    let _g = match channel.acquire_repair() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("source verify: {e}");
            return ExitCode::from(5);
        }
    };
    let cache_paths = source_mcp::repo_root().ok().map(CachePaths::from_root);
    let cool = if auto_cooldown {
        cache_paths
            .as_ref()
            .and_then(|p| cooldown_for(p, url, None).ok())
            .unwrap_or(cooldown)
            .max(cooldown)
    } else {
        cooldown
    };
    if cool > 0.0 {
        eprintln!("source verify: cooldown {cool:.1}s");
        thread::sleep(Duration::from_secs_f64(cool));
    }
    let ep = match McpEndpoint::load_defaults() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("source verify: {e}");
            return ExitCode::from(4);
        }
    };
    let client = Arc::new(McpClient::new(ep).with_client_name("source_verify"));
    let verify = McpVerifyPort::new(client)
        .with_keyword(keyword)
        .with_timeout_ms(timeout_ms);
    let started = std::time::Instant::now();
    let vr = match verify.check(&SourceKey::new(url), CheckOpts::default()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("source verify: {e}");
            return ExitCode::from(2);
        }
    };
    let out_doc = json!({
        "url": url,
        "keyword": keyword,
        "success": vr.success,
        "message": vr.message,
        "durationMs": vr.duration_ms.unwrap_or_else(|| started.elapsed().as_millis() as u64),
        "cooldown_s": cool,
    });
    if let Some(paths) = cache_paths.as_ref() {
        let _ = note_verify(paths, url, vr.success, vr.duration_ms.unwrap_or(0), cool);
        if !vr.success
            && (vr.message.contains("403")
                || vr.message.contains("429")
                || vr.message.contains("频繁"))
        {
            let _ = note_rate_limit(paths, url, (cool + 5.0).max(20.0));
        }
    }
    write_optional(out.as_ref(), &out_doc);
    println!(
        "{}",
        serde_json::to_string_pretty(&out_doc).unwrap_or_default()
    );
    if vr.success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[allow(clippy::too_many_arguments)]
fn run_log(
    url: &str,
    name: Option<&str>,
    status: &str,
    keyword: &str,
    root_cause: Option<&str>,
    check_json: Option<&Path>,
    out: &Path,
    index: Option<&Path>,
    agent: Option<&str>,
) -> ExitCode {
    let check = check_json.map(load_check_json).transpose();
    let check_val = match check {
        Ok(Some(v)) => Some(v),
        Ok(None) => None,
        Err(e) => {
            eprintln!("source log: {e}");
            return ExitCode::from(4);
        }
    };
    if status == "fixed" {
        if let Err(e) = assert_fixed_allowed(check_val.as_ref()) {
            eprintln!("source log: {e}");
            return ExitCode::from(1);
        }
    }
    let payload = json!({
        "url": url,
        "name": name.unwrap_or(""),
        "status": status,
        "keyword": keyword,
        "root_cause": root_cause.unwrap_or(""),
        "check": check_val,
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "agent": agent.unwrap_or("source-cli"),
    });
    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(
        out,
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    )
    .is_err()
    {
        eprintln!("source log: write {}", out.display());
        return ExitCode::from(1);
    }
    println!("wrote {}", out.display());
    if let Some(idx) = index {
        let entry = json!({
            "status": status,
            "url": url,
            "name": name,
            "evidence": out.to_string_lossy(),
            "agent": agent.unwrap_or("source-cli"),
            "root_cause": root_cause,
        });
        match append_index(idx, &entry) {
            Ok(_) => println!("updated index {}", idx.display()),
            Err(e) => {
                eprintln!("source log index: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

fn run_index(from_log: &Path, index: &Path) -> ExitCode {
    let raw = match fs::read_to_string(from_log) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("source index: {e}");
            return ExitCode::from(4);
        }
    };
    let entry: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("source index json: {e}");
            return ExitCode::from(4);
        }
    };
    let row = json!({
        "status": entry.get("status"),
        "url": entry.get("url"),
        "name": entry.get("name"),
        "evidence": from_log.to_string_lossy(),
        "agent": entry.get("agent"),
        "root_cause": entry.get("root_cause"),
    });
    match append_index(index, &row) {
        Ok(_) => {
            println!("updated {}", index.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("source index: {e}");
            ExitCode::from(1)
        }
    }
}

fn write_optional(path: Option<&PathBuf>, doc: &Value) {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, serde_json::to_string_pretty(doc).unwrap_or_default());
    }
}
