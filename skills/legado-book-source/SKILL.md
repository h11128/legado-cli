---
name: legado-book-source
description: >-
  Create, debug, and iterate Legado (阅读) book sources with the local
  legadoSkill knowledge base and the device MCP (URL from mcp_defaults.json,
  never hard-code DHCP IPs). Use when writing 书源, book sources, Legado rules,
  CSS/@js selectors, debug_source, save_source, or when the user asks to make/fix
  a source for 阅读/legado.
version: 1.0.0
license: MIT
metadata:
  hermes:
    tags: [legado, book-source, 书源, mcp, reading-app]
    related_skills: [legado-book-source-repair]
---

<!-- SOT: E:\shared-skills\legado-book-source\SKILL.md
     Distributed to:
       - Claude Code: ~/.claude/skills/legado-book-source/
       - Codex: ~/.codex/skills/legado-book-source/
       - Cursor: ~/.cursor/skills/legado-book-source/
       - Hermes: %LOCALAPPDATA%\hermes\skills\software-development\legado-book-source/
     Keep agent copies identical to this file (manual sync or audit-hooks when available).
-->

# Legado Book Source

Write and verify Legado book sources against the **real app** via MCP.
Local Python debugger is only approximate; official Kotlin source and
device `debug_source` are authoritative.

Works on Claude Code, Codex, Cursor, and Hermes. Prefer the MCP tool
surface named `legado` (server may appear as `legado` / `user-legado`).

## Paths

| Role | Path |
|------|------|
| Knowledge repo | `E:/Projects/legadoSkill` |
| Official app source | `E:/Projects/legado` (junction: `legadoSkill/legado`) |
| Upstream Trae mega-skill | `legadoSkill/skills/SKILLV0.7.md` |
| Essential knowledge | `legadoSkill/docs/ESSENTIAL_KNOWLEDGE_SUMMARY.md` |
| Self-check notes | `legadoSkill/assets/智能体自我认知.md` |
| CSS rules | `legadoSkill/assets/css选择器规则.txt` |
| Example sources | `legadoSkill/assets/knowledge_base/book_sources/` |
| Local debugger | `legadoSkill/debugger/test_universal.py` |
| Local venv | `legadoSkill/.venv` |

## Device MCP (`legado`)

- **SOT:** `E:/Projects/legadoSkill/config/mcp_defaults.json` — **never hard-code phone IPs in skills/prompts**
- Read URL/token from that file. On DHCP change or connect failure:
  ```
  source-cli discover --write --sync-cursor
  ```
  (Python `scripts/mcp_discover.py` is legacy until fully removed.)
- Phone MCP also auto-starts after boot / APK update when the in-app MCP switch was left on.
- Web UI: same host `:1122`
- **Create new sources / 找站:** `docs/guides/book-source-discovery.md` + `source-cli site-probe`
- **Repair failures:** skill **`legado-book-source-repair`** + `source-cli repair` (not ad-hoc Python)

Config locations:

| Agent | Config |
|-------|--------|
| Cursor | `~/.cursor/mcp.json` → `mcpServers.legado` (kept in sync by `mcp_discover.py`) |
| Codex | `~/.codex/config.toml` → `[mcp_servers.legado]` |
| Claude Code | `~/.claude.json` → `mcpServers.legado` (`type: http`) |
| Hermes | `%LOCALAPPDATA%/hermes/config.yaml` → `mcp_servers.legado` |

If **Cursor IDE** MCP tools still fail after discover wrote a new URL: reload MCP / restart the agent once (IDE HTTP client does not live inside `mcp_client.py`). Do **not** ask the user to hand-edit the IP.

### Tools

