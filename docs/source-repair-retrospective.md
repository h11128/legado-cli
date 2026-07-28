# Source Repair Retrospective (2026-07-26)

## Verdict

**修好一个可救书源，正常应是 2–5 分钟，不是 15 分钟。**  
「15 分钟」只是硬停上限，被误当成合理工期。壁钟时间主要被 **过程浪费** 吃掉，不是选择器本身难。

破万卷 / 爱久久有效改动都很小；设备单源校验各约 **2–3 秒**。详见会话全记录：  
`docs/source-repair-session-log-2026-07-26.md` + `temp/full_fix/repair_session_index.json`。

---

## 1. Timeline waste (observed)

| Waste | What happened | Cost |
|-------|----------------|------|
| Subagent cold start | Each agent rediscovered MCP IP/session/fail semantics | 3–8 min / agent |
| Channel contention | `full_check_runner` / `batch_check_mcp` zombies + fix agents | queue / timeout / redo |
| Fake “fixed” | `save_source` logged as fixed without single-URL check | ijjj/pow full rework |
| Rate-limit misread | debug → immediate check →「搜索失效」→ rewrite rules | wasted loop + 20s |
| Over-exploration | TOC bug but wander search/content/explore | larger blast radius |
| Ad-hoc scripts | Dozens of `legado/.local-scripts/inspect_*.py` one-offs | no reuse |
| Soft 15-min budget | Ceiling treated as target | permission to thrash |
| **Infra underuse** | legadoSkill docs/debugger/KB barely used | reinvented triage |

---

## 2. What actually broke (technical)

1. **破万卷**：`tocUrl` → content page → no `.catalog`. Fix: clear `tocUrl`.  
2. **爱久久**：broad `a@href##regex##` → homepage; `name` mixed `||`+`##`; **20s** search gap.  
3. **book18**：pagination/`name` (verified earlier in `verify_fixed.json`).

Once the *resolved* toc URL was inspected, fixes were minutes of work — not quarter-hours.

---

## 3. Gaps in the first retro (不够彻底的地方)

Earlier retro listed symptoms but under-specified:

| Gap | Missing detail | Now |
|-----|----------------|-----|
| No session ledger | Outcomes only in chat / scattered jsonl | Session log + index JSON |
| Fake-fixed not named | Which agent, which retest | Section 3 of session log; `fake_fixed_then_reworked` |
| Unverified batch “fixes” | bengben/zxcs/627txt claimed in `fix_log.jsonl` never in verify_fixed | Marked **unverified claims** |
| MCP IP drift | Skill still advertised `.43` while phone was `.139` | Standing note; repair skill default `.139` |
| Parallel policy | “don’t parallel” said late | Ban: 0 fix agents while bulk owns channel |
| **legadoSkill infra** | Almost absent from first retro | **§4 below** |
| Knowledge vs invent | Agents guessed CSS without CSS/TOC docs | Mandate doc skim before deep edit |
| Debugger unused | Local `debugger/` never in P0 path | Optional pre-check; device still authoritative |
| Logging standard | fix_log vs fix_pow shapes differ | Prefer `repair_source.py log` schema |
| Skip quality | Batch skips OK; no disable-on-device always applied | Follow with `disable_dead_sources` when skip=dead |

---

## 4. Did we use legadoSkill infra? Mostly no

### Inventory vs actual use this session

| Infra | Path | Used in repair waves? | Should have |
|-------|------|------------------------|-------------|
| Essential knowledge | `docs/ESSENTIAL_KNOWLEDGE_SUMMARY.md` | Barely | First read for HTML authenticity / rule pitfalls |
| CSS selector notes | `assets/css选择器规则.txt` | No | Before rewriting selectors |
| TOC pagination rules | `docs/TOC_PAGINATION_RULES.md` | No | Any 目录失效 |
| HTML authenticity checklist | `docs/HTML_AUTHENTICITY_CHECKLIST.md` | Partial (raw dumps yes, checklist no) | Always |
| Local debugger | `debugger/test_universal.py`, `legado_checker.py` | No | Quick PC sanity after HTML theory |
| Example sources KB | `assets/knowledge_base/book_sources/` | No | Pattern match similar sites |
| Upstream mega-skill | `skills/SKILLV0.7.md` | Only at install | Repair skill supersedes for fix loops |
| Precheck / batch check | `scripts/precheck_*.py`, `batch_check_mcp.py` | Yes (bulk) | Keep for bulk only; pause before fix |
| Disable dead | `scripts/disable_dead_sources.py` | Partial | After skip=dead |
| Full check runner | `scripts/full_check_runner.py` | Yes — **also collided** | Lockfile + exclusive with fix |
| Repair CLI (new) | `scripts/repair_source.py` | After the fact | **Default path now** |
| Throwaway probes | `legado/.local-scripts/inspect_*.py` | **Heavy** | Prefer `repair_source.py fetch` |
| Past fix writeups | `docs/歌书网书源错误分析与修复.md` etc. | No | Search docs before inventing |

