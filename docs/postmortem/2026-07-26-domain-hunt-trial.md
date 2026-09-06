# Domain hunt trial (2026-07-26) + policy (aligned 2026-07-28)

CLI SOT: `source-cli hunt --url … --probe` + seeds `config/domain_hunt_seeds.json`.
(Legacy Python `repair_domain_hunt.py` removed in Rust cutover.)

Probes = same L1/L2 as gate/prefilter (not App check).

## Results (trial)

| Source | Action | Best | Notes |
|--------|--------|------|-------|
| zxcs.info | **migrate** | https://www.zxcs.click/ | also live: zxcs.live, www.zxcs.info; zxcs.zip SSL fail |
| 627txt / 爱去 | **migrate** | https://www.aiqu226.com/ | aiqu225 also L2 OK |
| tiexue book | **no_mirror** | — | official shutdown ~2026-03; correct to disable |
| dddw.net | **none / weak** | — | random bxwx clones ≠ successor; do not auto-migrate |

## Policy (SOT — must match SKILL + gate + oneshot)

1. **Hunt before hard-disable** when reason is timeout / dead HTTP (`l1_unreachable`,
   `l2_http_dead`, L0 `timeout_cluster` → `GateAction::Hunt`).
2. Exceptions (**no seed hunt**): L0 `dead_site_shutdown_confirmed`; L2 parked/ad-hijack/nginx
   `deadish:` → Disable/Skip; password/bot wall → Skip.
   **Still run OSINT successor pass** when brand may have moved
   (`python scripts/domain-successor-hunt.py` in the **legado** repo): crt.sh +
   rate-limited Wayback + Google query templates. Trap: `hunt_osint_skipped`.
3. `source-cli repair --mode oneshot|batch` **must** resolve hunt (seed probe) before Disable.
   Wave must **not** ledger-seal hunt rows as final skip.
4. `migrate` = rewrite `bookSourceUrl` + re-verify; do not claim fixed on L2 alone.
5. Video hosts use `action: video` → `legado-video-source-repair`, not novel disable.
6. **Wayback**: never parallel/burst `web.archive.org` / `archive.org`. Use
   `legado/scripts/lib/wayback_cdx.py` (min interval default 12s, 429 exponential backoff,
   shared lock `temp/full_fix/cache/wayback_rate_lock.json`). Env:
   `WAYBACK_MIN_INTERVAL_S`, `WAYBACK_MAX_RETRIES`.

Harness: `source-gate/classify.rs`, `source-hunt` (`HuntAction` / `HuntResolve`),
`source-cli` `oneshot_live` + `hunt`, `source-check/wave.rs`,
`legado/scripts/domain-successor-hunt.py`.
Skill traps: `dead_skip_without_hunt`, `hunt_osint_skipped`.
