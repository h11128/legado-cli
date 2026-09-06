# Python → Rust cutover matrix

**Status: DONE (2026-07-28)** — all tracked `scripts/*.py` and `debugger/*.py` removed.

| Former Python | Rust replacement |
|---------------|------------------|
| `repair_closeout*.py` / `repair_retro.py` | `source-cli closeout` / `source-cli retro` |
| `repair_progress.py` | `source-cli progress` |
| `repair_*` shims | `source-cli gate/diagnose/repair/ledger/…` |
| `mcp_*` | `source-mcp` + `source-cli discover` / `check channel` |
| `batch_check_mcp` / `full_check_runner` | `source-cli check batch|full` |
| `repair_wave/harvest/serial` | `source-cli wave|harvest|serial` |
| `parity_*` | `source-cli parity` + `cargo test --workspace` |
| `debugger/*` | `source-parse` + `source-cli parse` |

Perf baseline: `docs/parity/PERF_BASELINE.json` (populate via future `source-cli parity perf`).