**Conclusion:** We treated legadoSkill as a **temp dump + MCP scratchpad**, not as the repair toolchain. That forced every subagent to re-derive session glue, HTML fetch, and rule folklore — the real reason wall time exploded.

### What “full utilization” looks like

```
1. repair_source.py triage          # fail layer + smells
2. Skim ESSENTIAL + TOC_PAGINATION if layer=toc/content
3. repair_source.py fetch           # headers + dump + toc candidates
4. Optional: debugger/test_universal on saved JSON (approx only)
5. Minimal save_source via MCP
6. repair_source.py verify --cooldown N
7. repair_source.py log → temp/full_fix/fix_*.json
8. Append one line to session index / fix_log.jsonl
```

Bulk path stays: `precheck` → **one** `full_check_runner` → classify materials → then **serial** repair with above loop.

---

## 5. Why it *felt* like 15 minutes

Waste path:

```
spawn → rediscover MCP → hand-roll session → blind debug → bare curl
→ save → claim fixed → parent retest fail → spawn again
→ rate-limit false search fail → sleep → verify
```

Effective path (with infra):

```
triage → fetch → 1–2 field edit → cooldown verify → log
```

---

## 6. Process bans

1. No “fixed” without `repair_source.py verify` (or equivalent single-URL check).  
2. No fix agent while bulk runner owns MCP.  
3. No rewriting searchUrl on rate-limit HTML.  
4. No `||` + `##` on the same field.  
5. No broad `a@href##…##` tocUrl without checking resolved URL.  
6. No treating 15 min as target (≤5 target, 10 hard stop).  
7. No new `inspect_*.py` for a one-off if `fetch` covers it.  
8. No ignoring legadoSkill docs on TOC/CSS failures.

---

## 7. Tooling now

| Script | Role |
|--------|------|
| `scripts/mcp_client.py` | MCP session + get_source |
| `scripts/repair_helpers.py` | layer / smells / headered fetch |
| `scripts/repair_source.py` | triage \| fetch \| verify \| log \| channel \| index |
| `scripts/mcp_channel.py` | Exclusive bulk↔repair lock |
| `scripts/repair_claim.py` | Anti fake-fixed + index append |
| `config/mcp_defaults.json` | MCP URL/token SOT |
| `docs/FIX_AGENT_PROMPT.md` | Subagent paste template |

Skill SOT: `E:/shared-skills/legado-book-source-repair/SKILL.md`

---

## 8. SLOs

| Metric | Target |
|--------|--------|
| Wall time / fixable source | **2–5 min** |
| Hard stop | **10 min** → skip + log |
| Device verify | Always |
| Parallel fix on one phone | **0** during verify/debug |
| Docs skim on toc/content fail | Required |
| Session ledger update | Required every verified/skip |

If another session burns 15+ minutes on a one-line tocUrl bug, failure mode is **process + infra neglect**, not the site.

---

## 9. Mitigations shipped (2026-07-26)

