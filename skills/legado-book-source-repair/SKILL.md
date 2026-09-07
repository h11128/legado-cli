---
name: legado-book-source-repair
description: >-
  Repair failing Legado (阅读) book sources after check/debug failures.
  Default path: source-cli dig (or diagnose → repair oneshot) → device verify →
  ledger/retro. Use when fixing 书源, 校验失败, 搜索失效, 目录失效, 正文失效,
  tocUrl bugs. MCP save_source/debug is fallback only (diagnose transport fail).
---

# Legado Book Source Repair

Device MCP is authoritative for verify. Enforce `.cursor/rules/book-source-repair-discipline.mdc`.

**Agents MUST follow the Deep-fix checklist in order.** Entry command:

```
source-cli dig --url URL --key 我的
```

(`dig` = channel → gate → diagnose → oneshot unless `--no-repair`). Do **not** start with
ad-hoc Python HTML + `LegadoMcp.save_source` unless diagnose transport failed.

After each fix/skip, refine skill/scripts if a new trap appeared, then continue the goal loop.
Track: `source-cli progress status`.

| Doc | Path |
|-----|------|
| MCP defaults (SOT) | `config/mcp_defaults.json` |
| MCP discover | `source-cli discover --write` |
| Platform (Rust) | `docs/reference/repair-adapter-architecture.md` — **full Rust cutover 2026-07-28** |
| Anti-stall matrix | `docs/reference/deep-diagnose-anti-stall.md` |

**Entry:** **`source-cli` only** — no Python shims. Build: `(cd crates && cargo build -p source_cli)`.

**Do not hard-code phone IPs in prompts or skill text.** Scripts call `ensure_session`,
which rediscovers on connect failure and updates SOT. Agents must not ask the user to
“手动改 IP / 手动 rediscover” unless Cursor IDE MCP is still stale after discover
(then: reload MCP / restart agent once).

## Goal loop (toward 100)

```
while fixed_n < 100:
  A) source-cli queue refresh-index
  B) source-cli queue rt --group 搜索失效
  C) source-cli serial --urls-file … --limit 100
  D) new trap? → update skill+scripts BEFORE next batch
```

**Serial efficiency rules (mandatory):**
1. Queue only from **phone index** (not stale `temp_tagged_fails` alone).
2. **No empty-probe verify** — `require_patch=True`；探针无补丁 → `no_patch_skip`（不烧设备校验）。
3. **One host one candidate** in queue; scheme-less URLs normalized; `host_key` works without `http://`.
4. migrate_to must pass L2; `search_endpoint_dead` → skip.
5. **Dead/timeout → hunt before disable** — L2 `l1_unreachable` / `l2_http_dead` → `action=hunt`；`source-cli repair` / `hunt --probe` 查 seeds；有活后继 → migrate+verify；`shutdown`/无种子/探针全死 → 才 disable。确认关站（L0 `dead_site_shutdown_confirmed`）与域名停车/广告劫持 **不** hunt。
6. After each URL: `source-cli retro append` + ledger；新 trap 立刻写 skill。

**Anti-pattern (banned):** classify/probe 20–50 tagged fails to “find a good one”.
That burned minutes and violated the 2–3 min budget. Pick → diagnose → patch → verify.
**Also banned:** batch/oneshot 对 `l2_http_dead` / timeout **直接 disable** 而不跑 hunt（见 trap `dead_skip_without_hunt`）。
**Also banned:** 仅凭 gate/serial/hunt-empty/「搜索失效」标签口头「修不了」收工（见 trap `shallow_unfixable_claim`）；批次 deep dig 结束必须写 `docs/postmortem/source-repair-retrospective.md` 总教训，不只 per-URL retro。

## Report modes (both supported)

| Mode | When | Command / agent behavior |
|------|------|---------------------------|
| **oneshot** 修一个报一个 | 默认深修、用户要盯进度 | `source-cli diagnose` → `source-cli repair --mode oneshot --url URL`。必须走 layer；假详情不修 toc |
| **batch** 批量 | 用户明确说「批量 / 做 N 个」 | `source-cli repair --mode batch --urls-file … --limit N`；勿整批 AwaitShell |
| **serial** 串行 | 批量深修队列 | `source-cli serial --urls-file … --limit N`（默认 `--url-timeout-s 120` 子进程杀挂）；**禁止** AwaitShell 等整批结束；短轮询 `temp/full_fix/serial_heartbeat.json` / `serial_last.json` mtime |

## Deep-fix checklist (one URL)

Budget clock starts at **pick**. Diagnose+patch **2–3 min**; hard stop **5 min**.

```
[ ] 0  channel idle
[ ] 0b preferred: source-cli dig --url URL   # channel+gate+diagnose+oneshot
[ ] 1  progress next  (script L2-gates walls/parked; ≤~20s)  OR shelf remaining pick
[ ] 2  if next.l2_gate.action=migrate → migrate first
[ ] 2b if action=hunt (l1_unreachable / l2_http_dead / L0 timeout_cluster)
       OR brand may have migrated (parked/广告壳但仍可能换域) →
       A) `source-cli hunt --url … --probe`（repair oneshot 已自动跑）
       B) **OSINT successor search**：
          使用搜索引擎（Google/Bing/百度）检索目标小说站名与最新替代域名。
       C) migrate | disable(no_mirror/none_alive/empty) | skip(weak)
[ ] 3  diagnose --url URL   # also L2-failfast BEFORE phone debug（dig 已含）
[ ] 4  if layer=skip → ledger already done → close-out (§ below) → **立刻汇报**
[ ] 5  else patch ONLY layer → ONE verify → ledger
[ ] 6  close-out: ledger → retro（自动 gate/sync）→ **git commit skill/scripts/docs** → progress next
```

Shelf stale-tag: after picking from `shelf-stale-tag-remaining.md` / triage, still run
steps 0/0b–6 — **same** path, not MCP-first.
## Per-URL close-out (mandatory)

User standing preference (this repo): **every** oneshot (fixed / skip / fail) must:

1. **Document** — append ledger; short note in `docs/postmortem/source-repair-retrospective.md`.
2. **Reflect** — `source-cli retro append --url … --status … --trap … --script-fix …`
3. **Improve** — decision tree below (gate enforces it).

### Improve decision tree (enforced)

