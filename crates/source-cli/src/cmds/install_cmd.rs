//! `source-cli install` — cargo install this crate onto the user cargo bin PATH.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

pub fn run_install(force: bool) -> ExitCode {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    if !manifest.is_file() {
        eprintln!("install: missing {}", manifest.display());
        return ExitCode::from(1);
    }
    let mut cmd = Command::new("cargo");
    cmd.arg("install").arg("--path").arg(manifest.parent().unwrap());
    if force {
        cmd.arg("--force");
    }
    // Never invent a second target dir (host_resource / cargo_single_target_dir).
    cmd.env_remove("CARGO_TARGET_DIR");
    eprintln!(
        "install: running `cargo install --path {}{}` …",
        manifest.parent().unwrap().display(),
        if force { " --force" } else { "" }
    );
    match cmd.status() {
        Ok(st) if st.success() => {
            eprintln!(
                "install: ok — open a new shell and run `source-cli --help`.\n\
                 If still not found: ensure `%USERPROFILE%\\\\.cargo\\\\bin` (or ~/.cargo/bin) is on PATH."
            );
            ExitCode::SUCCESS
        }
        Ok(st) => {
            eprintln!("install: cargo exited {st}");
            ExitCode::from(st.code().unwrap_or(1) as u8)
        }
        Err(e) => {
            eprintln!("install: spawn cargo: {e}");
            ExitCode::from(1)
        }
    }
}