| Tool | Use |
|------|-----|
| `list_sources` | Paginated summaries (`search`, `enabledOnly`, `offset`, `limit`; default page 100, max 500) |
| `get_source` | Read full JSON by `bookSourceUrl` |
| `save_source` | Write JS/JSON; optional `preserveEnabled`/`preserveGroup` (default true) |
| `debug_source` | Single-flight step debug (`url` + `key`); not for bulk |
| `start_check_sources` | Start multi-thread batch check (App 校验书源 logic) |
| `get_check_progress` | Poll batch check progress + paged results |
| `stop_check_sources` | Cancel batch check |
| `delete_sources` | Delete by URL list |
| `set_http_log_recording` | Toggle HTTP log capture |
| `get_http_logs` / `get_http_log` | Inspect redacted request logs |
| `clear_cookies` / `get_cookies` / `set_cookie` | CookieStore (App may expose; Cursor catalog may lag — prefer CLI) |

`debug_source` is single-flight. For bulk validation use `start_check_sources` then `get_check_progress`.
Prefer device MCP over local Python sim.

**CLI shortcuts (preferred for agents):**
```
# binary often not on PATH:
E:/Projects/legadoSkill/crates/target/debug/source-cli.exe check channel
E:/Projects/legadoSkill/crates/target/debug/source-cli.exe source push --file temp/full_fix/cache/new_sources/foo.json
E:/Projects/legadoSkill/crates/target/debug/source-cli.exe check clear-cookies --url http://www.example.com
```
Or `cargo install --path crates/source-cli --force`. Create checklist: `docs/guides/book-source-create.md`.
`source-cli` talks to the phone MCP URL in `config/mcp_defaults.json` directly (works even when Cursor's tool catalog omits `clear_cookies`). If the App build lacks the tool, CLI falls back to `eval_js` / tells you to clear in the UI.
Avoid pasting huge BookSource JSON through IDE `save_source` args (escaping breaks). Write a file → `source push`.

### Anti-block (rate / headers)

Bulk check and PC HTML probes must avoid triggering site WAF/bans:
- Pace **per host** (do not only raise global threadCount).
- Keep/reuse the source’s headerMap / UA / cookies when fetching for fixes — **except** when CookieStore holds a throttle cookie (`ss_search_delay`): then clear cookies and/or `enabledCookieJar=false`.
- Mass 403/空页/搜索失效 may mean blocking — back off before rewriting rules.
- **Search list=0 but「获取成功」:** open HTTP log **before** rewriting bookList. Tiny body with `alert("搜索间隔")` = throttle, not bad CSS.

### Bulk check on PC (precheck + batched MCP)

Do **not** dump thousands of sources at `threadCount=100` in one MCP call.
Phone heap is limited; PC should filter and page:

1. Export / list `bookSourceUrl`s (`list_sources` pages, or local URL file).
2. DNS precheck on PC:
   ```
   E:/Projects/legadoSkill/.venv/Scripts/python.exe scripts/precheck_sources.py \
     --urls-file urls.txt --concurrency 200 --out temp/precheck.json
   ```
3. Optionally disable/tag dead hosts on device:
   ```
   E:/Projects/legadoSkill/.venv/Scripts/python.exe scripts/disable_dead_sources.py \
     --precheck-json temp/precheck.json --disable --tag
   ```
4. Batch authoritative App check (50–100 URLs per call, wait until idle):
   ```
   E:/Projects/legadoSkill/.venv/Scripts/python.exe scripts/batch_check_mcp.py \
     --precheck-json temp/precheck.json --batch-size 80 --thread-count 64 \
     --keyword 我的 --out temp/batch_check_report.json \
     --materials-dir temp/check_materials
   ```
   (Omit `--mcp`; script should read `config/mcp_defaults.json`. If a flag is required, pass the URL from that file — never a remembered DHCP IP.)
   Report includes `by_failure_tag`; failed items are dumped under `temp/check_materials/<tag>/`.
5. Multi-phone: shard URLs first with `scripts/shard_urls.py`, then run batch check per device.
6. Or drive the same flow via agent MCP tools (`start_check_sources` /
   `get_check_progress`) if the script’s HTTP transport does not match.

Device-side check uses AIMD concurrency, host token buckets, work-stealing,
priority by respondTime, Bloom dedup, EWMA skip, hedged domain probe, TOC sampling,
skip-discovery-when-search-ok, batched DB writes, body caps, and DNS circuit-breaking.

Research (why not extract JVM engine): `docs/PC_CHECK_ENGINE_RESEARCH.md`
and `E:/Projects/legado/docs/pc-check-engine-research.md`.

### Agent call notes

- **Cursor**: discover server (often `user-legado`), then call tools.
- **Claude / Codex / Hermes**: use whatever MCP invoke API the host exposes for server `legado`.
- If tools are missing: reload MCP / restart agent; confirm phone service on `:1236`.
- After updating app MCP tools, rebuild/reinstall the app and restart MCP service.
## Workflow (3 phases)

```
- [ ] Phase 0 (找站): site-probe → pick make_candidate  [optional]
- [ ] Phase 1: gather (no save yet)
- [ ] Phase 2: draft rules from real HTML
- [ ] Phase 3: save + device debug + fix loop
```

**Full create checklist (traps + CookieJar + debug keys + CLI path):**
`docs/guides/book-source-create.md`

### Phase 0 — Discovery (找站 / 出版 / 公版)

Use when the user asks to **find sites** or make **出版/公版/古籍** sources — not when repairing an existing URL.

1. `source-cli check channel` (must idle).
2. Batch gate **new URLs** (not only the publish ledger):
   ```
   source-cli site-probe --url https://example.com \
     --out temp/full_fix/queues/site_probe.json
   ```
   Or `--urls-file urls.txt` (lines: `url|kind|note`).  
   Known 出版 seeds ledger: `--preset publish` (mostly static; add `--include-sourced` to recheck).
3. Only draft rows with `discovery=make_candidate`. Skip `skip_waf` / `skip_vpn` / `skip_app` / `skip_pdf` / `skip_catalog`.
4. Reuse phone inventory first: `list_sources --search 出版|公版|名著|古登堡`.
5. Full guide + traps: `legadoSkill/docs/guides/book-source-discovery.md`
6. Seeds SOT: `legadoSkill/config/site_candidates_publish.json` (append useful finds as `candidate` or `sourced`).

**Kind → keyword tips:** Gutenberg `Alice`; ctext `论语`; mixed `红楼`.  
**Traps:** ctext TOC must be absolute `https://ctext.org/...`; Gutenberg often one 全文 HTML chapter; never rewrite selectors on CF challenge HTML; MCP `save_source` may ignore `customOrder` unless `preserveOrderWeight=false`.

### Phase 1 — Gather (do not save)

1. Read `docs/ESSENTIAL_KNOWLEDGE_SUMMARY.md`; skim CSS rules and similar
   sources under `assets/knowledge_base/book_sources/` or
   `assets/book_source_database/`.
2. Detect site charset (response header / meta / probe fetch).
3. Fetch **raw HTTP HTML** (not DevTools DOM). Save under
   `legadoSkill/temp/` if useful. Use a browser tool only when the site
   needs JS/WebView.
4. Note search URL shape (GET vs POST), list/detail/toc/content URLs.

### Phase 2 — Draft

1. Build selectors from **raw HTML** only.
2. Prefer CSS short form (`.name@text`, `#id@text`, `.a.b@href`).
3. Handle lazy cover (`data-src` / `@data-src`), merged info fields,
   pagination (`nextTocUrl` / `nextContentUrl`).
4. **Multi-TOC containers** (笔趣阁 `list-charts`, ttks 最新+全部, etc.): do **not**
   hardcode `.0`/`.1`. Prefer `@js` that picks the container with the most chapter links.
5. **Relative ajax toc:** `tocUrl`/`ajax_index.html` → `@js: baseUrl + 'ajax_index.html'`.
6. **PC body empty / m. OK:** keep desktop search+TOC; rewrite chapter fetch to `m.` sibling.
7. Set `checkKeyWord` to a **rare title fragment** (not 「我的」) for the book you care about.
8. For JS rules: Rhino; prefer `var`; use `java.*` helpers. See
   `legadoSkill/assets/方法-JS扩展类.md` when needed.
9. When unsure, read official Kotlin under
   `legado/app/src/main/java/io/legado/app/`.

### Phase 3 — Save and verify on device

1. Write JSON under `temp/full_fix/cache/new_sources/<host>.json`.
2. `source-cli check channel` (idle) → `source-cli source push --file …`
   (binary often `crates/target/debug/source-cli.exe` — see create guide).
3. `debug_source` with a real search keyword. If **list=0**:
   - `set_http_log_recording(true)`, re-debug once, read `get_http_log`.
   - Throttle (`搜索间隔` / `ss_search_delay`): clear-cookies + `enabledCookieJar=false`.
   - CF needing cookies: `enabledCookieJar=true` + `webView:true`; still blocked → skip.
   - **Do not** rewrite `bookList` on throttle/challenge HTML.
4. Prove detail/toc/content with an **absolute** book URL (or `++URL`).  
   **Never** use `::URL` for that — `::` is 发现/explore.
5. `start_check_sources` with `checkDiscovery=false`, keyword = `checkKeyWord`,
   `timeoutMs` ≥ `verify_timeout_ms` in mcp_defaults (slow hosts ≥180000).
   `debug_source` `timeoutSec` ≥ `debug_timeout_s` (raise if mid-content truncated).
6. Only claim success on device `校验成功`.

## Hard rules

- Real HTML > browser rendered DOM.
- Official Kotlin in `E:/Projects/legado` > Python debugger.
- Device MCP debug > local simulation.
- Do not invent unsupported BookSource fields; mirror working examples.
- Keep tokens out of committed book-source JSON; MCP auth lives in agent configs.
- Search empty + HTTP ok → **check throttle/WAF body first**, then selectors.
- After meaningful book-source work, optionally write L3 memory via
  `audit-hooks l3 write "..."` (see `l3-memory-client` skill).

## Common traps (create path)

| Trap | Signal | Fix |
|------|--------|-----|
| `ss_search_delay_cookie` | list=0; body `搜索间隔`; Cookie `ss_search_delay` | clear-cookies + `enabledCookieJar=false` |
| `cookiejar_cf_needs_on` | CF search empty without cookies | jar true + webView; else skip/manual |
| `multi_toc_pick_longest` | TOC only latest N chapters | @js max link-count container |
| `relative_ajax_toc` | ajax_index relative → toc empty | `baseUrl + 'ajax_index.html'` |
| `desktop_empty_mobile_content` | PC content empty; m. readable | hybrid chapter URL to m. |
| `debug_colon_explore` | used `::URL` expecting detail | absolute URL or `++URL` |
| `check_keyword_too_broad` | first search hit bad/empty book | rarer `checkKeyWord` + matching check keyword |
| `multi_list_charts_toc` | 笔趣阁 list-charts `.1` only latest | same as multi_toc_pick_longest |
| CF Turnstile | webView 仍搜索失效 | skip / loginUrl 手工过验证 |
| IDE `save_source` escape | JSON 解析失败 / 规则被截断 | `source-cli source push --file` |
| PC curl OK / phone list=0 | 同公网 IP + 手机 Cookie 已限流 | clear phone cookies；少用 PC 连搜同域 |
| `source-cli` not found | command not found | `crates/target/debug/source-cli.exe` or `cargo install --path crates/source-cli` |

## When MCP is missing

Fallback: write JSON under `legadoSkill/temp/` and tell the user to import
via Web `:1122` or the app UI.

## Extra reference

- **Create checklist:** `docs/guides/book-source-create.md`
- **Discovery guide:** `docs/guides/book-source-discovery.md`
- Upstream Trae skill (long, custom tools): `skills/SKILLV0.7.md`
- Architecture: `docs/PROJECT_ARCHITECTURE.md`
- Charset / POST encoding: `docs/MCP编码使用指南.md`
- Local debugger: `docs/LEGADO_DEBUGGER.md`
- Install notes: `E:/Projects/legadoSkill/MULTI_AGENT_SETUP.md`