```
trap 已在 SKILL Traps / known:… ?
├─ YES → skill_fix=0；可不改 Rust（script_fix 可写 MCP 手工补丁说明）
└─ NO（novel）→ 必须同时完成，再 retro：
   A) SKILL Traps 加一行（Action 含 Harness: 落点或 no_auto）
   B) 改 harness + 测，或显式放弃自动修：
      · 规则可自动修 → source_patch（smell / apply_safe_rule_fixes）
      · 诊断会误导 → diagnose_tips（或 L2 / probe）
      · 仅人工例外 → script_fix="no_auto:<≥8字理由>"
   C) retro --skill-fix --script-fix 'source_patch/…'（或 no_auto:…）
```

**Gate（写死，不是散文）：** `skill_fix=1` 时 `--script-fix` 必须命中
`source_patch` / `diagnose_tips` / `crates/…` 等，或 `no_auto:<理由>`。
只加 SKILL 行、script_fix 写「MCP save…」→ `retro append` / `closeout pending` **拒绝**。

```bash
source-cli closeout gate --trap SLUG --skill-fix --script-fix 'source_patch/smells.rs'
source-cli retro append --url URL --status fixed --trap SLUG \
  --script-fix 'source_patch/smells.rs:…' --skill-fix
source-cli progress next   # 先跑 closeout pending
```

## Traps

