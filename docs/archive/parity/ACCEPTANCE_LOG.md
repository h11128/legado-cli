# Parity acceptance log

Record sign-off only when **thorough** gates pass (`docs/parity/THOROUGH_ACCEPTANCE.md`).

| date (UTC) | git sha | suites | operator | notes |
|------------|---------|--------|----------|-------|
| 2026-07-28 | `370fa84` | rust cutover + parity 3/3 | agent | Full Python removal. CLI subcommands; batch MCP + bulk channel lock; queue rt → JSON; closeout/retro in Rust; `cargo test/clippy --workspace` green. Live MCP smoke optional follow-up. |
| 2026-07-27 | `9ac2853` | 7/7 incl. search-parity | agent | **Thorough §12 functional (口径 A) green.** SEARCH_LAYER_GAPS S1–S12 all `done`. Suites + golden forms OK. Live E2E: biduju search-layer (`keyword`+GBK+`class.list@table`) then content `class.chapter@html` (textNodes empty) → device **校验成功**. S10 tips verified via `--debug-file` fake_detail + live probe. |
| 2026-07-27 | _(wip)_ | search-parity + rust-cli expanding | agent | **Retracted soft §12 functional green.** Gap: live rank / score / dead-endpoint / JS API were incomplete. Hard gate: search-layer E2E + `THOROUGH_ACCEPTANCE.md`. |
| 2026-07-27 | `87a431e` | 6/6 soft suites | agent | **SUPERSEDED** — CLI/shim inventory only; insufficient for thorough functional parity. |
| 2026-07-27 | e4d5d20 | 5/5 | agent | Phase A harness |

## Thorough gate (required)

See `THOROUGH_ACCEPTANCE.md`. Forbidden: claim green with only `layer=ok` smoke.

## Evidence

```
# Search-layer (earlier in same track)
source-cli repair --url http://www.biduju.net
→ keyword + GBK + class.list@table → 校验成功

# Content regression catch (2026-07-27)
debug: ContentEmptyException with class.chapter@textNodes
patch: ruleContent.content = class.chapter@html
start_check_sources → 校验成功 (2159ms)

# S10 diagnose tips (debug-file fake_detail + live probe)
tips include fake_detail trap + probe.best score=7 + GBK
```

Gap backlog: `SEARCH_LAYER_GAPS.md` (no open search-layer rows).
