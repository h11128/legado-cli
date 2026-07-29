//! Additional clap subcommands (keeps cli.rs under line budget).

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum CloseoutSub {
    Pending,
    Gate {
        #[arg(long)]
        trap: String,
        #[arg(long, default_value_t = false)]
        skill_fix: bool,
        #[arg(long, default_value = "")]
        script_fix: String,
    },
    SyncSkill,
    Status,
    /// Claim current deep-diagnose URL (blocks progress next until release/retro).
    Claim {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "manual")]
        note: String,
    },
    Heartbeat,
    /// Seal deep_active without full retro (escape hatch; prefer retro append).
    Release {
        #[arg(long, default_value = "")]
        url: String,
        #[arg(long, default_value = "skip")]
        status: String,
    },
    ClearActive,
}

#[derive(Subcommand)]
pub enum LedgerSub {
    Append {
        #[arg(long)]
        url: String,
        #[arg(long)]
        step: String,
        #[arg(long)]
        result: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, help = "what went wrong / minutes wasted")]
        waste: Option<String>,
    },
    Show {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum ProgressSub {
    Status {
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long)]
        goal: Option<usize>,
    },
    Next {
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long)]
        rules: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        l0_only: bool,
        #[arg(long)]
        goal: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum PatternSub {
    /// Cluster verify-ok sources into PatternCluster (full BookSource rules).
    Extract {
        #[arg(long)]
        sources_file: Option<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 3)]
        min_size: u32,
        #[arg(long, default_value_t = 0)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        write_db: bool,
        /// If snapshot missing full rules, pull via MCP (default true). Cached rows are reused.
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-from-mcp", action = clap::ArgAction::SetFalse)]
        from_mcp: bool,
        #[arg(long, default_value_t = false)]
        enabled_only: bool,
        /// Prefer ledger fixed/校验成功 keys (sets verify_ok). Default true.
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-fixed-only", action = clap::ArgAction::SetFalse)]
        fixed_only: bool,
    },
}

#[derive(Subcommand)]
pub enum RetroSub {
    Append {
        #[arg(long)]
        url: String,
        #[arg(long)]
        status: String,
        #[arg(long, default_value = "")]
        msg: String,
        #[arg(long, default_value = "")]
        name: String,
        #[arg(long)]
        respond_time: Option<i64>,
        #[arg(long, default_value_t = 0.0)]
        waste_s: f64,
        #[arg(long, default_value = "")]
        trap: String,
        #[arg(long, default_value = "")]
        harness: String,
        #[arg(long, default_value = "")]
        script_fix: String,
        #[arg(long, default_value_t = false)]
        skill_fix: bool,
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-seal", action = clap::ArgAction::SetFalse)]
        seal: bool,
    },
}

#[derive(Subcommand)]
pub enum SourceSub {
    Triage {
        #[arg(long)]
        url: String,
        #[arg(long)]
        fail_msg: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
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
    Verify {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long, default_value_t = 45_000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = true)]
        auto_cooldown: bool,
        #[arg(long, default_value_t = 0.0)]
        cooldown: f64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Log {
        #[arg(long)]
        url: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        status: String,
        #[arg(long, default_value = "我的")]
        keyword: String,
        #[arg(long)]
        root_cause: Option<String>,
        #[arg(long)]
        check_json: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        index: Option<PathBuf>,
        #[arg(long)]
        agent: Option<String>,
    },
    Index {
        #[arg(long)]
        from_log: PathBuf,
        #[arg(long)]
        index: PathBuf,
    },
    Channel {
        #[arg(long, default_value_t = false)]
        clear_stale: bool,
        #[arg(long, default_value_t = false)]
        force_clear: bool,
    },
}

#[derive(Subcommand)]
pub enum ClaimSub {
    Validate {
        #[arg(long)]
        check_json: PathBuf,
    },
    AppendIndex {
        #[arg(long)]
        index: PathBuf,
        #[arg(long)]
        status: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        evidence: Option<PathBuf>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        root_cause: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ParseSub {
    Rule {
        #[arg(long)]
        rule: String,
    },
    Url {
        #[arg(long)]
        url: String,
    },
}