| Trap | Signal | Action |
|------|--------|--------|
| **empire_cms_search_path** | list=0 but 浏览器手动提交表单能搜到; searchUrl 是 `/e/search/index.php` | 帝国CMS(EmpireCMS)真实搜索脚本是 `/e/search/indexsearch.php`（多 "search" 几个字），不是 `index.php`；改路径，不动 selector。Harness：`no_auto:empire_cms_search_path` |
| **booksourceurl_typo_masked_by_absolute_url** | bookSourceUrl 域名打错但 debug 仍能搜到 | searchUrl/exploreUrl 用了绝对 URL 绕过了错误域名，掩盖问题；用 curl/探针分别测两个域名确认哪个真实可解析，订正 bookSourceUrl 后 save+delete 旧记录。Harness：`no_auto:booksourceurl_typo_masked_by_absolute_url` |
| **source_cli_target_profile** | source-cli command not found | 检查 `crates/target/release/source-cli.exe` 和 `crates/target/debug/…`（构建可能只在 release 模式下），避免误判未安装。Harness：`no_auto:check_release_debug_profile` |
| **stream_protocol_error_reset** | 手机校验/HTTP 日志 `stream was reset: PROTOCOL_ERROR` / StreamResetException；http/https 同失败；hunt empty | **skip/disable** — 传输层挂，非选择器。Harness：`no_auto:transport_dead` |
| **tls_packet_header_corrupt** | 备注/日志 `Unable to parse TLS packet header` / Cronet `ERR_SSL_PROTOCOL_ERROR`；PC `SSL: record layer failure` / L2 `corrupt message`；TCP:443 通但握手崩；hunt empty | **skip/disable** — 源站 TLS 损坏，非选择器；勿只改 http。Harness：`no_auto:tls_origin_corrupt` |
| **cert_expired_phone_ok** | PC L2/`ureq` `certificate expired` / `invalid peer certificate` → 误 `l2_http_dead`→hunt_empty→oneshot disable；但手机 OkHttp 仍 200，搜索/目录/正文可读（红叶书斋 shufahouse） | **勿**只凭 PC 证书过期禁用。先 `debug_source`/校验；清陈旧「搜索失效」。Harness：`source-gate/classify.rs` → `l2_cert_expired` + `Verify` |
| **content_font_encrypt_dynamic** | 后继站详情/目录 OK，但正文 `#nr1` 为 `font-family:YHFixed` + `&#13xxxx` 实体；每章 `/style/_*.css` 内嵌不同 woff2（cmap PUA→uniXXXX 会变）；`webJs`/裸 `@text` 仍乱码 | **勿**只凭详情/目录宣称 fixed。无 Rhino 可跑的 woff2/cmap 解法则 **skip/disable**+换源；PC 可证 fontTools 解得通≠可进书源。Harness：`no_auto:needs_woff2_cmap_in_rhino` |
| **hunt_oneshot_mutates_wrong_twin** | dig/oneshot 对死 IP/`hunt→migrate` 时 `set searchUrl` 打到**已有活孪生**（常是 `m.host/` slash）并把 POST 搜改成坏的 GET，源校验失败 | migrate 后立刻 `get_source` 核对 **目标 URL**；勿让 oneshot 改 keeper。书架用 `legado-db-mutate remap`；坏 twin **disable**。Harness：`no_auto:verify_target_url_after_migrate` |
| **ip_url_host_header_parked** | `bookSourceUrl` 为裸 IP；`header.Host` 指向域名；IP 超时且 Host 域是「官网首页」/停车壳无小说 | **disable**；勿只换 Host。hunt 无后继则 skip。Harness：`no_auto:disable_ip_shell` |
| **cf_520_origin_error_hunt_empty** | 首页/搜索 Cloudflare **520 Origin Error**；hunt empty；已有活孪生（如 69shuba.com） | disable 死域；书架 remap/换源到孪生；勿抠选择器。Harness：`no_auto:migrate_or_disable` |
| **http_403_home_hunt_empty** | 首页 GET 403（手机 HTTP 日志）；searchUrl `@js`/`ajax` 抽 form 崩；`hunt --probe` empty；同名域停车/威胁页 | **skip/disable** — 非选择器问题。Harness：`no_auto:hunt_then_disable` |
| **manual_mcp_bypass_closeout** | Agent 用 `LegadoMcp.debug/save/check` 或 IDE MCP 深挖，却不跑 `diagnose`/`dig`，且想 `retro fixed` | **默认禁止当主路径**。仅 diagnose 传输失败或用户明示。`deep_active.entry=mcp_fallback`；`retro fixed` 须 trap 含 `manual_mcp_bypass` + `script_fix=no_auto:diagnose_transport…`/`no_auto:user_…`。Harness：`gate_fixed_diagnose_path` + `legado_mcp` claim `--entry mcp_fallback` |
| **diagnose_path_skipped** | 书架/深挖直接 PC HTML + MCP patch，跳过 `dig`/`diagnose` | **MUST** `source-cli dig` 或 diagnose→oneshot。Harness：`source-cli dig`；hook ASK `legado_mcp_without_diagnose_ask` |
| **host_phone_timeout_no_mirror** | PC 与手机均连不上（Cronet/URL 超时）；`hunt --probe` empty | 已 hunt 仍无后继 → disable/skip；勿反复 debug。Harness：`no_auto:hunt_then_disable` |
| **dns_nxdomain_hunt_empty** | PC `NXDOMAIN` / 手机 `UnknownHostException`；`hunt --probe` empty；假镜像为 XDNS 威胁页或影视壳 | **disable/skip**；勿当超时反复 debug；有真小说孪生且路径可开再 migrate+remap。Harness：`no_auto:hunt_then_disable` |
| **adb_db_push_stale_snapshot_wipes_source** | MCP `save_source(新URL)` 已成功，但随后 `push_legado_db` 用**更早 pull**（或缺 WAL checkpoint）的整库覆盖 → 新源从 `book_sources` 消失；或 `get_source` 仍像活着但 SQL 无行 | **禁止** MCP-save 后推旧快照。用 `legado-db-mutate` / 同文件 upsert + `require_source_urls`。`push` 默认 `merge_live_sources="auto"`（已在工作库→跳过；否则 MCP upsert；再否则 baseline 全量 merge）。校验 `wait_check_done`；PC probe≤12s。应对：严禁用陈旧快照覆盖手机活跃数据库。 |
| **home_404_paths_alive** | 首页 GET 404（或仅数百字节）；但 `/book/…`、`/plus/search.php` 等同站路径 200 且 HTML 含 `og:novel`/`cont-body` | **勿**只凭首页判死。PC 再探针搜索+一本详情/目录/正文；可迁 `bookSourceUrl` 到活孪生并 remap。Harness：`source-gate/sniff.rs` 不再把裸 `404 Not Found` 当 parked（→ L2 Hunt）；`no_auto:probe_book_and_search_paths` |
| **placeholder_web_accesible_google** | 首页 title=`Web accesible`（或西语「La web está accesible」）+ `meta refresh`→Google；书/搜索路径 404 | **disable/skip** — 占位壳非小说站。同名孪生若 CF 522/超时勿当可迁。Harness：`no_auto:placeholder_disable` |
| **name_similar_video_not_novel_twin** | 原站超时；同名 `.com` 等可开但是影视/视频壳（标题含影院/电影）；hunt 无小说候选 | **勿**迁到影视站。`skip`+disable；靠自动换源。Harness：`no_auto:title_sniff_video` |
| **search_empty_shell_open_ok** | `search.html` 200 但无结果节点（`#sitembox`/`dl` 空）；混淆字段（如 `369koolearn`）POST 仍空壳；详情 `#list`+`#content` 可读 | **勿**浅判整源死。修/保留打开路径；验证用 `checkSearch=false`+`checkDiscovery=true`（有 explore）或 `debug_source(bookUrl)`；**禁止** search+discovery 双关（见下条）；有活 `m.` 孪生则优先；假搜索勿用热门按钮当 bookList。Harness：`no_auto:open_path_verify` |
| **check_search_discovery_both_off_vacuous** | MCP `start_check_sources` 同时 `checkSearch=false`+`checkDiscovery=false`（且 `checkDomain=false`）→ `durationMs`≈1–10 仍报「校验成功」 | **假成功**：`BookSourceCheckRunner.doCheckSource` 未拉书。无搜索时必须开发现，或只用 `debug_source` 打开路径当证据。Harness：runner 拒「校验项为空」；`legado_mcp` 强制 `checkDiscovery=true` |
| **shallow_unfixable_claim** | Agent 仅凭 `gate`/`serial`/`hunt empty`/「搜索失效」标签口头判「修不了」；或把 `action=hunt` 缓修说成 skip；用户再深挖又能迁域/修打开路径 | **禁止**浅层终局。收工前至少：PC 首页+搜索+一本 TOC/正文，或手机 `debug_source`+`get_http_logs`；hunt 桶必须先 probe。搜索死仍要看打开路径；批次结束写 `docs/postmortem/source-repair-retrospective.md` 总教训。Harness：`no_auto:agent_must_html_or_phone_debug` |
| **content_qsbs_bb_base64** | 正文章节 HTML 含 `qsbs.bb('…base64…')`；`##…##@js:base64Decode` 易截断触发 Hutool AIOOBE | `ruleContent.content` 用 `@js`：`indexOf("qsbs.bb('")`→`substring`→`java.base64Decode`；搜索若 meta refresh 回首页则 `checkSearch=false` 或 disable §16。Harness：`no_auto:按正文脚本改` |
| **toc_href_slash_twin_unreachable** | 详情 TOC 几乎全是 `href="/"`（仅最新章真链）；PC 孪生（如 `biquge5200.cc`/`b5200.org`）目录/搜索 OK，但手机 Cronet 对 `23.224.*` 60s timeout | **勿**浅判「站点活着就能修」；手机不可达孪生 → `skip`+留证据；可达再 migrate。Harness：`no_auto:PC探针+手机HTTP日志` |
| 假详情 (wmp8) | list-empty + books≤1 + `/s.php` | **search** |
| **empty_search_detail_fallback_h1** | debug：`列表为空,按详情页解析` → 书名=`…搜索结果` / `Books: {{key}}`；bookUrl=搜索 URL | `ruleBookInfo.name` 勿用裸 `h1@text`；改成详情页专属（如 `h1.article-title` / `class.book-title`）。空搜应 `书籍总数:0`。**更通用的替代修法**：直接设 `bookUrlPattern`（真实详情页 URL 正则，`BookList.kt` 用 `.matches()` 全串匹配）——只要设了非空值，`collections.isEmpty()` 就不再触发按详情页解析这条回退，`ruleBookInfo` 完全不用动；只有当前 URL 真的匹配这个正则时才会走详情页解析（8biquge.com/m.sinodan.link/manga18fx.com 三源用此法修）。Harness：`no_auto:按站改详情选择器`或`no_auto:set_bookUrlPattern` |
| **search_notfound_row_matches_bookList** | debug：`列表大小:1`（非空）→ 书名=页面里"没有找到/没有相关信息"提示文案；author 等字段皆空 | 与上一条不同——这里 `bookList` 选择器命中的不是 0 个而是 1 个：网站把提示文案也渲染成跟真实结果同结构的一行（如 DedeCMS `<tbody><tr><td>没有找到包含：xxx的小说</td></tr></tbody>`，提示行没有 `<a>`）。修 `bookList` 本身，加 `:has(a)` 只留有真实链接的行，如 `tbody@tr:has(a)`；**注意** Legado 的 `tag.` 前缀写法（`tag.tr:has(a)`）不能正确配合 `:has()`，会把真实结果也一并过滤掉（列表大小变 0），只能用裸标签名 `tr:has(a)`（夜伴书屋 yybsw.com/yeban360.com 两源用此法修，同一后端两个镜像域名）。Harness：`no_auto:bookList加has(a)过滤提示行` |
| **search_author_highlight_span** | 搜索作者=`{{key}}`；HTML 标题内 `<span style=color>` 高亮关键词，`span.0` 误当作者 | `ruleSearch.author` 改 `span[itemprop=author]` / 真作者节点，勿用标题内第一个 span。Harness：`no_auto:按DOM改author` |
| **txt_header_author_no_html_field** | 搜索/详情 HTML **无作者字段**（仅上传者）；但 `down.php?bid=` TXT 全本文件头有 `作者：xxx`（需 Referer=详情页，`name=NULL` 亦可） | `ruleBookInfo.author`=`@js`：`java.cacheFile(downUrl+Referer)` 解析文件头；**勿**对搜索列表每条拉 TXT（数 MB）。搜索列表仍可能空作者。Harness：`no_auto:按站拼down.php+Referer` |
| **meta_author_content** | 详情可见区无作者/作者选择器空，但 `<meta name="author" content="…">`（或 og:novel:author）有值 | `ruleBookInfo.author`=`meta[name=author]@content`（可 `\|\|` 备选）。书名带《》时加 `##^《\|》$`。Harness：`no_auto:按站读meta` |
| **author_zhu_suffix** | 作者=`xxx著`（著作后缀） | **勿**写成 `…##前缀规则##著$`（第二条会被当成替换值）。用单条捕获：`##.*作者：\\s*(.*?)著?\\s*$##$1` 或仅 `##著$`（若前缀已干净）。Harness：`no_auto:去著后缀` |
| **name_format_noise** | 书名带 `《》` / `[分类]` / `最新章节` 后缀 | 单条删除：`##^《\|》$` / `##^\\[.*?\\]` / `##最新章节$`；**勿**写死具体分类名如 `##^\\[其他\\]`——换个分类（言情/都市/…）就漏网，同一批同库源常见（墨坛文学/云轩阁/永久小说网/天书小说网同坑）；勿多段 `##a##b` 误当替换。Harness：`no_auto:书名格式清洗` |
| **lofter_tuiwen_as_book** | Lofter 搜索命中标题以 `推文` 开头（推文当书） | `ruleSearch.name` 的 `<js>` 里：`if(/^推文/.test(result)) result=""`（或从 bookList 过滤）。Harness：`no_auto:丢弃推文标题` |
| **audiobook_announcer_in_author** | 听书源作者=`{{$.author}} 演播：{{$.announcer}}`（演播拼进作者） | `ruleSearch/ruleBookInfo.author` 改为 `$.author`（演播可另放 `kind`/`intro`）。Harness：`no_auto:去掉演播拼接` |
| **ximalaya_author_is_anchor** | 喜马拉雅搜索/详情 API 只有 `nickname`/`anchorName`（主播），无原著作者字段 | **无法**修成小说作者；作者=主播是平台数据。勿把 nickname 当解析错误反复抠。Harness：`no_auto:平台无原著作者字段` |
| 真 TOC (画本) | search≥2 + 目录空 + real detail | tocUrl/ruleToc |
| 假「假详情」 | search≥2 but log shows search URL first | still toc/content |
| 空 tocUrl + JSON (长佩) | `$.data.list` + empty tocUrl | chapter API tocUrl |
| webView 单引号 (长佩) | `{'webView': true}` | `{"webView":true}` |
| debug=ok / tagged fail | flake or 发现-only | harvest/verify; don't over-patch |
| API 目录要登录 | 认证失败 / device 必填 | **skip** |
| 验证码搜索 | getcode / yzm / actyzm | **skip** |
| 域名停车/过期 | L2 GET 正文含 for sale/出售/域名到期 / Redirecting shell | **disable/skip**（勿当搜索规则坏；**不** hunt） |
| **dead_skip_without_hunt** | batch/agent 对 `l1_unreachable`/`l2_http_dead` 直接 disable | **禁止** — 先 `hunt --probe` / oneshot 自动 hunt；无后继再 disable。Harness：`classify.rs`→`Hunt`；`oneshot_live` resolve；wave 不把 hunt ledger 成 final skip |
| **gate_hunt_deferred_unprobed** | 分流把 `gate_action=hunt` / DNS `Unable to resolve host` 丢进「maybe later / 像 skip」却**不跑** `hunt --probe`（tongrenquan：www 无 A，种子已有 `m.`） | **禁止** — triage 时 hunt 必须当场 probe；有 migrate/verify 候选 → **立刻升优先**深挖，勿口头缓修。Harness：`source-cli hunt --probe`；`no_auto:triage_must_hunt_probe` |
| **hunt_osint_skipped** | 只跑 `hunt --probe` empty 就结案；不做 Google / crt.sh / 限流 Wayback /（有 key 时）付费 DNS 档案；或对 archive.org 连发触发 429 | **禁止** — deep dig 迁域嫌疑必须跑 `source-cli hunt --url … --probe` + 浏览器 Google（书名+站名）。Wayback 查询需加并发限制。Harness：`no_auto:osint_then_disable` |
| **serial_await_idle** | Agent 对整批 `serial`/`batch` 长 AwaitShell（数小时） | **禁止** — 最多短轮询 60–90s；看 `serial_heartbeat.json` / `serial_last.json` mtime；心跳停滞 > `url-timeout-s+30` → kill 父进程、`check channel --force-clear`、续跑。Harness：`serial_cmd`/`serial_spawn` |
| **agent_turn_stall** | 后台 diagnose 后收工；或 login/AES/广告源反复抠 >2min；或 MCP `10060` 堵死整环 | **禁止** — 回合结束前必须 close-out 当前 URL 或写明下一动作；auth/广告证据够就 seal；diagnose 传输失败 → PC probe + 直连 MCP debug/check，或 `skip:mcp_transient` 下一源。**Harness：** `deep_active.json` claim；`closeout pending`/`progress next` 未 seal 则拒；`closeout release`；Discipline §21–23 |
| **hedged_ledger_success** | ledger `校验成功或见上` / `见上` / 假成功（migrate verify_ok=false 仍记成功） | **禁止** — `ledger append` 硬拒；pending 拒；goal 不计。只写精确 `校验成功` / `skip:…` / `fail:…`。Harness：`ledger_gate.rs` |
| **kujiang_header_split** | 全局带 `app/platform/version` 时 search 空体；catalog 无这些头则「版本不再支持」；带过期 `auth-code` 也空 | 源 header 只用 `User-Agent:KuJiang/…`；`tocUrl`/`chapterUrl` 选项里单独加 app 头（**不要** auth-code）。Harness：`no_auto:按接口分头` |
| **serial_url_timeout** | 单源 MCP/debug 卡住超墙钟 | serial 杀子进程 → `skip:url_timeout` + ledger；继续下一 URL。Harness：`--url-timeout-s`（默认 120） |
| **mcp_lock_zombie** | Windows 死 PID 仍占 `mcp_channel.lock`（旧实现永远 alive） | `check channel` 自动清；repair stale **15m**；Win32 `OpenProcess`。Harness：`channel.rs`/`channel_pid.rs`；卡死活进程用 `--force-clear` |
| **mcp_timeout_sot** | 超时写死在代码 / 找不到配置 | 改 `config/mcp_defaults.json`：`http_timeout_s`（默认90）、`debug_timeout_s`（默认45，仅 `debug_source`）、`verify_timeout_ms` / `verify_max_wait_s`。Harness：`timeouts.rs`；discover 重写 URL 会保留这些字段 |
| 主机跳转 | bookSourceUrl host ≠ final host（如 .org→.com） | **migrate** 再修搜索 |
| **migrate_false_friend_cms** | gate `l2_host_redirect`/301 新域通，但是**无关 CMS**（如 Z-Blog `zb_users`/`San_Media`）；旧 `/novel_*`、`modules/article/search.php` **404**；OSINT 真后继（如 `.net`）手机/PC **CF 522** | **勿**按「主机跳转」盲目 migrate。先嗅新域模板/旧路径；假友 → disable；真后继不可达 → skip+换源。Harness：`no_auto:sniff_cms_before_migrate` |
| **meta_search_third_party_toc_dead** | 书源是夸克/神马等 **SERP 聚合**；新 DOM（`qk-title-text`）可搜出书，但 `bookUrl` 落在随机第三方（403/`TocEmpty`）；书架常是搜索 URL/起点搜书页 | **勿**只修选择器当 fixed。无稳定本站详情/目录 → **skip/disable**+换源。Harness：`no_auto:aggregator_disable` |
| **waf_401_meta_refresh_cluster** | 首页/详情/搜索均 **HTTP 401**，体为 `<meta http-equiv=refresh content=0>` 或 `loading host` 旋转壳；`_wa_`/`p2d` cookie 后仍 401；同库孪生（mozhua/2wxh/juqisw/dizhuwu）一锅端；偶发手机 **TLS packet header** | **skip/disable** — WAF/需登录，非选择器。webView 仍 401 勿死磕。Harness：`no_auto:waf401_disable` |
| **desktop_ua_blocked_mobile_ok** | HTTP 日志 `403` + 正文 `The User-Agent has been blocked`；PC/默认桌面 Chrome UA 失败，Android Mobile UA 同路径 200 | header 改移动 UA；再验打开/发现。勿当整站死。Harness：`no_auto:set_mobile_ua` |
| **js_loading_jwt_ad_hijack** | 首页/书 URL 仅 `Loading...` + `location.replace(...?ch=1&js=JWT)`；`webView` 后跳 `ovret.com` 等广告联盟；`wwNN.` 子域为品牌壳 | **skip/disable** — 非选择器。Harness：`no_auto:jwt_shell_disable` |
| **chapter_verify_html_wall** (shuhui) | 迁域后详情/www TOC 可读，章节 **302→`/user/verify.html#…`**「加载中」；`/user/search.html` 恒 `[]` | **skip/disable** — 非选择器；勿空耗 webView。Harness：`no_auto:verify_wall` |
| **没有找到站点 (521danmei)** | title=`没有找到站点` / 空壳 | L2 `deadish:没有找到站点` → **skip**。Harness：`sniff.rs` DEADISH_HINTS |
| **apex_no_a_try_m (tongrenquan)** | 裸 IP/`没有找到站点`；`header.Host=tongrenquan.org`；apex/`www` 无 A，但 `m.` 有 CF A；备注 `Unable to resolve host` | **勿**对 IP 空壳/死 www **缓修当 skip**。读 Host/旧域 → **立刻** `hunt --probe`（种子常已有 `m.`）→ migrate+verify+书架 remap。见 `gate_hunt_deferred_unprobed`。Seeds：`domain_hunt_seeds.json` |
| **nginx 空站 (cstxt)** | title=`Welcome to nginx!` | L2 `deadish:welcome to nginx` → **skip** |
| **域名广告劫持 (pyzht)** | title=精选推荐 / `gg_card` / 18+广告壳 | L2 deadish 广告标记 → **skip** |
| **Empire 搜索体 (fuxsb)** | debug 有书但 check「搜索失效」；`show=a,b,c` 体 | 简化 `keyboard={{key}}&show=title&tempid=1` + Referer；正文 `.co-by`→`.conbd` |
| **webView 在 bookUrl** | `##$##,{'webView': true}` | `apply_safe_rule_fixes` 现修 ruleSearch.bookUrl（不仅 chapterUrl） |
| URL 前导空格 | `get_source` 失败但 list 能见到 | trim `bookSourceUrl` 再 get/migrate |
| bookbenx 换域 | `.item` + `/search81.html?searchkey=`（新书迷楼→shukuai99） | 固定 searchUrl，勿依赖 ajax 抽 form |
| 假首页搜索 (爱丽丝) | form=`/?keyword=` 但结果=首页壳；真入口 `/search.php?q=` | **继续修** — rank；换真 searchUrl |
| **搜索口不对** | 首页有 form，但书源 searchUrl/common_path 404/错页 | **继续修** — 先试 form 的 action（POST/GET），勿 skip |
| **搜索口挂了** | form 指向的真实入口稳定 HTTP 5xx /「连接数据库失败」 | **skip** — 非规则问题 |
| xunsearch pid 链 | `javascript:…pid: N` 无真实 href | bookUrl=`##pid:(\\d+)##/novel/$1.html###` |
| EmpireCMS keyboard | form field=`keyboard`（`/e/sch/`） | searchUrl 含 `keyboard={{key}}`；探针已收录 |
| POST 搜索未打分 | 旧 rank 跳过带 JSON 的 POST | rank 现会 POST 试抓 |
| common_path 误伤 | `/search?q=` 得虚高分压过 form POST（ixs7） | form 优先；error_page 扣分；5xx→`search_endpoint_dead` |
| 候选海选 | 多 URL classify+probe 选“好修的” | **banned** — `progress next` 只取一个 |
| 站点 DB 挂了 | 正文/搜索页仅「连接数据库失败」 | **skip**（同「搜索口挂了」） |
| 密码墙 / urldance | title=请输入密码；跳转 urldance.com | **skip**（L2 `wall:`；勿 diagnose） |
| L2 未过就 debug | phone debug + rank 打在死站上 | diagnose/`progress next` 先 L2 fail-fast |
| **`--l0-only` 误用 (dcrsu)** | progress/diagnose 带 `--l0-only` → 超时站仍进 tips/probe（~37s） | **禁止** live 挑源/深修用 `--l0-only` |
| **jieqi 搜索 0 条 (b483)** | POST/m「共有 0 条」；浏览仍有书 | **disable**。禁止首页过滤假搜索。引擎 `site:` 思路延期：`docs/research/engine-site-search-deferred.md` |
| rate-only | only concurrentRate | not a fix (unless verify already OK) |
| URL 无 scheme | bookSourceUrl/searchUrl=`www.foo.com` | 自动补 `http://` 并 save；get_source 试去 `#` 变体；probe 限时 5s×6 |
| 空探针仍设备校验 | notes 空 + 搜索失效（浪费 ~10s×N） | serial `require_patch`；无补丁 → `no_patch_skip` |
| 过期 tagged_fails | missing「未找到书源」 | `repair_refresh_phone_index` + 队列只取 on_phone |
| **bookUrl class-space (po18f)** | search 有书名但详情链接=search.php；`class.X a@href` | → `class.X@tag.a@href`；去掉 `\|\|@js:baseUrl`；章节在详情页则清空 tocUrl |
| **fake_detail_empty_search** | 搜索无结果时列表空，但未配置 `bookUrlPattern` 触发阅读「按详情页解析」；且详情书名规则过宽（如 `h1@text` 或 title 正则）将无结果提示抓为假书名并报 TocEmptyException | 配置精准 `bookUrlPattern`；限定 `ruleBookInfo.name` 选择器范围（如 `div.info h1@text`），彻底删除 title 兜底正则 |
| **登录壳首页 (96biquge)** | 首页仅 `#loginform`+密码框、无小说搜索 | L2 `wall:login_shell_not_novel` → **skip** |
| **charset 误标 (52dmshu)** | searchUrl `,{"charset":"gbk"}` 但站点已 UTF-8 → 列表空 | 去掉 gbk / 改 utf-8；重抓结果 DOM（常为 `#sitembox dl`，bookUrl=`dt a@href`） |
| **probe 未解压 gzip** | Accept-Encoding 有 gzip 但 body 不解压 → forms=[] | `fetch_text`/`_post_fetch` 必须 gzip 解压后再 parse |
| **form 体过长** | `<form>…</form>` >800 字符被截断漏抓（52dmshu=807） | forms regex 上限调到 4000 |
| **CF 空搜索体 (qiufeng)** | debug「获取成功」但 `a`/`p` 列表亦为 0；PC POST→403 Just a moment | **skip** — WAF，非选择器 |
| **正文 textNodes 空 (biduju)** | debug 到正文步 `ContentEmptyException`；章节有 `<br/>`/`font` | `class.chapter@html`（勿死磕 textNodes） |
| **域名改行 (jinyongwang)** | title=「…专业生产厂家」；搜索 placeholder=查询的产品 | L2 deadish → **skip/disable** |
| **ac.qq 移动↔桌面分流 (acqq_mobile_chapter_redirect)** | m 搜索 302 丢 query→列表空；OkHttp 读章节 302→桌面 ComicView，原 `@js` 解密读不到 `data:` | 搜索改 `ac.qq.com/Comic/searchList`+桌面 selectors；详情/目录用 desktop `works-*`；章节 URL 仍指 m；正文需 `java.get(…).body()`+移动 UA 或后续 API 研究 — **未完全修** |
| **progress next 卡死** | 候选按 URL 字母序 → 永远先 `api.*`；index 无 RT | 优先 `queues/repair_serial100_queue.json` `items` |
| **stale_queue_after_migrate** | 迁域/删旧源后 `progress next` 仍挑到旧 URL（serial 快照未刷；`phone_source_index` 过期仍含旧域） | queue 候选必须仍在 `phone_source_index.by_url`；`migrate` 成功后 ledger `skip:migrated_to:` 封 `from_url` **并** `refresh_phone_index`。手工 MCP 迁域后同样要 `queue refresh-index` + ledger skip。Harness：`progress.rs` + `migrate.rs` + `progress_ledger` |
| **phone_index_group_alias** | `refresh-index` 后 progress/rt 候选变 0 | MCP 行是 `bookSourceGroup`/`bookSourceName`；index 写 `group`/`name` 别名，progress/rt **双读**。Harness：`source-queue/index.rs` + `rt_queue.rs` + `progress.rs` |
| **App JSON 搜索空壳 (ihuaben)** | `/search` HTML 404；旧 `$.pageUtil` 规则把 HTML 当 JSON → `$.book` 吃到 String | `so.ihuaben.com/search?keyword=` + `.searchresult`；bookUrl 正则映射 `/book/{id}.html`→`/book/app/book?bookId=`；tocUrl `cdn/chapters/{{id}}/{{Date.now()}}`（勿 `java.time()`） |
| **重复 phone pull (serial)** | 每批 `refresh_phone_index` 全量 list_sources ~55s | 用 `repair_state.sqlite` + TTL；`repair_refresh_phone_index --force` 才重拉；`get_source` 走 snapshot cache |
| **Vue SSR 搜索空 (qimao miao)** | `/search/index/` 200 但无 `ul.qm-pic-txt`；`__NUXT__` 壳；phone list=0 | `searchUrl=https://miao.qimao.com/api/search/result?keyword={{key}}`；`bookList=$.data.search_list[*]`；bookUrl `@js` put bid→`api-miao…/chapter-list`；content `.article@html` |
| **inte_base64 搜索壳 (xinbiquge)** (inte_base64_search) | `search.aspx` 体为 `inte_base64:{"c":base64}`，选择器打空 | `bookList` `@js`: 先 strip `inte_base64:` → `JSON.parse` + `java.base64Decode(o.c)` + `java.setContent` → `div.hot_sale`；规则本身常仍可用。Harness：`diagnose_tips` |
| **m 站 500→桌面 (aaread)** (m_host_500_try_desktop) | `m.*/search`/`/book` 5xx；桌面搜索+详情目录 OK | **优先于** `search_endpoint_dead` skip：先探 apex/`www` 再决定；迁 `bookSourceUrl` + 重写 searchUrl/selectors（query 可能是 kw/q/keyword） |
| **起点壳正文 (aaread)** (qidian_clone_getcontent) | `.j_readContent` 只有「内容读取中」；`ajaxGetContent`→`/_getcontent.php?id=` | content `@js`: 从 `/chapter/{bid}/{cid}` 取 cid → `java.ajax` + `setContent` + `getString('p@text')`；勿 `return`（Rhino）。Harness：`diagnose_tips` |
| **URL 尾 CR (ruochu)** (url_trailing_cr) | `bookSourceUrl` 含 `\r` → list/get 怪异、校验挂搜索目录 | 删旧源，保存无 CR 的干净 URL；索引里 `repr(url)` 先查 |
| **若初/黑岩目录 (ruochu)** | 详情「查看章节目录」→`w2.heiyan.com/chapter/{id}`；旧 `.float-list` 空 | `tocUrl=text.查看章节目录@href`；`chapterList=.chapter-list a` |
| **搜索页过大超时 (roushuwu)** (huge_search_page_timeout) | 「我的」POST 回 ~2MB/~1900 条；分页 TOC 每页 ~30s → 校验超时 | `bookList=.….0:20`；`checkKeyWord` 用更稀词（剑来）；慢站可去掉 `nextTocUrl`；check `timeoutMs≥180000`。Harness：`diagnose_tips` |
| **轻之文库搜索 (linovel)** (linovel_search_book) | 旧 `rank-book`/`rank-book-list@a` 空；真结果是 `a.search-book`；`:443` searchUrl 易慢 | `searchUrl=https://www.linovel.net/search?kw=`；`bookList=a.search-book`；https 迁域。Harness：`diagnose_tips` |
| **tocUrl 阅读链 (powanjuan)** | `tocUrl span.read a`→首章；误走 `index/1.html` 目录空 | **清空 tocUrl**；详情页 `div.catalog` + 已有 `ruleToc` |
| **COS toc 403 (tybook)** | `chapters/{bid}.json` 403 | 改 signed `/tf/chapter_list?` @js |
| **sticky_host_header_cdn (tybook)** | `header.Host` 钉死 API 域；`/tf/chapter_list` 302→`scdn…/chapters/{bid}.json` 后列表空 | **去掉 Host**（或克隆无 Host 的 sibling UA）；不要只改 tocUrl。Harness：`diagnose_tips` |
| **目录 href 伪装 (gaysay/tantan/1redbook)** | 全部 `<a href="/book/id/">`；真 URL 在 `data-c*` base64（哈希名因站而异：`c8dcb4a`/`c9e6f9f`） | `chapterList` 必须选到 **`@tag.a`**（attr 在 a 上，不在 li）；`chapterUrl` `@js:java.base64Decode(result.attr('data-…'))`；`chapterName` `@data-…\|\|text`；无 scheme 的 `bookSourceUrl` 先补 `https://` |
| **POST /sa 搜索空 (yoduzw)** | phone POST 200 list=0；分类页有书 | **disable** §16 |
| **小米浏览器书城 (miui)** | `reader.browser.miui.com` API 搜索 list=0；L2 body 0；需 App 签名 | **disable/skip** — 非公开 HTML 书源 |
| **17mb 空 index + 未审书 (xinbanzhu)** (17mb_empty_index_unapproved) | 「查看目录」→`…/index.html`/`zx.js` 空壳；新书「未经审核」首条 TocEmpty | toc=`/html/{dir}/{id}_1/` 静态；校验勿用易撞空书的「我的」。Harness：`apply_safe_rule_fixes`→`17mb_empty_index_tocUrl`；diagnose TOC tip |
| **17mb POST+GBK 搜索 (mpo18)** (17mb_post_gbk_search) | GET `s.php?s=` 空；真搜索是 POST `s`+`type=articlename`+GBK；结果在 `p.sone`；CF 单源校验易 90s 超时 | `searchUrl=…/s.php,{"charset":"GBK","method":"POST","body":"s={{key}}&type=articlename"}`；`bookList=class.searchresult@p.sone`；补 bookInfo name/author；check `timeoutMs≥180000`。Harness：`diagnose_tips` Search tip |
| **JSON API 详情空字段 (cooks)** (json_api_bookinfo_fields) | search/toc/content OK；bookInfo 无 name/author；`@js` 模板字符串导致 js失效；init stringify 后 coverUrl 读不到 `.articleid` | 补 `$.articlename`/`$.author`；coverUrl `JSON.parse(result)`；`@js` 用字符串拼接。Harness：`diagnose_tips` Search tip |
| **ss_search_delay_cookie (15u)** | debug「获取成功」list=0；HTTP 体 `alert(搜索间隔)` / Cookie `ss_search_delay` | **勿改 bookList** — `source-cli check clear-cookies --url …`；`enabledCookieJar=false`；可选 searchUrl `@js` removeCookie。Harness：`sniff_search_rate_limit` + probe `search_rate_limit` + `diagnose_tips` |
| **cookiejar_cf_needs_on (twkan)** | CF 搜索需要 cookie/webView；与限流站「关 jar」相反 | jar true + webView；仍挡 → skip。Harness：`diagnose_tips`；指南 `book-source-create.md` |
| **multi_toc_pick_longest (15u/ttks)** | 多块目录容器；固定 `.1`/frame 只有最新章 | `@js` 取链接数最多的容器。Harness：`diagnose_tips` |
| **multi_list_charts_toc (15u/jile1)** | 多个 `ul.list-group.list-charts`；固定 `.1` 只有最新几章，**或**第二块是相关推荐黄链 | 正文在首块用 `.0`；块序不定用 multi_toc_pick_longest |
| **relative_ajax_toc (sto55)** | `ajax_index.html` 相对路径目录空 | `tocUrl=@js: baseUrl + 'ajax_index.html'`。Harness：`diagnose_tips` |
| **desktop_empty_mobile_content (xsw)** | PC 正文空；m. 可读 | 桌面搜+目录，章节改写 m.。Harness：`diagnose_tips` |
| **debug_colon_explore** | `::URL` 当详情调试 | 用绝对 URL / `++URL`。Harness：`diagnose_tips` |
| **check_keyword_too_broad** | 「我的」首条坏书 → 假目录失败 | 稀有书名片段作 checkKeyWord。Harness：`diagnose_tips` |
| **booksourcecomment_holds_js_helpers** | 规则 `eval(String(source.bookSourceComment))`；改备注后 SyntaxError/找不到 map | **禁止**白话覆盖 comment；备份后再改。Harness：`no_auto:restore_comment_js` |
| **alias_label_host_nxdomain_api_alive** | `bookSourceUrl` NXDOMAIN，但 `type=`/API/`data:;base64` 搜索仍通 | 勿仅凭 L1 hunt_empty 禁用；先 `debug_source`。Harness：`no_auto:debug_before_disable_alias` |
| **alias_booksourceurl_false_dead** | PC gate `l2_bot_shell`/CF，但手机可跟 301 到 `.org` 等活域；或备注仍写 Unable to resolve | **勿**只凭 PC CF 禁用。先 `debug_source`/校验；清陈旧 DNS 备注。Harness：`no_auto:phone_debug_before_disable_cf` |
| **dig_migrate_report_without_persist** | dig/oneshot 报 `status=migrated`，但手机 `bookSourceUrl` 未改、新 host 源不存在 | **勿**信报告 alone。`get_source`/SQL 确认后手工 upsert+remap+校验；再禁旧 URL。Harness：`no_auto:migrate_upsert_remap` |
| **l2_502_brand_cluster_dead** (18ys) | 主域 L2 **502**；同品牌镜像/`m.`/相关站也 502 或 NXDOMAIN/lander；hunt+OSINT 无活后继 | **skip/disable** — 勿把广告 lander（`www.18ys.com`→`/lander`）当后继。Harness：`no_auto:site_dead` |
| **search_author_concat_sibling_div** (rouwen/xn--7dv) | 搜索 `class.author@text` 拼出 `新乙\n阅读量：882`；同级第二个 `div.author` 是阅读量；详情无 author | search `class.author.0@text##作者：`；bookInfo `class.booktag@tag.a.0@text`。Harness：`no_auto:站点 DOM 特例` |
| **dict_url_decode_fake_name** (haici/dict.cn) | `name=@js:decodeURI(baseUrl…)` + `bookList=body` → 任意换源关键词都「书名命中」；详情「该词条未找到」 | bookList `@js` 遇未找到返回 `[]`；name 用页面 `tag.h1`/`.word`；换源侧 `isAcceptableChangeSourceHit`（空/假最新章、本地有作者却空作者、词典 intro）。Harness：`ChangeBookSourceQuality` |
| **url_decode_fake_booklist_non_novel** (百度图片/知道) | `bookList=@js:[{title:decodeURIComponent(word…)}]` 把搜索词伪造成一书；非小说站 | `bookList=@js:[]`；换源 `isNonNovelSearchHost` 黑名单。Harness：`ChangeBookSourceQuality` |
| **qq_search_state_not_items** (白浏览器/松鹤) | `bookList=$.data.state[*]` + 搜索 JSON 无 lastChapter → 换源空最新章被丢 | flatten `novel_search_list.items`；bookInfo 补 `lastSerialname`；App 侧作者+长简介可放行空最新章 |

