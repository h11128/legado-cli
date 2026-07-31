//! Wave / harvest / bench / deep-wave / search-wave / goal15 orchestration.
//! Serial lives in `serial_cmd` (per-URL subprocess timeout).

use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;
use source_check::{
    default_rules_path, load_urls_file, run_bench10, run_deep_wave, run_harvest, run_search_wave,
    run_wave, BenchOpts, DeepWaveOpts, HarvestOpts, SearchWaveOpts, WaveOpts,
};

use super::repair::{run_repair, RepairArgs};

pub use super::serial_cmd::{run_serial_cmd, SerialArgs};

pub struct WaveArgs {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub thread_count: u32,
    pub patch_workers: usize,
    pub timeout_ms: u64,
    pub check_discovery: bool,
    pub disable_dropped: bool,
    pub out: PathBuf,
    pub rules: Option<PathBuf>,
    pub l2_timeout: f64,
}

pub struct HarvestArgs {
    pub fails: PathBuf,
    pub limit: usize,
    pub keyword: String,
    pub thread_count: u32,
    pub timeout_ms: u64,
    pub out: PathBuf,
}

pub struct BenchArgs {
    pub urls_file: Option<PathBuf>,
    pub keyword: String,
    pub thread_count: u32,
    pub patch_workers: usize,
    pub timeout_ms: u64,
    pub disable_dropped: bool,
    pub out: PathBuf,
    pub rules: Option<PathBuf>,
    pub l2_timeout: f64,
}

pub struct DeepWaveArgs {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub budget_s: f64,
    pub out: PathBuf,
}

pub struct SearchWaveArgs {
    pub urls_file: PathBuf,
    pub keyword: String,
    pub workers: usize,
    pub thread_count: u32,
    pub timeout_ms: u64,
    pub out: PathBuf,
}

pub struct Goal15Args {
    pub urls_file: Option<PathBuf>,
    pub keyword: String,
    pub rules: Option<PathBuf>,
    pub l0_only: bool,
    pub tcp_timeout: f64,
    pub l2_timeout: f64,
    pub out: PathBuf,
}

pub fn run_wave_cmd(args: WaveArgs) -> ExitCode {
    let rules = args.rules.unwrap_or_else(default_rules_path);
    match run_wave(WaveOpts {
        urls_file: args.urls_file,
        keyword: args.keyword,
        thread_count: args.thread_count,
        patch_workers: args.patch_workers,
        timeout_ms: args.timeout_ms,
        check_discovery: args.check_discovery,
        disable_dropped: args.disable_dropped,
        out: args.out,
        rules,
        l2_timeout: args.l2_timeout,
    }) {
        Ok(report) => {
            println!("{}", report);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("wave: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_harvest_cmd(args: HarvestArgs) -> ExitCode {
    match run_harvest(HarvestOpts {
        fails: args.fails,
        limit: args.limit,
        keyword: args.keyword,
        thread_count: args.thread_count,
        timeout_ms: args.timeout_ms,
        out: args.out,
    }) {
        Ok(report) => {
            println!("{}", report);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("harvest: {e}");
            if e.to_string().contains("no urls") {
                ExitCode::from(2)
            } else {
                ExitCode::from(1)
            }
        }
    }
}

pub fn run_bench_cmd(args: BenchArgs) -> ExitCode {
    let urls = args
        .urls_file
        .as_ref()
        .and_then(|p| load_urls_file(p).ok())
        .unwrap_or_default();
    let rules = args.rules.unwrap_or_else(default_rules_path);
    match run_bench10(BenchOpts {
        urls,
        keyword: args.keyword,
        thread_count: args.thread_count,
        patch_workers: args.patch_workers,
        timeout_ms: args.timeout_ms,
        disable_dropped: args.disable_dropped,
        out: args.out,
        rules,
        l2_timeout: args.l2_timeout,
    }) {
        Ok(report) => {
            println!("{}", report);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bench: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_deep_wave_cmd(args: DeepWaveArgs) -> ExitCode {
    match run_deep_wave(DeepWaveOpts {
        urls_file: args.urls_file,
        keyword: args.keyword,
        budget_s: args.budget_s,
        out: args.out,
    }) {
        Ok(report) => {
            println!("{}", report);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("deep-wave: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_search_wave_cmd(args: SearchWaveArgs) -> ExitCode {
    match run_search_wave(SearchWaveOpts {
        urls_file: args.urls_file,
        keyword: args.keyword,
        workers: args.workers,
        thread_count: args.thread_count,
        timeout_ms: args.timeout_ms,
        out: args.out,
    }) {
        Ok(report) => {
            println!("{}", report);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("search-wave: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run_goal15_cmd(args: Goal15Args) -> ExitCode {
    let queue = args.urls_file.or_else(|| {
        let p = PathBuf::from("temp/full_fix/goal15_queue.json");
        if p.is_file() {
            Some(p)
        } else {
            None
        }
    });
    let code = run_repair(RepairArgs {
        url: String::new(),
        urls_file: queue,
        mode: "batch".into(),
        limit: 15,
        rules: args.rules,
        l0_only: args.l0_only,
        tcp_timeout: args.tcp_timeout,
        l2_timeout: args.l2_timeout,
        dry_run: false,
        no_verify: false,
        html: None,
        prefetch: true,
        key: args.keyword.clone(),
        skip_diagnose: false,
    });
    if let Some(out) = args.out.parent() {
        let _ = std::fs::create_dir_all(out);
    }
    let _ = std::fs::write(
        &args.out,
        json!({"goal15": true, "exit": if code == ExitCode::SUCCESS { 0 } else { 1 }}).to_string(),
    );
    code
}
