//! CLI subcommands contract & smoke tests.
//!
//! Automatically verifies that all subcommands and parameters advertised
//! in documentation (README, guides, skills) exist and behave according to contract.

use std::process::Command;

fn get_source_cli_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("failed to get test exe path");
    path.pop(); // pop test exe name
    path.pop(); // pop deps
    path.push("source-cli.exe");
    if !path.exists() {
        path.set_extension("");
    }
    path
}

fn run_cmd(args: &[&str]) -> (bool, String, String) {
    let bin = get_source_cli_bin();
    let output = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute source-cli with args {:?}: {}", args, e));

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_root_help() {
    let (success, stdout, _) = run_cmd(&["--help"]);
    assert!(success, "source-cli --help should succeed");
    assert!(stdout.contains("LegadoSkill repair platform CLI"));
    assert!(stdout.contains("diagnose"));
    assert!(stdout.contains("repair"));
    assert!(stdout.contains("gate"));
    assert!(stdout.contains("site-probe"));
    assert!(stdout.contains("dig"));
    assert!(stdout.contains("mcp"));
    assert!(stdout.contains("hunt"));
    assert!(stdout.contains("migrate"));
    assert!(stdout.contains("wave"));
    assert!(stdout.contains("search-wave"));
    assert!(stdout.contains("serial"));
    assert!(stdout.contains("check"));
    assert!(stdout.contains("ledger"));
    assert!(stdout.contains("retro"));
    assert!(stdout.contains("closeout"));
    assert!(stdout.contains("source"));
}

#[test]
fn test_core_subcommands_help() {
    let subcommands = [
        "gate",
        "repair",
        "diagnose",
        "dig",
        "site-probe",
        "hunt",
        "migrate",
        "wave",
        "search-wave",
        "serial",
        "ledger",
        "retro",
        "closeout",
        "source",
        "check",
        "mcp",
        "cache",
        "ewma",
        "parse",
    ];

    for sub in subcommands {
        let (success, stdout, stderr) = run_cmd(&[sub, "--help"]);
        assert!(
            success,
            "source-cli {} --help failed: stdout={}, stderr={}",
            sub, stdout, stderr
        );
    }
}

#[test]
fn test_nested_subcommands_help() {
    let nested_cases = [
        vec!["source", "scaffold", "--help"],
        vec!["source", "push", "--help"],
        vec!["source", "triage", "--help"],
        vec!["source", "verify", "--help"],
        vec!["source", "get", "--help"],
        vec!["source", "list", "--help"],
        vec!["source", "delete", "--help"],
        vec!["check", "channel", "--help"],
        vec!["check", "clear-cookies", "--help"],
        vec!["check", "precheck", "--help"],
        vec!["check", "batch", "--help"],
        vec!["check", "full", "--help"],
        vec!["ledger", "append", "--help"],
        vec!["ledger", "show", "--help"],
        vec!["retro", "append", "--help"],
        vec!["closeout", "pending", "--help"],
        vec!["closeout", "gate", "--help"],
        vec!["mcp", "list", "--help"],
        vec!["mcp", "probe", "--help"],
        vec!["mcp", "add", "--help"],
        vec!["mcp", "switch", "--help"],
        vec!["cache", "cooldown", "--help"],
    ];

    for args in nested_cases {
        let (success, stdout, stderr) = run_cmd(&args);
        assert!(
            success,
            "source-cli {:?} failed: stdout={}, stderr={}",
            args, stdout, stderr
        );
    }
}

#[test]
fn test_readme_command_arguments_validation() {
    let (_, stdout, _) = run_cmd(&["check", "clear-cookies", "--help"]);
    assert!(
        stdout.contains("--url <URL>"),
        "clear-cookies must have --url option"
    );

    let (_, stdout, _) = run_cmd(&["check", "channel", "--help"]);
    assert!(
        stdout.contains("--force-clear"),
        "check channel must have --force-clear"
    );
    assert!(
        stdout.contains("--clear-stale"),
        "check channel must have --clear-stale"
    );

    let (_, stdout, _) = run_cmd(&["source", "scaffold", "--help"]);
    assert!(
        stdout.contains("--url <URL>"),
        "source scaffold must have --url"
    );
    assert!(
        stdout.contains("--name <NAME>"),
        "source scaffold must have --name"
    );

    let (_, stdout, _) = run_cmd(&["source", "push", "--help"]);
    assert!(
        stdout.contains("--file <FILE>"),
        "source push must have --file"
    );

    let (_, stdout, _) = run_cmd(&["migrate", "--help"]);
    assert!(
        stdout.contains("--from-url <FROM_URL>"),
        "migrate must have --from-url"
    );
    assert!(
        stdout.contains("--to-url <TO_URL>"),
        "migrate must have --to-url"
    );

    let (_, stdout, _) = run_cmd(&["retro", "append", "--help"]);
    assert!(
        stdout.contains("--url <URL>"),
        "retro append must have --url"
    );
    assert!(
        stdout.contains("--status <STATUS>"),
        "retro append must have --status"
    );
    assert!(
        stdout.contains("--trap <TRAP>"),
        "retro append must have --trap"
    );

    let (_, stdout, _) = run_cmd(&["ledger", "append", "--help"]);
    assert!(
        stdout.contains("--url <URL>"),
        "ledger append must have --url"
    );
    assert!(
        stdout.contains("--step <STEP>"),
        "ledger append must have --step"
    );
    assert!(
        stdout.contains("--result <RESULT>"),
        "ledger append must have --result"
    );

    let (_, stdout, _) = run_cmd(&["wave", "--help"]);
    assert!(
        stdout.contains("--urls-file <URLS_FILE>"),
        "wave must have --urls-file"
    );
    assert!(
        stdout.contains("--thread-count <THREAD_COUNT>"),
        "wave must have --thread-count"
    );

    let (_, stdout, _) = run_cmd(&["serial", "--help"]);
    assert!(
        stdout.contains("--urls-file <URLS_FILE>"),
        "serial must have --urls-file"
    );
    assert!(
        stdout.contains("--url-timeout-s <URL_TIMEOUT_S>"),
        "serial must have --url-timeout-s"
    );
}
