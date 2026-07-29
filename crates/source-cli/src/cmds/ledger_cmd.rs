//! Ledger append/show CLI.

use std::process::ExitCode;

use source_closeout::gate_ledger_result;
use source_mcp::{default_jsonl_path, DualLedgerPort};
use source_ports::LedgerPort;
use source_types::{LedgerRow, LedgerStep, Url};

pub enum LedgerCmd {
    Append {
        url: String,
        step: String,
        result: String,
        note: Option<String>,
        waste: Option<String>,
    },
    Show {
        limit: usize,
    },
}

pub fn run_ledger(cmd: LedgerCmd) -> ExitCode {
    let port = match DualLedgerPort::from_defaults() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ledger: {e}");
            return ExitCode::from(4);
        }
    };
    match cmd {
        LedgerCmd::Append {
            url,
            step,
            result,
            note,
            waste,
        } => {
            if let Err(msg) = gate_ledger_result(&result) {
                eprintln!("ledger BLOCK: {msg}");
                return ExitCode::from(1);
            }
            let u = match Url::new(url.trim()) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("ledger: bad url: {e}");
                    return ExitCode::from(4);
                }
            };
            let step = match step.as_str() {
                "gate" => LedgerStep::Gate,
                "diagnose" => LedgerStep::Diagnose,
                "patch" | "apply" => LedgerStep::Apply,
                "check" | "verify" => LedgerStep::Check,
                "skip" => LedgerStep::Skip,
                "migrate" => LedgerStep::Migrate,
                "hunt" => LedgerStep::Hunt,
                _ => LedgerStep::Check,
            };
            let ts = chrono::Utc::now().to_rfc3339();
            let mut row = LedgerRow::new(ts, u, step, result);
            row.note = note;
            row.waste = waste;
            if let Err(e) = port.append(&row) {
                eprintln!("ledger: append: {e}");
                return ExitCode::from(2);
            }
            // Heartbeat deep_active if this URL is claimed.
            if let Ok(paths) = source_closeout::CloseoutPaths::from_repo() {
                let _ = source_closeout::heartbeat_active(&paths);
            }
            println!("{}", serde_json::to_string(&row).unwrap_or_default());
            ExitCode::SUCCESS
        }
        LedgerCmd::Show { limit } => {
            let path = match default_jsonl_path() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("ledger: {e}");
                    return ExitCode::from(4);
                }
            };
            let raw = std::fs::read_to_string(&path).unwrap_or_default();
            let lines: Vec<_> = raw.lines().rev().take(limit).collect();
            for line in lines.into_iter().rev() {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
    }
}
