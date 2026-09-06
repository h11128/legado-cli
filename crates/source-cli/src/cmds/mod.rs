//! CLI subcommands for source-cli.

mod cache_cmd;
mod check_cmd;
mod check_ops;
mod claim_cmd;
mod closeout;
mod db_cmd;
mod debug_vs_check;
mod diagnose;
mod diagnose_http_log;
mod diagnose_tips;
mod dig;
mod discover_cmd;
mod ewma;
mod fetch_cmd;
mod gate;
mod hunt;
mod install_cmd;
mod knowledge_cmd;
mod ledger_cmd;
mod mcp_raw;
mod mcp_tools;
mod migrate;
mod oneshot_finish;
mod oneshot_hunt;
mod oneshot_live;
mod oneshot_prep;
mod ops_bridge;
mod orchestrate;
mod parity;
mod parity_search;
mod parse_cmd;
mod pattern_cmd;
mod probe;
mod probe_score;
mod progress;
mod progress_goal;
mod progress_ledger;
mod queue_cmd;
mod queue_ops;
mod repair;
mod repair_dry;
mod repair_outcome;
mod retro;
mod scaffold;
mod search_dead;
mod search_plan;
mod serial_cmd;
mod serial_spawn;
mod site_probe;
mod source_cmd;
mod version;
mod video_route;

pub use claim_cmd::{run_claim, ClaimCmd};
pub use closeout::{run_closeout, CloseoutArgs};
pub use debug_vs_check::{run_debug_vs_check, DebugVsCheckArgs};
pub use diagnose::{run_diagnose, DiagnoseArgs};
pub use dig::{run_dig, DigArgs};
pub use discover_cmd::{run_discover, DiscoverArgs};
pub use ewma::run_ewma;
pub use fetch_cmd::{run_fetch, FetchArgs};
pub use gate::{run_gate, GateArgs};
pub use hunt::{run_hunt, HuntArgs};
pub use install_cmd::run_install;
pub use ledger_cmd::{run_ledger, LedgerCmd};
pub use mcp_raw::{run_mcp_raw, McpRawCmd};
pub use mcp_tools::{run_mcp_tools, McpToolsCmd};
pub use migrate::{run_migrate, MigrateArgs};
pub use ops_bridge::{
    run_cache_sub, run_check_sub, run_db_sub, run_knowledge_sub, run_pattern_sub, run_queue_sub,
};
pub use orchestrate::{
    run_bench_cmd, run_deep_wave_cmd, run_goal15_cmd, run_harvest_cmd, run_search_wave_cmd,
    run_serial_cmd, run_wave_cmd, BenchArgs, DeepWaveArgs, Goal15Args, HarvestArgs, SearchWaveArgs,
    SerialArgs, WaveArgs,
};
pub use parity::{run_parity, ParityArgs};
pub use parse_cmd::{run_parse, ParseCmd};
pub use probe::{run_probe, ProbeArgs};
pub use probe_score::run_probe_score;
pub use progress::{run_progress, ProgressArgs};
pub use repair::{run_repair, RepairArgs};
pub use repair_dry::{run_repair_dry, RepairDryArgs};
pub use retro::{run_retro, RetroArgs};
pub use scaffold::{default_out_for as scaffold_default_out, run_scaffold};
pub use site_probe::{run_site_probe, SiteProbeArgs};
pub use source_cmd::{run_source, SourceCmd};
pub use version::run_version;
pub use video_route::run_video_route;
