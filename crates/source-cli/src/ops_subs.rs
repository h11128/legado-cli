//! Clap enums for db / cache / queue / check / knowledge ops (keeps cli_subs short).

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum DbSub {
    Migrate,
    Status,
    ImportLedger {
        #[arg(long, default_value = "temp/full_fix/repair_session_ledger.jsonl")]
        path: PathBuf,
    },
    ImportHtmlCache {
        #[arg(long = "dir", default_value = "temp/full_fix/cache/html")]
        dir: PathBuf,
    },
    ImportHostStats {
        #[arg(long, default_value = "temp/full_fix/cache/host_stats.json")]
        path: PathBuf,
    },
    ImportCache,
    ExportPhoneIndex {
        #[arg(long, default_value = "temp/full_fix/phone_source_index.json")]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum CacheSub {
    GetHtml {
        #[arg(long)]
        url: String,
        #[arg(long, default_value_t = 3600.0)]
        max_age: f64,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    PutHtml {
        #[arg(long)]
        url: String,
        #[arg(long)]
        body_file: PathBuf,
        #[arg(long)]
        meta_file: Option<PathBuf>,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    Cooldown {
        #[arg(long)]
        url: String,
        #[arg(long)]
        concurrent_rate: Option<String>,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    NoteRateLimit {
        #[arg(long)]
        url: String,
        #[arg(long, default_value_t = 20.0)]
        suggested_gap: f64,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    NoteVerify {
        #[arg(long)]
        url: String,
        #[arg(long, default_value_t = true)]
        success: bool,
        #[arg(long, default_value_t = 0)]
        duration_ms: u64,
        #[arg(long, default_value_t = 3.0)]
        used_cooldown: f64,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    GetTriage {
        #[arg(long)]
        url: String,
        #[arg(long, default_value_t = 1800.0)]
        max_age: f64,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
    PutTriage {
        #[arg(long)]
        url: String,
        #[arg(long)]
        report_file: PathBuf,
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum QueueSub {
    RefreshIndex {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Rt {
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "搜索失效")]
        group: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = 8000)]
        max_rt_ms: i64,
        #[arg(long, default_value_t = false)]
        full: bool,
        #[arg(long)]
        all_sources: Option<PathBuf>,
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Cluster remain URLs by full BookSource rules (pulls get_source by default).
    Cluster {
        #[arg(long)]
        queue: Option<PathBuf>,
        #[arg(long)]
        sources_file: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value_t = 3)]
        min_size: u32,
        #[arg(long)]
        out: Option<PathBuf>,
        /// If a URL has no full BookSource in DB/file yet, pull via MCP get_source (default true).
        /// Already-cached snapshots are reused; use --no-from-mcp to never call the phone.
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-from-mcp", action = clap::ArgAction::SetFalse)]
        from_mcp: bool,
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
    /// Build prioritized fail queue from check JSON/JSONL.
    Build {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "temp/full_fix/repair_queue.json")]
        out: PathBuf,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Decide fail_msg layer/action, or classify a resolved URL kind.
    Classify {
        #[arg(long)]
        fail_msg: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        html: Option<String>,
        #[arg(long)]
        html_file: Option<PathBuf>,
    },
    /// Annotate why-rows with buckets (input JSON array or `{rows:[…]}`).
    Why {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum CheckSub {
    Channel {
        /// Remove dead/aged locks (also runs on plain status).
        #[arg(long, default_value_t = false)]
        clear_stale: bool,
        /// Force-remove repair+bulk locks even if PID still looks alive (hung agent).
        #[arg(long, default_value_t = false)]
        force_clear: bool,
        /// Also call phone `reset_mcp_channel` — force-release a stuck App-side
        /// debug/batch-check channel (only the phone, separate from the local
        /// file locks above). Emergency recovery; prefer stopping the batch check
        /// normally when it's just running long.
        #[arg(long, default_value_t = false)]
        reset_remote: bool,
    },
    Precheck {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value_t = 4.0)]
        timeout: f64,
        #[arg(long, default_value_t = 32)]
        concurrency: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Batch {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 80)]
        batch_size: usize,
        #[arg(long, default_value_t = 64)]
        thread_count: u32,
        #[arg(long, default_value_t = 45.0)]
        timeout: f64,
        #[arg(long)]
        materials_dir: Option<PathBuf>,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    Full {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 80)]
        batch_size: usize,
        #[arg(long, default_value_t = 64)]
        thread_count: u32,
        #[arg(long, default_value_t = 120.0)]
        timeout: f64,
        #[arg(long, help = "reuse precheck JSON with alive_urls/dead_urls")]
        precheck_json: Option<PathBuf>,
        #[arg(long)]
        materials_dir: Option<PathBuf>,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Consistent-hash shard URLs across device nodes.
    Shard {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long, help = "comma-separated device ids")]
        nodes: String,
        #[arg(long, default_value_t = 64)]
        virtual_nodes: u32,
        #[arg(long, default_value = "temp/shards.json")]
        out: PathBuf,
    },
    /// Disable and/or tag dead sources from precheck JSON.
    DisableDead {
        #[arg(long)]
        precheck_json: PathBuf,
        #[arg(long, default_value_t = false)]
        disable: bool,
        #[arg(long, default_value_t = false)]
        tag: bool,
        #[arg(long, default_value_t = 0)]
        limit: usize,
        #[arg(long, default_value = "temp/disable_dead_report.json")]
        out: PathBuf,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Batch L0→L2 classify aggregate (Python `repair_prefilter.filter_urls`).
    Prefilter {
        #[arg(long)]
        urls_file: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 16)]
        concurrency: usize,
        #[arg(long, default_value_t = 4.0)]
        l2_timeout: f64,
        #[arg(long)]
        rules: Option<PathBuf>,
    },
    /// Clear App CookieStore for a host (MCP `clear_cookies`). Use before retesting
    /// search after `ss_search_delay` / 搜索间隔 throttle.
    ClearCookies {
        #[arg(long)]
        url: String,
    },
    /// Read persisted+session cookies for a URL's registrable domain (MCP `get_cookies`).
    CookieGet {
        #[arg(long)]
        url: String,
    },
    /// Merge-write a persisted cookie for a URL's registrable domain (MCP `set_cookie`).
    CookieSet {
        #[arg(long)]
        url: String,
        #[arg(long)]
        cookie: String,
    },
    /// Toggle App HTTP log capture (MCP `set_http_log_recording`).
    LogRecording {
        #[arg(long)]
        enabled: bool,
    },
    /// Recent redacted HTTP request summaries (MCP `get_http_logs`).
    LogList {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// One redacted HTTP request/response by id (MCP `get_http_log`).
    LogGet {
        #[arg(long)]
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum KnowledgeSub {
    Search {
        #[arg(long)]
        query: String,
        #[arg(long, default_value = "")]
        layer: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
}
