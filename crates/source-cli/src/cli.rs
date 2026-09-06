//! Clap CLI definition (kept out of main.rs for the 300-line limit).

use crate::cli_subs::{
    ClaimSub, CloseoutSub, LedgerSub, McpSub, ParseSub, PatternSub, ProgressSub, RetroSub, SourceSub,
};
use crate::ops_subs::{CacheSub, CheckSub, DbSub, KnowledgeSub, QueueSub};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "source-cli",
    about = "LegadoSkill repair platform CLI",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    Gate {
        #[arg(long)]
        url: String,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
    },
    RepairDry {
        #[arg(long)]
        url: String,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    Repair {
        #[arg(long, default_value = "")]
        url: String,
        #[arg(long)]
        urls_file: Option<PathBuf>,
        #[arg(long, default_value = "oneshot", value_parser = ["oneshot", "batch"])]
        mode: String,
        #[arg(long, default_value_t = 15)]
        limit: usize,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        no_verify: bool,
        #[arg(long)]
        html: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        no_prefetch: bool,
        #[arg(long, default_value = "我的")]
        key: String,
        #[arg(long, default_value_t = false)]
        skip_diagnose: bool,
    },
    Diagnose {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "我的")]
        key: String,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long)]
        debug_file: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Official deep-dig entry: channel → gate → diagnose → oneshot (unless --no-repair).
    Dig {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "我的")]
        key: String,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long, default_value_t = false)]
        no_repair: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        no_verify: bool,
    },
    Fetch {
        #[arg(long)]
        url: String,
        #[arg(long)]
        page: Option<String>,
        #[arg(long, default_value = "temp/full_fix/cache/html")]
        dump_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Probe {
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        html: Option<String>,
        #[arg(long)]
        html_file: Option<PathBuf>,
        #[arg(long, default_value = "我的")]
        key: String,
        #[arg(long, default_value_t = false)]
        js_engine: bool,
    },
    Migrate {
        #[arg(long = "from-url")]
        from_url: String,
        #[arg(long = "to-url")]
        to_url: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        keep_old: bool,
        #[arg(long, default_value_t = false)]
        verify: bool,
        #[arg(long, default_value_t = false)]
        enable: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Hunt {
        #[arg(long)]
        url: String,
        #[arg(long)]
        seeds: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        probe: bool,
        #[arg(long, default_value_t = 5.0)]
        l2_timeout: f64,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Batch L0–L2 reachability for discovery seeds (`--preset publish`).
    SiteProbe {
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        urls_file: Option<PathBuf>,
        #[arg(long)]
        url: Vec<String>,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        include_sourced: bool,
    },
    Progress {
        #[command(subcommand)]
        cmd: ProgressSub,
    },
    Ledger {
        #[command(subcommand)]
        cmd: LedgerSub,
    },
    Ewma {
        #[arg(long, default_value_t = 3.0)]
        prev: f64,
        #[arg(long, default_value_t = 20.0)]
        suggested: f64,
    },
    ProbeScore {
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 200)]
        status: u16,
        #[arg(long)]
        html: Option<String>,
    },
    VideoRoute {
        #[arg(long)]
        url: String,
        #[arg(long)]
        routes: Option<PathBuf>,
    },
    Closeout {
        #[command(subcommand)]
        cmd: CloseoutSub,
    },
    Retro {
        #[command(subcommand)]
        cmd: RetroSub,
    },
    Discover {
        #[arg(long, default_value_t = false)]
        write: bool,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        sync_cursor: bool,
    },
    Check {
        #[command(subcommand)]
        cmd: CheckSub,
    },
    Queue {
        #[command(subcommand)]
        cmd: QueueSub,
    },
    Pattern {
        #[command(subcommand)]
        cmd: PatternSub,
    },
    Db {
        #[command(subcommand)]
        cmd: DbSub,
    },
    Cache {
        #[command(subcommand)]
        cmd: CacheSub,
    },
    Knowledge {
        #[command(subcommand)]
        cmd: KnowledgeSub,
    },
    Parse {
        #[command(subcommand)]
        cmd: ParseSub,
    },
    Parity {
        #[arg(long)]
        suite: Option<String>,
    },
    Wave {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 8)]
        thread_count: u32,
        #[arg(long, default_value_t = 4)]
        patch_workers: usize,
        #[arg(long, default_value_t = 45_000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = false)]
        check_discovery: bool,
        #[arg(long, default_value_t = false)]
        disable_dropped: bool,
        #[arg(long, default_value = "temp/full_fix/wave_report.json")]
        out: PathBuf,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
    },
    Harvest {
        #[arg(long)]
        fails: Option<PathBuf>,
        #[arg(long, default_value_t = 16)]
        limit: usize,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 6)]
        thread_count: u32,
        #[arg(long, default_value_t = 18_000)]
        timeout_ms: u64,
        #[arg(long, default_value = "temp/full_fix/harvest_last.json")]
        out: PathBuf,
    },
    Serial {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        rebuild_queue: bool,
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-auto-retro", action = clap::ArgAction::SetFalse)]
        auto_retro: bool,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Kill hung oneshot child and continue (seconds). 0 = no kill (in-process).
        #[arg(long, default_value_t = 120.0)]
        url_timeout_s: f64,
        /// Run oneshot in-process (no subprocess kill). Prefer default subprocess.
        #[arg(long, default_value_t = false)]
        in_process: bool,
    },
    Bench {
        #[arg(long)]
        urls_file: Option<PathBuf>,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 8)]
        thread_count: u32,
        #[arg(long, default_value_t = 4)]
        patch_workers: usize,
        #[arg(long, default_value_t = 45_000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = false)]
        disable_dropped: bool,
        #[arg(long, default_value = "temp/full_fix/bench10_last.json")]
        out: PathBuf,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
    },
    DeepWave {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 45.0)]
        budget_s: f64,
        #[arg(long, default_value = "temp/full_fix/deep_wave_last.json")]
        out: PathBuf,
    },
    SearchWave {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 4)]
        workers: usize,
        #[arg(long, default_value_t = 8)]
        thread_count: u32,
        #[arg(long, default_value_t = 45_000)]
        timeout_ms: u64,
        #[arg(long, default_value = "temp/full_fix/search_wave_last.json")]
        out: PathBuf,
    },
    Goal15 {
        #[arg(long)]
        urls_file: Option<PathBuf>,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long, default_value_t = 1.5)]
        tcp_timeout: f64,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long, default_value = "temp/full_fix/goal15_results.json")]
        out: PathBuf,
    },
    Source {
        #[command(subcommand)]
        cmd: SourceSub,
    },
    Claim {
        #[command(subcommand)]
        cmd: ClaimSub,
    },
    DebugVsCheck {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "我的")]
        key: String,
        #[arg(long, default_value = "temp/full_fix/debug_vs_check.json")]
        out: PathBuf,
        #[arg(long, default_value_t = false)]
        no_ledger: bool,
    },
    /// Install `source-cli` onto `~/.cargo/bin` (PATH) via `cargo install --path`.
    Install {
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-force", action = clap::ArgAction::SetFalse)]
        force: bool,
    },
    /// Manage remembered MCP endpoints (list, add, switch, probe, remove).
    Mcp {
        #[command(subcommand)]
        cmd: McpSub,
    },
    Version,
}
