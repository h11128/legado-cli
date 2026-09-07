# Book-source discovery (找站 → 做源)

How to find sites and create Legado book sources efficiently.
Repair of *existing* failing sources stays in `legado-book-source-repair` + `source-cli repair`.

**CLI SOT:** `source-cli` only (no new Python repair scripts).

## When to use

- User asks for 出版 / 公版 / 古籍 / 找书 / “帮我找网站做书源”
- Need a **new** source category, not fix an enabled URL

## Fast path (default)

```
1) source-cli check channel          # must idle
2) New sites: source-cli site-probe --url https://a.com --url https://b.com \
     --out temp/full_fix/queues/site_probe.json
   Or paste a list: --urls-file urls.txt   (lines: url|kind|note)
   Ledger / triage known 出版 seeds (mostly static buckets):
     source-cli site-probe --preset publish --out temp/full_fix/queues/site_probe_publish.json
   Recheck already-sourced rows: add --include-sourced
3) Pick rows with discovery=make_candidate (ignore already_sourced unless improving)
4) source-cli fetch --url … --dump-dir temp/full_fix/cache/html
5) source-cli probe --base-url … --html-file … --key <keyword>   # optional form/rank hint
6) Draft BookSource JSON from raw HTML → `temp/full_fix/cache/new_sources/<host>.json`
7) `source-cli source push --file …` → debug_source → start_check_sources (checkDiscovery=false)
   If search list=0: HTTP log first; `搜索间隔`/`ss_search_delay` → `check clear-cookies` + jar off
8) Tag bookSourceGroup; append seed to preset JSON; ledger/retro if needed
```

**Create deep checklist:** [`book-source-create.md`](./book-source-create.md) (CookieJar table, `::` vs absolute debug, multi-TOC, hybrid m., CLI binary path).

Wall budget: probe batch ≤1–2 min; one new source draft+verify ≤5–8 min. Skip VPN/App/PDF early.
If search empty burns >2 min on throttle cookies — clear cookies once, then one verify; do not rewrite selectors on alert HTML.

CLI binary (often not on PATH): `crates/target/debug/source-cli.exe` or `cargo install --path crates/source-cli --force`.

**Note:** `config/site_candidates_publish.json` is a curated ledger (`sourced` / `skip_*`).
Empty `make_candidate` on `--preset publish` alone is expected until you add `status=candidate`
rows or pass new `--url` / `--urls-file` inputs.

## Discovery buckets (`site-probe`)

| `discovery` | Meaning | Agent action |
|-------------|---------|--------------|
| `make_candidate` | L0–L2 pass | Fetch HTML → draft source |
| `skip_waf` | CF / challenge / wall | Do not draft selectors |
| `skip_dead` | L1/L2 deadish / unreachable | Hunt only if user insists |
| `skip_app` | App/login storefront | Catalog only; no正文源 |
| `skip_pdf` | OA/PDF portals | Optional browser-open helper later |
| `skip_vpn` | Needs proxy / DNS fail | Skip unless user has magic |
| `skip_catalog` | Metadata only (Douban…) | Not a reading source |
| `already_sourced` | Preset marks `status=sourced` | Improve only if broken |

Preset file: [`config/site_candidates_publish.json`](../../config/site_candidates_publish.json).

## Kind → Legado shape

| `kind` | Typical type | Notes |
|--------|--------------|-------|
| `public_domain_en` | 0 | Gutenberg-style: search list + one 全文 HTML chapter |
| `classics_zh` | 0 | ctext: absolute chapter URLs; `td.ctext` body |
| `mixed_publish` | 0 | yodu: POST search + 经典文学 explore |
| `download_finder` | 0/3 | jiumo: content = browser download tip |
| `illustrated_classics` | 2 | 名著 API / comic-like |
| `oa_academic` / `app_store` | — | Prefer skip for 正文 |

## Traps (from 2026-07-30 出版 session)

| Trap | Signal | Fix |
|------|--------|-----|
| `preserveOrderWeight` | MCP save 不改 `customOrder` | `preserveOrderWeight=false` (App) or delete→save |
| Relative TOC on path base | `analects` + `analects/xue-er` → doubled path | Push **absolute** `https://ctext.org/...` in `@js` TOC |
| JSON TOC `@js` + `$.text` | 目录列表大小 0 | Prefer `{name,url}` objects + `chapterName=name` |
| Gutenberg multi-h2 | Full book one HTML | Single 全文 chapter → `*-images.html` |
| Site has no search | kanunu | Explore-only; don't fake searchUrl to a writer page |
| CF / Just a moment | 99csw | `skip_waf` — never rewrite selectors on challenge HTML |
| 需魔法 | annas-archive | `skip_vpn` |
| Catalog has no search | xuges / tianyabooks | `searchUrl` `@js: source.put('sk', key)` then `bookList` `@js` filter with `source.get('sk')` — `key` is **not** bound in AnalyzeRule |
| GBK sites | xuges / tianyabooks | URL option `,{"charset":"GBK"}` |
| TY empty chapters | newer /world/ shells | Prefer older books with `#neirong`; skip stub first chapters |
| `ss_search_delay` / 搜索间隔 | list=0; HTTP body is alert script | `check clear-cookies` + `enabledCookieJar=false`; never rewrite bookList |
| Multi `list-charts` | TOC only latest N chapters | `@js` pick ul with max `li>a` |
| IDE save_source escape | Truncated/broken JSON on MCP call | `source-cli source push --file` |
| CF Turnstile search | uukanshu-style | skip or manual loginUrl; do not fake selectors |
| CookieJar opposite cases | throttle vs CF | see `book-source-create.md` decision table |
| Relative ajax toc | sto55-style | `baseUrl + 'ajax_index.html'` |
| Desktop empty / m. OK | xsw-style | hybrid chapter URL |
| `::URL` debug | treated as 发现 | use absolute / `++` for detail |
| Broad checkKeyWord | first hit junk | rare title fragment |
| source-cli missing | not on PATH | `crates/target/debug/source-cli.exe` |

## Related

1. **Batch reachability first** (`site-probe`), never serially curl 20 sites in the agent loop.
2. **Reuse phone sources**: `list_sources --search 出版|公版|名著|古登堡` before drafting twins.
3. **One phone MCP job** at a time; parallelize only PC gate/fetch.
4. **Keyword by kind**: Gutenberg `Alice`; ctext `论语`; mixed `红楼` / `三体`.
5. **Do not claim fixed/created** without device `校验成功` / debug content non-empty.
6. After a useful new site class, append one row to the preset JSON (`status` + `note`).

## Links

- Create checklist: `docs/guides/book-source-create.md`
- Create skill: `legado-book-source` (Phases 1–3 + this doc link)
- Repair skill: `legado-book-source-repair`
- MCP SOT: `config/mcp_defaults.json`
- Session note: `docs/postmortem/source-repair-retrospective.md` §129 / §131