| Problem | Layer | Fix |
|---------|-------|-----|
| Fake `fixed` | Script | `log --status fixed` refuses without verify `success=true` |
| Channel contention | Script | `mcp_channel.py`; verify asserts idle; bulk runner acquires lock |
| Stale MCP IP | Config | `config/mcp_defaults.json` SOT |
| Ad-hoc inspect_* | Hook + MDC | beforeShell prompt; discipline mdc |
| Subagent cold start | Script | **`repair_one.py` one-shot** + `FIX_AGENT_PROMPT.md` |
| Infra underuse | Script | **`repair_knowledge.py`** searches docs/assets |
| Auto fix smells | Script | **`repair_patches.py`** clear tocUrl / split \|\|`##` |
| URL class | Script | **`repair_classify.py`** homepage/content/catalog |
| Host search gap | Script | **EWMA in `repair_cache.py`** + verify `--auto-cooldown` |
| Queue priority | Script | **`repair_queue.py`** |
| HTML refetch | Script | **HTML cache** under `temp/full_fix/cache/` |
| Disable dead | Script | `repair_one` decision=disable → `disable_source` |
| 15 min as target | MDC + Skill | Target 2–5 / hard stop 10 |
| Ledger | Script | `log --index` / repair_one writes index |

Default command: `python scripts/repair_one.py --url … --fail-msg …`




## Follow-up retro (migrate/video evening)

See `docs/source-repair-retro-migrate-video-2026-07-26.md` and phase log `docs/source-repair-session-phase-migrate-video-2026-07-26.md`.

---

## 10. Missed alicesw search (2026-07-26 night)

**Why the agent missed it earlier**

1. **L2 used to treat HTTP 200 as alive** (HEAD, almost no body) → parked / redirected hosts looked “fine”; migrate to `www.alicesw.com` was delayed until body-sniff + host-redirect landed.
2. **`repair_search_probe` trusted the first homepage form** → empty-action `keyword` form became `/?keyword={{key}}`, which returns the **homepage shell** (looks “found”, scores as search candidate, zero real books).
3. **No common-path fallback / no live score** → never tried `/search.php?q=` (xunsearch) until a human said “alice 有搜索”.
4. **xunsearch hrefs are `javascript:…pid: N`** → first HTML scrape reported “no bookish links”, easy to mis-skip as broken search.

**Mitigations shipped**

| Gap | Fix |
|-----|-----|
| Fake form candidate | `score_search_html` + `rank_candidates` (penalize same-title-as-home) |
| Missed endpoint | `COMMON_GET_TEMPLATES` includes `/search.php?q=` etc. |
| pid JS links | `bookUrl_hint` → `##pid:(\\d+)##/novel/$1.html###` |
| Agent guidance | skill traps: 假首页搜索 / xunsearch pid；diagnose `best` + signals |

Proof: `PYTHONPATH=scripts python -c "from repair_search_probe import probe_search_forms; print(probe_search_forms('https://www.alicesw.com/', keyword='重生')['best'])"`

---

## 11. Standing close-out (2026-07-27)

User preference locked into discipline + skill: **after every URL** → document (`docs/…` + ledger) → `repair_retro.py` → patch skill/code if new trap → only then next URL.

## 12. biduju content empty (2026-07-27)

| Signal | Cause | Fix |
|--------|-------|-----|
| debug OK search/toc; `ContentEmptyException` / check「搜索正文失效」 | `ruleContent.content=class.chapter@textNodes` on chapter HTML that uses `<br/>` + nested `<font>` | `class.chapter@html` |
| diagnose briefly said `layer=ok` while check failed | debug path can look “complete” if content flake; trust check tag + re-debug | prefer fail_msg `搜索正文` → content; re-debug when check≠debug |

## 13. jinyongwang domain repurposed (2026-07-27)

| Signal | Cause | Fix |
|--------|-------|-----|
| diagnose `layer=search` / fake_detail; form `keywords` | Domain still 200 but title=「安盛风机…专业生产厂家」; product search shell | **skip/disable** — not a novel site |
| `forms_js` panic on Chinese HTML | byte window slice mid-codepoint | `utf8_window` in `forms_js.rs` |
| `progress next` stuck on `api.xingliang…` | URL alpha + phone index 无 RT；queue 字段是 `items` | progress 优先读 `repair_serial100_queue.json` 的 `items` |
| L2 missed | no industrial-shell hints | `专业生产厂家` / `工业通风` / `请输入您要查询的产品` in sniff + prefilter |

## 14. dcrsu L2 HTTP dead (2026-07-27)

