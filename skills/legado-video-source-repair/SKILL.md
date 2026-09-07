---
name: legado-video-source-repair
description: >-
  Repair Legado 视频/影视/听书-style sources (bookSourceType video/audio),
  not novel HTML TOC sources. Use for 影视站, M3U8, 网盘片库, taopian, ukuzy,
  when check fails with 下载链接为空 on media sites.
---

# Legado Video / Media Source Repair

**Separate from** `legado-book-source-repair` (小说搜→详→目→文).

Media sources fail novel check with 「下载链接为空」 by design if rules target
M3U8 / magnet / drive links. Do not force novel TOC patches on them.

## When to use

- Host/name looks like 影视 / 资源网 / M3U8 / 片库
- `bookSourceType` is **3 (file/下载)** or **4 (video)** — not novel text `0`
- Check message: 下载链接为空, explore-only media catalogs

## Flow

```
1. Classify via CLI: source-cli video-route --url <URL>
   If classified as video/audio/file → divert from novel TOC repair flow.
2. Inspect source via MCP: confirm bookSourceType is 3 (file) or 4 (video); check missing downloadUrls.
3. Fix search bookUrl (must point to detail page) + downloadUrls / m3u8 playlist rules.
4. Verify on device: debug_source; run `source-cli debug-vs-check` if debug differs from check; then run check.
5. Closeout: `source-cli ledger append --url <URL> --step check --result "校验成功"`
```

## Done criteria (video/file)

- Device `check_source` returns **校验成功**, or `debug_source` confirms non-empty `downloadUrls` / m3u8 stream.
- Common fix for type=3: set `ruleBookInfo.downloadUrls` (e.g. `input[name=copy_sel]@value`) and ensure search `bookUrl` is a **detail** URL (not search page), or infoHtml hijacks detail parse.
- If debug has m3u8 but check says 下载链接为空 → do not thrash CSS `||`; run `source-cli debug-vs-check`.

## Do not

- Apply novel TOC patches or clear `tocUrl` blindly on media sources.
- Put taopian/uku into novel-only verify batches without file rules.
- Claim fixed without device verify.

Configuration SOT: `config/mcp_defaults.json`.
Novel repair skill: `legado-book-source-repair`.
Domain migration CLI: `source-cli migrate --from-url <OLD> --to-url <NEW>`.
