//! Domain migrate CLI.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use source_mcp::{
    DualLedgerPort, FsChannelPort, McpClient, McpEndpoint, McpSourceRepository, McpVerifyPort,
};
use source_migrate::migrate_book_source;
use source_ports::{ChannelPort, LedgerPort, SourceRepository, VerifyPort};
use source_types::{CheckOpts, LedgerRow, LedgerStep, SourceKey, Url};

pub struct MigrateArgs {
    pub from_url: String,
    pub to_url: String,
    pub dry_run: bool,
    pub keep_old: bool,
    pub verify: bool,
    pub enable: bool,
    pub out: Option<PathBuf>,
}

pub fn run_migrate(args: MigrateArgs) -> ExitCode {
    let ep = match McpEndpoint::load_defaults() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("migrate: {e}");
            return ExitCode::from(4);
        }
    };
    let client = Arc::new(McpClient::new(ep).with_client_name("source_cli_migrate"));
    if let Err(e) = client.ensure_session() {
        eprintln!("migrate: session: {e}");
        return ExitCode::from(2);
    }
    let channel = match FsChannelPort::from_repo() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("migrate: channel: {e}");
            return ExitCode::from(4);
        }
    };
    if let Err(e) = channel.assert_idle_for_repair() {
        eprintln!("migrate: {e}");
        return ExitCode::from(5);
    }
    let _g = match channel.acquire_repair() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("migrate: acquire: {e}");
            return ExitCode::from(5);
        }
    };
    let repo = McpSourceRepository::new(Arc::clone(&client));
    let src = match repo.get(&SourceKey::new(args.from_url.trim())) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("migrate: get: {e}");
            return ExitCode::from(2);
        }
    };
    let mut migrated = match migrate_book_source(&src, &args.from_url, &args.to_url) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("migrate: rewrite: {e}");
            return ExitCode::from(4);
        }
    };
    if args.enable {
        let mut v = migrated.into_value();
        v["enabled"] = json!(true);
        migrated = source_types::BookSource::new(v);
    }
    if args.dry_run {
        let report = json!({
            "schema_version": "1",
            "capability": "migrate",
            "dry_run": true,
            "from": args.from_url,
            "to": args.to_url,
            "source": migrated.as_value(),
        });
        print_report(&report, args.out.as_ref());
        return ExitCode::SUCCESS;
    }
    if let Err(e) = repo.save(&migrated) {
        eprintln!("migrate: save: {e}");
        return ExitCode::from(2);
    }
    if !args.keep_old {
        let _ = repo.delete(&[SourceKey::new(args.from_url.trim())]);
    }
    // Seal *from* URL so stale serial/RT queues cannot re-pick the old domain.
    seal_migrated_from(&args.from_url, &args.to_url);
    // Refresh phone index so queue ∩ on_phone drops the deleted from_url.
    match source_queue::refresh_phone_index(None) {
        Ok(r) => eprintln!(
            "migrate: refreshed phone index {} (total={})",
            r.path.display(),
            r.total
        ),
        Err(e) => eprintln!("migrate: warn: phone index refresh failed: {e}"),
    }
    let mut report = json!({
        "schema_version": "1",
        "capability": "migrate",
        "mode": "oneshot",
        "url": args.from_url.trim(),
        "status": "migrated",
        "message": "migrated",
        "migrate_to": args.to_url.trim(),
        "verify_ok": null,
    });
    if args.verify {
        thread::sleep(Duration::from_secs(2));
        let verify = McpVerifyPort::new(client);
        match verify.check(&SourceKey::new(args.to_url.trim()), CheckOpts::default()) {
            Ok(vr) => {
                report["verify_ok"] = json!(vr.success);
                report["verify_message"] = json!(vr.message);
                if !vr.success {
                    print_report(&report, args.out.as_ref());
                    return ExitCode::from(1);
                }
            }
            Err(e) => {
                report["verify_error"] = json!(format!("{e}"));
                print_report(&report, args.out.as_ref());
                return ExitCode::from(2);
            }
        }
    }
    print_report(&report, args.out.as_ref());
    ExitCode::SUCCESS
}

fn seal_migrated_from(from_url: &str, to_url: &str) {
    let Ok(ledger) = DualLedgerPort::from_defaults() else {
        eprintln!("migrate: warn: could not open ledger to seal from_url");
        return;
    };
    let Ok(u) = Url::new(from_url.trim()) else {
        return;
    };
    let row = LedgerRow::new(
        Utc::now().to_rfc3339(),
        u,
        LedgerStep::Skip,
        format!("skip:migrated_to:{}", to_url.trim()),
    );
    if let Err(e) = ledger.append(&row) {
        eprintln!("migrate: warn: ledger seal from_url: {e}");
    }
}

fn print_report(report: &serde_json::Value, out: Option<&PathBuf>) {
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            path,
            serde_json::to_string_pretty(report).unwrap_or_default(),
        );
    }
    println!("{}", serde_json::to_string_pretty(report).unwrap_or_default());
}