| Signal | Cause | Fix |
|--------|-------|-----|
| full `gate` → `l2_http_dead` / timeout 10060 | host dead (CF IP but no TCP reply) | **disable/skip** |
| diagnose with `--l0-only` still ran tips/probe (~37s) | agent used L0-only on live pick path | skill+prompt: **ban `--l0-only`** for progress/diagnose/repair live |

## 15. b483 jieqi search index empty (2026-07-27)

| Signal | Cause | Decision |
|--------|-------|----------|
| POST/m 搜索恒 0 条；浏览/详情仍正常 | 服务端搜索索引空 | **disable**（用户偏好） |
| 曾用 JS 拉 home/top/sort + contains(key) | 假搜索，覆盖面差、脏链 | **撤回** — 不算正经修法 |
| Bing/Google `site:host key` | KB 有先例：顶点 `ddxsmf` → `cn.bing.com/search?q=site:…` | **延期**：见 `docs/engine-site-search-deferred.md`（Brave MCP / Serper 等）；有价值再做 |

Proof of prior engine-search pattern: `assets/knowledge_base/book_sources/6875_顶点小说ddxsmf_书源_20260218_103244.md`.

## 16. ihuaben app search dead → so HTML (2026-07-27)

| Signal | Cause | Fix |
|--------|-------|-----|
| diagnose `layer=search` / fake_detail；`/app/search` → `{}` | 旧 Android 搜索 API 空壳；站点仍活 | **继续修** |
| `so.ihuaben.com/search?keyword=` 62KB；`.searchresult`×30 | 真搜索在 so 子域 HTML | `searchUrl` 改 so；`bookList=.searchresult`；`h2 a` 书名/链接 |
| 详情 HTML + `cdncn…/cdn/chapters/{id}` JSON 仍 200 | 目录/正文 API 未死 | 详情页 CSS bookInfo；`tocUrl` JS 抽 bookId → CDN；`ruleToc`/`ruleContent` 保持 JSON |
| listv2 发现仍 OK | 勿动 explore（默认不修发现） | 仅修搜索层 |

Proof: device verify `校验成功` ~3.5s（`checkDiscovery=false`）.

## 17. Phone pull cache + repair_state.sqlite (2026-07-27)

| Pain (this thread) | Cause | Fix |
|--------|-------|-----|
| `repair_refresh_phone_index` ~55s every serial batch | always `list_sources` 4719 rows via MCP | SQLite `source_snapshot` + `phone_pull_at`; TTL default 3600s (`config/repair_db_defaults.json`); `--force` to re-pull |
| Repeated `get_source` per URL in oneshot | no PC cache of BookSource JSON | `mcp_client.get_source` reads TTL-fresh `source_snapshot`; `save_source` upserts; env `REPAIR_SKIP_PHONE_CACHE=1` bypass |
| Ledger grep-only / progress re-read JSONL | no indexed store | dual-write JSONL + `ledger_events` via `repair_db.append_ledger_row` |
| HTML/host_stats whole-file rewrite | race + no query | `repair_cache` still writes files; also upserts `html_cache_meta` / `host_stats` tables |

**Ops:** `python scripts/repair_db_cli.py migrate|status|import-ledger|import-cache|export-phone-index`  
**DB:** `temp/full_fix/repair_state.sqlite` (gitignored via `temp/`). Rust `source-cli ledger` + oneshot use `DualLedgerPort` (JSONL + SQLite). Python `scripts/repair_db.py` is the live access layer until §12 cutover.

## 18. tybook.taoyuewenhua.net (2026-07-28)

| Issue | Fix | Verify |
|-------|-----|--------|
| COS `chapters/{bid}.json` 403 | `tocUrl` → signed `/tf/chapter_list?` @js (mibook sign) | 校验成功 6411ms |

## 19. yoduzw.com (2026-07-28)

| Issue | Action | Verify |
|-------|--------|--------|
| POST `/sa` 200 but list=0 (all keywords/selectors); browse/category OK | **disable** (rule §16 search API dead) | 校验失败:搜索失效 → disabled |

## 20. powanjuan.cc (2026-07-28)

| Issue | Fix | Verify |
|-------|-----|--------|
| `tocUrl span.read a` → 首章 URL，`index/1.html` 目录空 | 清空 `tocUrl`，用详情页 `div.catalog` + 已有 `ruleToc` | 校验成功 4432ms（keyword=斗罗） |