## Worked examples

| Source | Fix |
|--------|-----|
| 画本 | toc `/list/{id}` + `.chapter-row` |
| 御书屋 | search.php + `#sitebox dl` + 目录 + `#YiJianZhan` |
| 长佩 | `chapterGetList` + `{"webView":true}` + `.chapter-render-box` |
| 爱丽丝 | migrate `.org→.com`；`/search.php?q=`(xunsearch)；pid→`/novel/`；`article#chapterContent` |
| 新书迷楼 | trim 空格 URL；migrate→`shukuai99.net`；`/search81.html?searchkey=` + `.item` |
| 卧龙 paper027 | https；`/api/v1/books/search?q=` + `$.data.data`；toc `/chapter/$id`；`.prose` |
| PO18文学 po18f | `bookUrl=class.bookname@tag.a@href`；清空 tocUrl；`id.list-chapterAll@a`；校验成功 |
| 吾爱耽美 52dmshu | 去 gbk charset；`#sitembox dl` + `dt a@href`；目录在详情 `#list dd a`；校验成功 |
| 繁星四月 fuxsb | migrate https；Empire 搜索体简化+Referer；正文 `.conbd@html`；校验成功 |
| 笔趣 96biquge | 登录壳非小说站 → skip |
| 笔趣 bqgcn | diagnose=ok → verify |
| 猫眼 / 123du / 古诗文 | skip (auth / captcha / WAF) |
| 必读居 biduju | search `keyword`+GBK+`class.list@table`；正文 `class.chapter@html` |
| 金庸 jinyongwang | 域名改行工业风机站 → disable/skip |
| 稻草 dcrsu | L2 HTTP timeout → disable/skip |
| 免费小说 b483 | search.php 空索引 → **disable**（勿首页过滤 workaround） |
| 话本 ihuaben | `so.ihuaben.com` HTML 搜 + app book JSON + CDN toc/content；`Date.now()` tocUrl；校验成功 ~3.5s |
| 破万卷 powanjuan | 清空 `tocUrl`；详情页 catalog + ruleToc；keyword=斗罗 校验成功 |
| 基友 gaysay | toc `data-c8dcb4a` base64 chapterUrl；641 章 + 正文 OK |
| 淘小说 tybook | COS 403 → `/tf/chapter_list` signed tocUrl |
| 第一版主 xinbanzhu | migrate `m→i`；toc `/html/{dir}/{id}_1/`；content `#nr1`+`pb_next`；校验 keyword=斗破 |
| PO18脸红心跳 mpo18 | POST+GBK `s.php`；`class.sone`；bookInfo `cataloginfo@h3`；校验 ~130s 成功（timeout 180s） |
| 小说阅读网 cooks.tw | JSON API search/detail/chapter；补 bookInfo `$` 字段；coverUrl 禁模板字符串；校验 ~4s |

## Scripts / CLI

| Entry | Role |
|--------|------|
| **`source-cli diagnose`** | L2 fail-fast + debug layer / fake_detail |
| **`source-cli repair`** | Live oneshot/batch |
| **`source-cli closeout`** | pending / gate / sync-skill / status |
| **`source-cli retro`** | Per-source reflection + optional ledger seal |
| **`source-cli progress` / `ledger`** | Queue next + session log |
| **`source-cli discover`** | MCP LAN probe + write mcp_defaults.json |
| **`source-cli check`** | channel / precheck / batch / full / **clear-cookies** |
| **`source-cli source`** | triage / fetch / verify / log / **push --file** |
| **`source-cli queue`** | refresh-index / rt queue |
| **Create guide** | `docs/guides/book-source-create.md` (CookieJar / debug keys / CLI path) |
| **`source-cli wave` / `harvest` / `serial`** | Batch orchestration |
| **`source-cli parse`** | Offline rule/url analysis |
| **`source-cli parity`** | `cargo test --workspace` + inventory |
