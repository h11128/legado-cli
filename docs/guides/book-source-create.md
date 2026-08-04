# Book-source create checklist (找站后做源)

Agent entry: skill `legado-book-source`. Repair of existing URLs stays in `legado-book-source-repair`.

## CLI binary (Windows)

`source-cli` is often **not** on PATH. Prefer an absolute path:

```bash
# from repo (Cargo writes under crates/target when built from crates/)
E:/Projects/legadoSkill/crates/target/debug/source-cli.exe --help

# rebuild
cd E:/Projects/legadoSkill/crates && cargo build -p source_cli

# optional install onto cargo bin (then `source-cli` works in new shells)
cargo install --path E:/Projects/legadoSkill/crates/source-cli --force
```

Never invent a second `--target-dir`. MCP URL/token: `config/mcp_defaults.json`.

## Fast path

```
1) source-cli check channel
2) Fetch raw HTML (PC curl / source-cli fetch) — not DevTools DOM
3) Draft JSON → temp/full_fix/cache/new_sources/<host>.json
4) source-cli source push --file …          # not IDE save_source paste
5) debug_source keyword → if list=0: HTTP log first
6) debug detail/toc/content (see debug keys below)
7) start_check_sources checkDiscovery=false, timeoutMs≥90000 (slow≥180000)
8) Only claim done on 校验成功
```

Timeouts SOT: `config/mcp_defaults.json` (`debug_timeout_s`, `verify_timeout_ms`, `http_timeout_s`).
MCP `debug_source` may pass `timeoutSec` to override; check may pass `timeoutMs`.

## CookieJar decision (do not guess)

| Signal | enabledCookieJar | Action |
|--------|------------------|--------|
| HTTP body `搜索间隔` / Cookie `ss_search_delay` | **false** | `check clear-cookies --url …` then retest; optional searchUrl `@js` `cookie.removeCookie` |
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
| `ss_search_delay_cookie` | list=0; alert 搜索间隔 | clear-cookies + jar false |
| `cookiejar_cf_needs_on` | CF search empty without jar/webView | jar true + webView; else skip |
| `multi_toc_pick_longest` | TOC only latest N | @js max link-count container |
| `relative_ajax_toc` | toc empty; ajax_index relative | baseUrl + path |
| `desktop_empty_mobile_content` | PC content empty; m. OK | hybrid chapter URL |
| `debug_colon_explore` | used `::URL` for detail | absolute / `++` URL |
| `check_keyword_too_broad` | first hit bad book | rarer checkKeyWord |
| `ide_save_escape` | save JSON broken | `source push --file` |
| CF Turnstile | webView still fail | skip / manual loginUrl |
| CLI not found | `source-cli: command not found` | use `crates/target/debug/source-cli.exe` |

## Related

- Discovery / 找站: `docs/guides/book-source-discovery.md`
- Skill: `legado-book-source`
- Repair: `legado-book-source-repair`
- MCP: `config/mcp_defaults.json`