## 21. miao.qimao.com (2026-07-28)

| Issue | Action |
|-------|--------|
| search/index Vue SSR 无 `ul.qm-pic-txt`；api-miao 无 search 端点 | **disable**（browse/shuku OK） |

## 22. gaysay.com (2026-07-28)

| Issue | Fix |
|-------|-----|
| 目录 `href` 全指向 `/book/id/`；真实 URL 在 `data-c8dcb4a` base64 | `chapterUrl` @js base64Decode；`chapterName` @data-cf3b593 |

## 23. reader.browser.miui.com (2026-07-28)

| Issue | Action |
|-------|--------|
| `/api/v2/search/word` phone list=0；PC 404；L2 body=0 | **disable** — 小米浏览器 App 内嵌 |

## 24. m.ac.qq.com 腾讯漫画 (2026-07-28)

| Issue | Action |
|-------|--------|
| m 搜索 302→桌面丢 query；正文 m 章节 302→ComicView 解密失败 | 搜索/详情/目录改 desktop ac.qq；**fail** 正文仍缺（trap `acqq_mobile_chapter_redirect`） |

## 25. qmbook.taoyuewenhua.net 全免小说 (2026-07-28)

| Issue | Action |
|-------|--------|
| 目录取不到：站点是 App API，`/tf/chapter_list` 要 md5 `sign=` | tocUrl 改 signed `@js`（tybook 同族，已知 trap）→ 校验成功 5114ms |

顺手发现的 harness 缺口（已修）：oneshot 校验成功后往 ledger 写的是 `check: ok`，
而排队只认「校验成功」/`fixed:`，**修好的源会被无限重挑**。
现在 `source-types::LEDGER_VERIFY_OK` 是唯一写法，`oneshot_ok.rs` / `apply.rs` 都用它。

## 26. manmanapp.com 漫漫漫画 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `/search/word-{{key}}.html` 404；www 搜索已迁 m 域 | `searchUrl=https://m.manmanapp.com/search/search.html?keyword={{key}}`；`ruleSearch` 改 `.classification_list li` / `h3` / `.author` / `.story_plot` → **校验成功** 3422ms |

## 27. www.ireader.com 掌阅书城 (2026-07-28)

| Issue | Action |
|-------|--------|
| 全站 HTTP 202 + `probe.js` 反爬壳；browse/search 均无 DOM；m.zhangyue 亦腾讯 WAF | **disable** — 非公开 HTML 书源（同 miui / qiufeng 类） |

## 28. xinbanzhu 第一版主 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `m.xinbanzhu.net` JS 劫持死域 | migrate → `http://i.xinbanzhu.net/` |
| 「查看目录」→`…/index.html` 空壳；`zx.js` 是广告不是目录加载器 | tocUrl=`/html/{dir}/{id}_1/` 静态 li；去掉 webView |
| 关键词「我的」首条常是「未经审核」空书 → TocEmpty 误判规则坏 | 校验用「斗破」；`#nr1`+`pb_next` 正文分页 |
| 设备校验 | **校验成功** 7255ms（keyword=斗破） |
| Harness 补齐（同会话后补） | `source_patch` smell+auto `17mb_empty_index_tocUrl`；`diagnose_tips` TOC trap |

## 29. paper027.com 卧龙小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `/search?keyword=` 404；站点已改 API | `searchUrl=/api/v1/books/search?q={{key}}` + `$.data.data`；toc `/chapter/{id}`；正文 `.chapter-html-content` |
| 旧 URL `http://…#🎃` | migrate → `https://www.paper027.com`；**校验成功** 3241ms |

同轮：`jyapi.jyacg.com` TLS 过期 +「站点已暂停」→ disable/skip；队列 `m.xinbanzhu` 残留 → skip。

## Close-out 标准（每轮）

1. **诊断证据**：`diagnose` + phone `debug_source` / fetch → ledger + retro.msg  
2. **反思**：`repair_retro.py append`（trap / harness / script_fix / **skill_fix 如实**）  
3. **文档**：本节或 dated retro  
4. **改进**：新 trap → patch SKILL + Rust/Python **再** next URL（2026-07-28 补：thread trap + `diagnose_tips.rs`）

