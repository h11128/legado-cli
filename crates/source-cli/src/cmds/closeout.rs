//! Close-out gate CLI — pending / gate / sync-skill / status.

use std::process::ExitCode;

use serde_json::json;
use source_closeout::{
    ensure_ready_for_next, gate_script_fix, gate_trap, pending_closeout, skill_in_sync,
    sync_skill_to_cursor, CloseoutPaths,
};

pub struct CloseoutArgs {
    pub cmd: String,
    pub trap: Option<String>,
    pub skill_fix: bool,
    pub script_fix: String,
}

pub fn run_closeout(args: CloseoutArgs) -> ExitCode {
    let paths = match CloseoutPaths::from_repo() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("closeout: {e}");
            return ExitCode::from(4);
        }
    };

    match args.cmd.as_str() {
        "pending" => match ensure_ready_for_next(&paths) {
            Ok(detail) => {
                let mut payload = detail.extra;
                if let Some(o) = payload.as_object_mut() {
                    o.insert("closeout".into(), json!("ready"));
                }
                println!("{}", payload);
                ExitCode::SUCCESS
            }
            Err(errors) => {
                for e in &errors {
                    eprintln!("close-out BLOCK: {e}");
                }
                let (_, _, detail) = pending_closeout(&paths);
                let mut payload = detail.extra;
                if let Some(o) = payload.as_object_mut() {
                    o.insert("closeout".into(), json!("blocked"));
                }
                println!("{}", payload);
                ExitCode::from(1)
            }
        },
        "gate" => {
            let trap = args.trap.unwrap_or_default();
            let mut errs = Vec::new();
            if let Err(e) = gate_trap(&paths, &trap, args.skill_fix, &[], false) {
                errs.extend(e);
            }
            if let Err(e) = gate_script_fix(&args.script_fix, args.skill_fix) {
                errs.extend(e);
            }
            if errs.is_empty() {
                println!(
                    "{}",
                    json!({
                        "ok": true,
                        "trap": trap,
                        "skill_fix": args.skill_fix,
                        "script_fix": args.script_fix,
                    })
                );
                ExitCode::SUCCESS
            } else {
                for e in errs {
                    eprintln!("close-out gate FAIL: {e}");
                }
                ExitCode::from(1)
            }
        }
        "sync-skill" => match sync_skill_to_cursor(&paths) {
            Ok(msg) => {
                println!("{}", json!({"ok": true, "cursor_skill": msg}));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("sync-skill FAIL: {e}");
                ExitCode::from(1)
            }
        },
        "status" => {
            let (ok, errors, detail) = pending_closeout(&paths);
            let mut out = detail.extra;
            if let Some(obj) = out.as_object_mut() {
                obj.insert("skill_in_sync".into(), json!(skill_in_sync(&paths)));
                obj.insert("errors".into(), json!(errors));
            }
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        other => {
            eprintln!("closeout: unknown subcmd {other}");
            ExitCode::from(2)
        }
    }
}
