//! MCP HTTP client + port adapters for legadoSkill (architecture §14.2 / Phase B–C).
//!
//! Python SOT: `scripts/mcp_client.py`, `scripts/mcp_channel.py`,
//! `scripts/repair_wait.py`, `scripts/repair_session_log.py`.

mod batch_verify;
mod channel;
mod channel_pid;
mod client;
mod discover;
mod endpoint;
mod fakes;
mod ledger;
mod root;
mod source_repo;
mod timeouts;
mod verify;

pub use batch_verify::{batch_check_urls, batch_max_wait_s};
pub use channel::{
    clear_stale_locks, force_clear_locks, status as channel_status_raw, FsChannelGuard,
    FsChannelPort, BULK_STALE_S, REPAIR_STALE_S,
};
pub use client::McpClient;
pub use discover::{
    apply_discovery, discover, ensure_reachable, probe_mcp, sync_cursor_mcp_json,
    write_defaults as write_discover_defaults,
};
pub use endpoint::{DefaultsConfig, McpEndpoint, McpEndpointRecord};
pub use fakes::{
    MemChannelGuard, MemChannelPort, MemClock, MemLedgerPort, MemSourceRepository, MemVerifyPort,
};
pub use ledger::{
    default_jsonl_path, default_sqlite_path, DualLedgerPort, JsonlLedgerPort, SqliteLedgerPort,
};
pub use root::repo_root;
pub use source_repo::{url_candidates, McpSourceRepository};
pub use timeouts::{timeouts_from_existing_defaults, McpTimeouts};
pub use verify::{is_repair_success, McpVerifyPort};
pub fn channel_status() -> Result<serde_json::Value, source_types::PortError> {
    let root = repo_root()?;
    channel::status(&root)
}

#[cfg(test)]
mod live_tests {
    use std::sync::Arc;

    use source_ports::{SourceRepository, VerifyPort};
    use source_types::{CheckOpts, SourceKey};

    use super::*;

    /// Live phone MCP — run with `cargo test -p source_mcp -- --ignored`.
    #[test]
    #[ignore = "requires reachable phone MCP from mcp_defaults.json"]
    fn live_get_save_roundtrip_smoke() {
        let ep = McpEndpoint::load_defaults().expect("defaults");
        let client = Arc::new(McpClient::new(ep).with_client_name("source_mcp_live"));
        client.ensure_session().expect("session");
        let repo = McpSourceRepository::new(client);
        // Intentionally use a URL unlikely to exist — expect Permanent, not transport panic.
        let key = SourceKey::new("https://source-mcp-live-missing.example/");
        let err = repo.get(&key).expect_err("missing source");
        let _ = format!("{err}");
    }

    #[test]
    #[ignore = "requires reachable phone MCP; starts a device check"]
    fn live_verify_smoke() {
        let ep = McpEndpoint::load_defaults().expect("defaults");
        let client = Arc::new(McpClient::new(ep).with_client_name("source_mcp_live"));
        let verify = McpVerifyPort::new(client).with_max_wait_s(30.0);
        let key = SourceKey::new("https://source-mcp-live-missing.example/");
        let _ = verify.check(&key, CheckOpts::default());
    }
}
