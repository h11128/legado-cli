//! Structural close-out: trap gate, pending progress next, retro seal, SKILL sync.
//!
//! Rust port of `scripts/repair_closeout.py` + `scripts/repair_retro.py`.

mod improve;
mod jsonl;
mod paths;
mod pending;
mod retro;
mod session_index;
mod skill;
mod trap;

pub use improve::{gate_script_fix, script_fix_ok};
pub use jsonl::{read_jsonl, JsonRow};
pub use paths::CloseoutPaths;
pub use pending::{pending_closeout, PendingDetail};
pub use retro::{append_retro, RetroAppendOpts, RetroRow};
pub use session_index::{append_index, assert_fixed_allowed, load_check_json};
pub use skill::{skill_fingerprint, skill_in_sync, sync_skill_to_cursor};
pub use trap::gate_trap;

/// Exit 0 when progress may pick the next URL; non-zero when blocked.
pub fn ensure_ready_for_next(paths: &CloseoutPaths) -> Result<PendingDetail, Vec<String>> {
    let (ok, errors, detail) = pending_closeout(paths);
    if ok {
        Ok(detail)
    } else {
        Err(errors)
    }
}
