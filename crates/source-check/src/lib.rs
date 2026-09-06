//! Bulk MCP check orchestration — port of batch_check_mcp / full_check_runner.
//! Also: wave/harvest orchestration, URL sharding, disable-dead (Python parity).

mod batch;
mod bench;
mod channel;
mod deep_wave;
mod disable_dead;
mod harvest;
mod materials;
mod precheck;
mod prefilter;
mod search_form;
mod search_wave;
mod shard;
mod wave;
mod wave_patch;

pub use batch::{
    dedupe_urls, load_alive_from_precheck, load_urls_file, run_batch_check, BatchCheckOpts,
    BatchCheckSummary,
};
pub use bench::{run_bench10, BenchOpts, DEFAULT_BENCH_URLS};
pub use channel::channel_status_json as channel_status;
pub use deep_wave::{run_deep_wave, DeepWaveOpts};
pub use disable_dead::{
    apply_disable_dead, apply_limit, ensure_tag, filter_alias_dead_urls, load_dead_urls,
    plan_disable_dead, strip_tag, write_report, DisableDeadError, DisableDeadOpts, DEAD_TAG,
};
// Re-export alias helpers used by disable-dead / ops scans.
pub use harvest::{default_fails_path, run_harvest, HarvestOpts};
pub use materials::{classify_results, dump_fail_materials, FAIL_TAGS};
pub use precheck::{
    parse_host, precheck_json, precheck_report, precheck_urls, probe_one, PrecheckRow,
};
pub use prefilter::{filter_urls, PrefilterSummary};
pub use search_wave::{run_search_wave, SearchWaveOpts};
pub use shard::{
    build_ring, load_urls_file as load_shard_urls_file, mix, node_for, shard_urls, str_hash32,
    write_shards, ShardError,
};
pub use source_gate::{
    classify_alias_row, is_alias_book_source_url, refuse_dead_tag_reason, ALIAS_GATE_REASON,
};
pub use wave::{default_rules_path, run_wave, WaveOpts};
