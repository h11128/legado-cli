# Serial repair reflection (2026-07-27)

## Run summary (`repair_serial.py --limit 100`)

| Metric | Value |
|--------|-------|
| Elapsed | ~414s (~6.9 min) |
| Fixed (this pass) | 10 |
| Skip (L2/pre) | 39 |
| Fail | 29 |
| Missing (not on phone) | 9 |
| Goal progress | fixed_n **58**/100 |

Artifacts:
- Queue: `temp/full_fix/queues/repair_serial100_queue.json`
- Per-URL retro: `temp/full_fix/repair_serial_retro.jsonl`
- Summary: `temp/full_fix/serial_last.json`

## What got more efficient (Harness)

| Gap | Component | Fix |
|-----|-----------|-----|
| Pick slow sites | Script + Config | `repair_rt_queue.py` sort by `respondTime`, cap 15s |
| Agent forgets to reflect | Script | `repair_retro.py` + serial auto-append |
| CF zstd empty HTML | Script | `Accept-Encoding: gzip, deflate` |
| JS search shell (paper027) | Script + Skill | `detect_js_search_api` |
| Serial dies on one bad URL | Script | try/except around `process_one` |
| Scheme-less `www.*` / `searchUrl` | Script + Skill | auto `http://` + save |
| Probe burns 2 min | Script | rank max_fetch=6, timeout 5s |
| Fail retries same host | Script | fail → block host in-run |
| Missing not in ledger | Script | append skip on get_source miss |
| get_source `#tag` miss | Script | hash-strip variants |

## Remaining inefficiencies

1. **Stale `all_sources.json`** — many queue URLs missing on phone; need MCP/web export refresh before each 100.
2. **Empty-probe fails** — still verify without patch (~2–3s each); OK for cheap wins, but search fails need diagnose-depth or family adapters.
3. **Bad migrate targets** — L2 migrate to parked/corporate hosts (`verint.com`) then timeout; gate migrate_to with L2 on target.
4. **Weird bookSourceUrl** (`x.com`) — verify OK via relative search; normalize to real host when possible.
5. **Duplicate variants** in tagged fails — host block helps in-run; rebuild queue must exclude ledger hosts (already).

## Next serial pass policy

1. Rebuild RT queue after ledger growth.
2. Prefer enabled novel type=0, respondTime≤10s for denser wins.
3. On `fail` with no notes: optional one `repair_diagnose` only if respondTime≤2s (fast sites worth depth).
4. Refresh phone source dump when web:1122 available.
