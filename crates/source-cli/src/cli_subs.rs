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
        /// diagnose | oneshot | mcp_fallback | create
        #[arg(long, default_value = "")]
        entry: String,
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
    /// Single-flight `debug_source` (search keyword, or absolute/::/++/-- URL).
    Debug {
        #[arg(long)]
        url: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 120)]
        timeout_sec: u64,
    },
    /// Raw `get_source` — full BookSource JSON by bookSourceUrl (unlike `triage`,
    /// which prints a smell/layer summary instead of the source itself).
    Get {
        #[arg(long)]
        url: String,
    },
    /// `list_sources` — paginated summaries, optional name/URL substring filter.
    List {
        #[arg(long)]
        search: Option<String>,
    },
    /// `delete_sources` — permanently remove one or more by bookSourceUrl.
    Delete {
        #[arg(long, value_delimiter = ',')]
        urls: Vec<String>,
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
    /// Push BookSource JSON file to phone via MCP `save_source` (avoids IDE escaping hell).
    Push {
        #[arg(long)]
        file: PathBuf,
        /// Overwrite enabled/group from JSON (default true for new drafts).
        #[arg(long, default_value_t = true)]
        #[arg(long = "no-overwrite", action = clap::ArgAction::SetFalse)]
        overwrite: bool,
    },
    /// Scaffold a 笔趣阁-family draft JSON (MVP — rewrite selectors from live HTML).
    Scaffold {
        #[arg(long, help = "site base url or host, e.g. http://www.15u.cc")]
        url: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(
            long,
            help = "default: temp/full_fix/cache/new_sources/<host>_scaffold.json"
        )]
        out: Option<PathBuf>,
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

#[derive(Subcommand)]
pub enum McpSub {
    /// List all remembered MCP endpoints with active marker and status
    List {
        #[arg(long, default_value_t = false)]
        probe: bool,
    },
    /// Add or update a remembered MCP endpoint
    Add {
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "1234")]
        token: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, default_value_t = false)]
        switch: bool,
    },
    /// Switch active MCP endpoint to the specified URL
    Switch {
        #[arg(long)]
        url: String,
    },
    /// Remove an MCP endpoint from remembered list
    Remove {
        #[arg(long)]
        url: String,
    },
    /// Probe all remembered endpoints and switch to the first alive one
    Probe {
        #[arg(long, default_value_t = 3.0)]
        timeout: f64,
    },
}
