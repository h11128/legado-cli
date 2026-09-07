# Book-source create checklist (找站后做源)

Agent entry: skill `legado-book-source`. Repair of existing URLs stays in `legado-book-source-repair`.

## CLI binary (Windows)

Prefer installing onto PATH once:

```bash
cd crates && cargo build -p source_cli
# from repo root (or crates/):
source-cli install          # cargo install --path … --force → ~/.cargo/bin
# new shell:
source-cli --help
```

If `source-cli` is still missing, use the local debug binary:

```bash
./crates/target/debug/source-cli.exe --help
```

Never invent a second `--target-dir`. MCP URL/token: `config/mcp_defaults.json`.

**Clear cookies SOT:** always `source-cli check clear-cookies --url …`.
Do **not** rely on Cursor MCP tool lists — they often omit App `clear_cookies`
even though the phone implements it (CLI calls MCP or falls back to `eval_js`).

## Fast path

```
1) source-cli check channel
2) Optional scaffold: source-cli source scaffold --url http://host [--name …]
3) Fetch raw HTML (PC curl / source-cli fetch) — not DevTools DOM; rewrite selectors
4) Draft JSON → temp/full_fix/cache/new_sources/<host>.json
5) source-cli source push --file …          # claims deep_active (prefer over IDE save_source)
6) debug_source keyword → if list=0: HTTP log / diagnose (auto tip on 搜索间隔)
7) debug detail/toc/content (see debug keys below)
8) start_check_sources checkDiscovery=false, timeoutMs≥90000 (slow≥180000)
9) Only claim done on 校验成功
10) MANDATORY close-out (same as repair — do not wait for user reminder):
     ledger append → retro append (trap/skill_fix/script_fix) → improve → git commit
```

**Unsealed `deep_active` after push → `progress next` / `closeout pending` DENY.**
Turn-end: stop hook `.cursor/hooks/check-deep-active-stop.py` auto-followups close-out
(wire via `.cursor/hooks.json.example` → local gitignored `hooks.json`).
Seal with `retro append --status fixed|skip|fail`. Escape: `closeout release`.
**Do not wait for the user to remind** — improve skill/docs/harness on novel traps before the next host.

Timeouts SOT: `config/mcp_defaults.json` (`debug_timeout_s`, `verify_timeout_ms`, `http_timeout_s`).
MCP `debug_source` may pass `timeoutSec` to override; check may pass `timeoutMs`.

## Phase 4 — Close-out / improve (mandatory after every create attempt)

Same gate as repair (discipline §14 / §14b). After success **or** skip/fail:

1. `source-cli ledger append --url <bookSourceUrl> --step check --result '校验成功'|fail:…|skip:…`
2. `source-cli retro append --url … --status fixed|skip|fail --trap '…' --skill-fix 0|1 --script-fix '…'`
3. If **novel** trap: update `legado-book-source` and/or `legado-book-source-repair` **and**
   harness (`diagnose_tips` / `source_patch` / sniff…) or `script_fix=no_auto:<理由>`
4. Append a short note to `docs/source-repair-retrospective.md` when useful
5. `git commit` skill/docs/rust before the next site

Do **not** start the next host until close-out finishes. User should not have to remind.
## CookieJar decision (do not guess)

| Signal | enabledCookieJar | Action |
|--------|------------------|--------|
| HTTP body `搜索间隔` / Cookie `ss_search_delay` | **false** | **`source-cli check clear-cookies --url …`** then retest; optional searchUrl `@js` `cookie.removeCookie` |
| CF / Turnstile needs browser cookie to search | **true** + often `webView:true` | If still blocked → skip / `loginUrl` manual; do not rewrite selectors on challenge HTML |
| Site needs login session | **true** | loginUrl / loginUi; do not clear cookies mid-debug |

Note: even with jar-save off, Legado **reads** CookieStore into requests — after a throttle hit you must **clear**, not only set `enabledCookieJar=false`.

## debug_source key modes

| key | Meaning |
|-----|---------|
| plain text | Search keyword |
| `https://…/book/…` absolute URL | Book info → toc → content |
| `++https://…` | Force book-info style entry (when needed) |
| `::https://…` | **Explore / 发现**, not detail — do not use for TOC/content prove |
| `--https://…` | Other debug modes (see App) |

Wrong: `::http://site/xs/1/2/` then wondering why「发现页」list=0.

## TOC: multiple containers → pick longest

Same class of bug as 15u `list-charts` and ttks「最新+全部」:

- Several `ul` / `div` / frames hold chapter links.
- Fixed `.0` / `.1` / `chapters_frame.1` often grabs 「最新章节」 only.
- Prefer `@js`: among candidate containers, pick the one with **max** `a[href]` (or `li>a`) count.

## Relative tocUrl / ajax catalog

If HTML has `ajax_index.html` or relative catalog path:

- Relative-only `tocUrl` often resolves wrong against chapter URL.
- Use `@js: baseUrl + 'ajax_index.html'` (or absolute `https://host/...`).

## PC empty content / mobile OK (hybrid)

Some sites (e.g. xsw.tw): desktop detail/TOC OK, chapter body empty without JS; `m.` host has `#nr1` / readable HTML.

Pattern:

1. Keep search + TOC on desktop `www.`
2. Rewrite `chapterUrl` (or content fetch) to `m.` sibling when PC body empty
3. Verify on device; do not rewrite TOC selectors for empty PC content alone

## checkKeyWord

- Default「我的」hits junk / empty first books → false TOC fail.
- Prefer a **rare title fragment** of the book you care about (`收徒万倍`, `收徒萬倍`, …).
- Align `start_check_sources` `keyword` with `ruleSearch.checkKeyWord`.

## Common traps

| Trap | Signal | Fix |
|------|--------|-----|
| `ss_search_delay_cookie` | list=0; alert 搜索间隔 | **`source-cli check clear-cookies`** + jar false |
| `cookiejar_cf_needs_on` | CF search empty without jar/webView | jar true + webView; else skip |
| `multi_toc_pick_longest` | TOC only latest N | @js max link-count container |
| `relative_ajax_toc` | toc empty; ajax_index relative | baseUrl + path |
| `desktop_empty_mobile_content` | PC content empty; m. OK | hybrid chapter URL |
| `debug_colon_explore` | used `::URL` for detail | absolute / `++` URL |
| `check_keyword_too_broad` | first hit bad book | rarer checkKeyWord |
| `ide_save_escape` | save JSON broken | `source push --file` |
| CF Turnstile | webView still fail | skip / manual loginUrl |
| CLI not found | `source-cli: command not found` | `source-cli install` or `crates/target/debug/source-cli.exe` |

## Scaffold (笔趣阁系 MVP)

```bash
source-cli source scaffold --url http://www.example.com --name '例站'
# → temp/full_fix/cache/new_sources/www_example_com_scaffold.json
```

Draft only: CookieJar off, GET-ish search with `removeCookie`, longest-list TOC JS,
common content selectors. **Must** fetch live HTML and rewrite before verify.

## Related

- Discovery / 找站: `docs/guides/book-source-discovery.md`
- Skill: `legado-book-source`
- Repair: `legado-book-source-repair`
- MCP: `config/mcp_defaults.json`
