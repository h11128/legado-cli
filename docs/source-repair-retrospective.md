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

## 30. stale_queue_after_migrate (2026-07-28)

| Issue | Fix |
|-------|-----|
| 手工迁域后 `progress next` 仍挑 `m.xinbanzhu.net` | 根因：`repair_serial100_queue.json` 陈旧 + `phone_source_index` 未刷仍含旧 URL；migrate 未封 ledger |
| Harness | `progress`：queue ∩ `by_url`；`migrate`：`skip:migrated_to:` + `refresh_phone_index`；ledger 认 `migrated to`/`migrated_to`；SKILL trap `stale_queue_after_migrate` |

## 31. phone index `bookSourceGroup` 别名 + 丁丁小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `queue refresh-index` 后 progress 候选=0 | MCP 字段是 `bookSourceGroup`/`bookSourceName`，progress/rt 只读 `group`/`name` → 别名写入 index + 双读 |
| `http://api.xingliangglobal.com##@遇知` 丁丁/猫眼 | debug+verify（keyword=斗破）**校验成功** 1227ms；无规则补丁（旧「搜索失效」多为限流/关键词 flake） |

## 32. yqk.net 言情小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| POST `search.php`+gb2312 常「找不到结果」；首页 form 是 **GET** | `searchUrl=/search.php?searchkey={{key}}&page={{page}},{"charset":"GBK"}`；`concurrentRate=1/10000` |
| 校验关键词 | 用「言情」（短词/穿越易空）；**校验成功** 23715ms |


## 33. yqk.net#♤yc 言情小说一程 (2026-07-28)

| Issue | Fix |
|-------|-----|
| 同站另一 fragment，仍为 POST 搜索 | 同 #yc1101：GET+GBK + concurrentRate；**校验成功** 23360ms |


## 34. m.1qxs.com 一七小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `search.html?kw=` 丢参；列表选择器过期 | `searchUrl=/search?kw={{key}}&p={{page}}`；`bookList=.show ul a` |
| `/catalog_*` 返回错误页「刷新」（WAF） | 清空 tocUrl；目录用详情 `.catalog .show li a` |
| 设备校验 | **校验成功** 1240ms（keyword=斗破） |


## 35. m.1qxs.com#🎃 包七小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| 同站另一 fragment | 克隆一七修复；**校验成功** 1215ms |


## 36. m.88xiaoshuo.net 变化说网 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `/search.html` POST 稳定 HTTP 500；首页浏览仍有书 | **disable/skip**（§16 搜索口挂了） |


## 37. m.shoujix.com 手机小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| POST `/search/` 空结果页（短词还 alert≥10字）；首页仍有书 | **disable/skip** §16 |


## 38. miao.qimao.com 奇妙小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| Vue SSR 搜索空壳（known） | **disable/skip** |


## 39. wap2.xinbiquge.org 笔趣阁 (2026-07-28)

| Issue | Fix |
|-------|-----|
| search.aspx 返回 `inte_base64` WAF 壳；列表空 | **disable/skip** |


## 40. wenxue.bkneng.com 可能世界 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `/wap/index/search` 与 `/www/search` 均 404 | **disable/skip** §16 |


## 41. ihuaben.com 话本小说 (2026-07-28)

| Issue | Fix |
|-------|-----|
| `/app/search` 带过期 `tokenId`；bookList 路径 | 去掉 token；`bookList=$..pageList[*]`；bookUrl HTML；tocUrl CDN chapters |
| 校验 | **校验成功** 2659ms |


## 42. manmanapp.com#Haxc1107 漫漫漫画 (2026-07-28)

| Issue | Fix |
|-------|-----|
| 旧 PC 搜索 URL | 克隆 `#♤yc` 的 m 域搜索；**校验成功** 5120ms |


## 43. manmanapp.com#yc1101 漫漫漫画 (2026-07-28)

| Issue | Fix |
|-------|-----|
| 同站 fragment | 克隆 `#♤yc`；**校验成功** 5168ms |


## 44. manmanapp.com#♤Haxc 漫漫漫画 (2026-07-28)

| Issue | Fix |
|-------|-----|
| 同站 fragment | 克隆 `#♤yc`；**校验成功** 5152ms — **goal 100** |

## 45. Batch50 close-out (2026-07-28)

处理 **50** 个失效标签书源（目标 goal 150 的下一批）：

| 结果 | 数量 | 说明 |
|------|------|------|
| **fixed（设备校验成功）** | 3 | `manmanapp.com#一程`；`m.1qxs.com/`；`yqk.net/`（clone `#yc1101`，keyword 言情） |
| skip / disable | 48 | 死站 L2、晋江/铁血签名 API、爱奇艺/起点非 HTML、假详情 timebox、Vue SSR 等 |

**过程坑（已修驱动脚本，未改 SKILL）：**
1. `source verify` JSON 后跟 cooldown 日志 → `json.loads` 整段失败，误判成功为失败（改 `raw_decode`）。
2. diagnose tips 常同时带 `fake_detail` + `vue_ssr` 模板句 → 误 disable 贼吧等（改：fake_detail 时不按 vue_ssr disable）。
3. `progress next` 只取前 40 且要求 L2=`verify`；剩余候选全是 disable/skip/migrate → `NO_CANDIDATE`（改：自建候选队列 `batch_repair50b.py`）。
4. 非法 trap 名 `假详情页` → retro BLOCK 卡住 closeout（改用 SKILL 已有 `假详情`）。

摘要：`temp/batch50/summary.jsonl`。goal：见 `progress status --goal 150`。

## 46. dead_skip_without_hunt policy align (2026-07-28)

| Gap | Fix |
|-----|-----|
| Gate `l1_unreachable` / `l2_http_dead` → Disable；batch 直接关站 | → `GateAction::Hunt`；oneshot `resolve_hunt` 后再 migrate/disable |
| wave 把 hunt 当 final skip 写 ledger | hunt 只进 report，不 seal |
| SKILL/discipline 未写死「先 hunt」 | 加 serial rule + checklist 2b + trap `dead_skip_without_hunt`；`docs/domain-hunt-trial` 改 CLI SOT |

## 47. Hang prevention harness (2026-07-29)

根因：serial 进程内 oneshot 卡在手机 MCP；Windows `pid_alive` 恒 true → 死锁占锁最长 6h；Agent 对整批 AwaitShell 空等约 11h。

| Layer | Fix |
|-------|-----|
| Channel | Win32 `OpenProcess` 判死 PID；repair stale **15m** / bulk **2h**；`check channel --force-clear` |
| MCP client | HTTP 超时 120→**90s** |
| serial | 默认子进程 oneshot + `--url-timeout-s 120`；杀挂续跑；`serial_heartbeat.json` + 每 URL 刷 `serial_last.json` |
| Agent | discipline §19–20；SKILL traps `serial_await_idle` / `serial_url_timeout` / `mcp_lock_zombie` |

## 48. MCP timeout SOT in mcp_defaults.json (2026-07-29)

| Field | Default | Used by |
|-------|---------|---------|
| `http_timeout_s` | 90 | `McpClient` ureq（get/save/check…） |
| `debug_timeout_s` | 45 | 仅 `tools_call("debug_source")` |
| `verify_timeout_ms` | 45000 | `start_check_sources` |
| `verify_max_wait_s` | 90 | `get_check_progress` 轮询上限 |

`source-cli discover` 重写 URL 时 **保留** 上述字段。Harness：`crates/source-mcp/src/timeouts.rs`。

## 49. so.ihuaben.com 搜索目录失效副本 (2026-07-29)

| Issue | Fix |
|-------|-----|
| `bookSourceUrl` 误存搜索 URL；`tocUrl=text.章节目录@href` + HTML `-.chapters p` | 克隆已修 `#🎃`：CDN `tocUrl` + `$..chapters[*]` + `$..content` |
| diagnose | `layer=toc`；CDN chapters/chapter 仍 200 |
| 校验 | **校验成功** 4447ms；`skill_fix=0`（已知 trap） |

## 50. so.ihuaben.com# fragment twin (2026-07-29)

| Issue | Fix |
|-------|-----|
| 同站 `#` 副本，旧 HTML TOC | 同 §49 CDN 克隆 |
| 校验 | **校验成功** 13974ms |

## 51. tybook.taoyuewenhua.net sticky Host (2026-07-29)

| Issue | Fix |
|-------|-----|
| COS 403 已知；只改 signed `chapter_list` 仍 TocEmpty | debug：`Host: tybook…` 跟随 302→scdn 后列表空 |
| 修法 | 克隆 `#` 规则并去掉 sticky Host；校验成功 4454ms |
| harness | SKILL trap `sticky_host_header_cdn` + `diagnose_tips` |

## 52. topwork.cc search 301 dead DNS (2026-07-29)

| Issue | Action |
|-------|--------|
| L2 首页 OK；`/search/` 301→`s-topwork-cc.188111.xyz` NXDOMAIN | **disable** §16；hunt empty |
| diagnose | `layer=skip` |

## 53. 199.33.126.51 没有找到站点 gate miss (2026-07-29)

| Issue | Fix |
|-------|-----|
| title=`没有找到站点` 但 L2 `action=verify` | `DEADISH_HINTS` 缺该串；已补 + 单测 |
| 源 | 已 `enabled=false`；ledger skip |

## 54. pilisf.com CF search wall (2026-07-29)

| Issue | Action |
|-------|--------|
| diagnose `layer=search`；probe `/s.php` score=5 | phone POST → CF「Just a moment」list=0 |
| | **disable/skip**（搜索墙，非选择器） |

## 55. mianfei22.com SPA detail down (2026-07-29)

| Issue | Action |
|-------|--------|
| search 18 本 OK；详情 webView 文案「遇到故障,在修复中」 | TocEmpty → **skip/disable** |

## 56. 同人圈 199.33.126.51 → m.tongrenquan.org (2026-07-29)

| Issue | Fix |
|-------|-----|
| 裸 IP title=`没有找到站点` 被 skip | `header.Host=tongrenquan.org`；apex 无 A，`m.` CF 有 A |
| 迁域 | `https://m.tongrenquan.org`；去掉 sticky Host；**校验成功** 755ms |
| harness | trap `apex_no_a_try_m` + seeds + diagnose_tips |

## 57. m.diyibanzhu.buzz https migrate (2026-07-29)

| Issue | Fix |
|-------|-----|
| diagnose `layer=ok`；旧标签校验超时 | http→https migrate；**校验成功** |

## 58. blnovel.cc schemeless→https (2026-07-29)

| Issue | Fix |
|-------|-----|
| `bookSourceUrl=blnovel.cc` 无 scheme | migrate `https://blnovel.cc`；**校验成功** |

## 59. m.liehuozw.com https migrate (2026-07-29)

| Issue | Fix |
|-------|-----|
| layer=ok / 校验超时 | http→https；**校验成功** |

## 60. m.1qxs.com 一七小说 re-apply (2026-07-29)

| Issue | Fix |
|-------|-----|
| schemeless migrate 后仍用 `search.html?kw=` → fake_detail | 重套 §34：`/search?kw={{key}}&p={{page}}`；`bookList=.show ul a`；空 tocUrl；`.catalog .show li a` |
| 设备校验 | **校验成功** 1051ms（keyword=斗破） |

## 61. api.myweipin.com 猫眼看书 (2026-07-29)

| Issue | Action |
|-------|--------|
| search 仍 15 本；详情/目录 `认证失败` code 4005 | JWT `exp`≈2025-09 已过期；三备份同病；需第三方 openid |
| | **disable/skip**（SKILL：猫眼 / API 目录要登录） |

## 62. app.wanshu.com 绾书文学网 (2026-07-29)

| Issue | Fix |
|-------|-----|
| `@JSon:` + 正文 `replace(/\<…/` Rhino SyntaxError / ContentEmpty | `bookList/chapterList=$.data`；`content=$.data.content`；去坏 @js |
| 设备校验 | **校验成功** 1334ms |

## 63. m.nshkedu.com 文趣阁 (2026-07-29)

| Issue | Action |
|-------|--------|
| Loading JWT 跳转 → 广告/威胁页；search 回 HTML | decrypt JS `Unexpected token <`；**disable**（广告劫持，不 hunt） |

## 64. nav.jijia-co.com 闲看小说 (2026-07-29)

| Issue | Action |
|-------|--------|
| search 空列表；`$.data` 为 String；plate `params exception` | **disable** |

## 65. app.shubl.com 书耽 (2026-07-29)

| Issue | Action |
|-------|--------|
| search 可解；TOC/详情 AES `BadPadding`；login_token 过期 | **disable**（需登录） |

## 66. m.123yuzhaiwu.com 肉文阁 (2026-07-29)

| Issue | Fix |
|-------|-----|
| diagnose `layer=ok` | 无补丁重校验；**校验成功** 18514ms |

## 67. book.mywebos.cn 国学書库 (2026-07-29)

| Issue | Action |
|-------|--------|
| diagnose skip；`unexpected end of stream` | **disable** |

## 68. Agent turn stall (2026-07-29)

| Issue | Fix |
|-------|-----|
| deep diagnose 卡在 `3322t`：`diagnose` MCP 10060 后台后**回合结束**；login/AES/广告源抠太久 | Discipline **§21** + SKILL trap `agent_turn_stall`：回合必须 close-out 或写下步；auth/广告 ≤2min seal；diagnose 失败改 PC+直连 MCP |
| 续跑 | 从 `deep_verify_pending` 的 `m.3322t.com#🎃` 继续（PC 搜索已通） |

## 69. m.88xiaoshuo.net seal (2026-07-29)

| Issue | Action |
|-------|--------|
| POST `/search.html` 500（§36） | 已 disable；本轮补 ledger/retro |

## 70. m.3322t.com 珀包文学 (2026-07-29)

| Issue | Action |
|-------|--------|
| PC 搜索 18 本；设备校验 timeout~90s（搜索 alone~31s） | **disable**（慢站/墙钟） |

## 71. skip batch (2026-07-29)

| URL | Action |
|-----|--------|
| `m.ac.qq.com` type=2 漫画 | disable |
| `m.gushiwen.cn` captcha | disable |
| `m.ixs7.com` host reset | disable |
| `m.88xiaoshuo` §16 | already disabled |

## 72. m.jjjxsw.com 久久小说 (2026-07-29)

| Issue | Fix |
|-------|-----|
| tocUrl `{'webView': true}`；设备曾超时 | `{"webView":true}`；`show=title`；**校验成功** 32178ms |

## 73. m.kujiang.com 酷匠 (2026-07-29)

| Issue | Fix |
|-------|-----|
| 全局 app 头 → search 空体；catalog 无 app 头 → 版本不再支持；auth-code 过期 | header 仅 `KuJiang` UA；toc/read 选项加 app 头；**校验成功** 4774ms |
| trap | `kujiang_header_split` |

## 74. Full anti-stall harness (2026-07-29)

| Layer | Control |
|-------|---------|
| Rust | `deep_active` claim；pending/progress deny unsealed；`ledger_gate` 拒假成功；goal 不计 hedged |
| Hook | `legado_hedged_ledger_success` / `legado_l0_only_live_repair` deny；`legado_serial_long_await` ask |
| Docs | `docs/deep-diagnose-anti-stall.md`；discipline §21–23；SKILL traps |

### 74b. Harness 复核发现（同日，跑 `harness verify-change` 才看到）

写完组件不等于生效。按 agent-harness 流程复核后改掉三处：

1. **HookRule 放错位置**：规则先写进 `~/.cursor/audit-logs/custom_rules.json`，
   Windows `audit-hooks hook` 根本不读这个文件 → 三条规则全程没触发。
   证据：塞一条 `ZZPROBEZZ` deny 探针进去，hook 回 `allow`。
   移到 `<repo>/.cursor/audit-hooks/custom_rules.json` 后 deny/ask 全部命中。
   另一个坑：`intercept` 里必须再写一遍 `events`，否则只记录不拦截。
2. **规则没进 git**：`.gitignore` 屏蔽了 `.cursor/audit-hooks/`，换机器就没有护栏。
   已取消忽略并同步到 `legado`（原来只有 legadoSkill 有 §21–23）。
3. **prompt hook 误伤**：`.cursor/hooks.json` 里的 LLM prompt hook 会匹配 agent 自己写的
   测试文本，把自测命令也拦下。确定性规则接管后删掉，只留 `stop` 那条。

仍未解决（工具侧，不在本仓）：

- `harness register` 不扫描项目级 `custom_rules.json`，HookRule 在 `harness audit` 里看不到。
- `harness verify-change` 在仓库根跑 `cargo fmt/test`，本仓 workspace 在 `crates/` → `os error 267`；
  需手动 `cd crates && cargo fmt --all -- --check && cargo test -p source_closeout`。
- Codex projection 里 skill 路径写死成 WSL 的 `/root/Projects/agent-memory\...`，
  Windows 侧 push 全部 conflict（os error 3）。


## 75. m.longtengxiaoshuo.org — fixed (2026-07-29)

Diagnose `layer=ok` after L0–L2 gate pass. Oneshot device verify → `校验成功` (~7s).
Prep only: concurrentRate→1000. Trap `known:layer_ok_device_verify`; skill_fix=0.

## 76. m.mpo18.com PO18脸红心跳 — fixed (2026-07-29)

Root cause: 17mb-style search — GET `s.php?s=` empty; real search is **POST** `s`+`type=articlename`+**GBK**;
result nodes `class.searchresult@p.sone`. Also filled bookInfo name/author (`cataloginfo@h3` /
`infotype@p.0@a`). Device `debug_source` search/detail/toc/content OK; first check timed out at 90s
(CF slow); recheck `timeoutMs=180000` → **校验成功** (~130s).

Novel trap `17mb_post_gbk_search` → SKILL + `diagnose_tips` Search/GBK tip. skill_fix=1.

## 77. m.po18.xyz 荏染柔木 — fixed (2026-07-29)

Rules already had POST `articlename` + `common-bookele`; site is UTF-8 (not GBK).
Failure mode was **CF-slow**: debug ~125s / old check 45s → 校验超时. Prep: `concurrentRate=1000`,
fix broken coverUrl quote, verify `timeoutMs=180000` → **校验成功** (~130s).
Trap `known:cf_slow_check_needs_180s` (same 180s note as §76); skill_fix=0.

## 78. m.popofree.com#🎃 佩蒲斐榕 — fixed (2026-07-29)

Same PO-family POST `articlename` + `.common-bookele`; rules OK. Old fail = 45s check timeout.
Prep concurrentRate=1000, drop empty charset; verify → **校验成功** (~14s).
Trap `known:cf_slow_check_needs_180s`; skill_fix=0.

## 79. m.roushuwu.com 肉书屋 — fixed (2026-07-29)

GBK POST `/search.php` + `.sort_box_list` already correct. Slow host (~30s/req) + multi-page
TOC (4 pages) + old check timeout 15s → 校验超时. Prep concurrentRate=1000; verify keyword
韩娱GD `timeoutMs=300000` → **校验成功** (~91s). Trap `known:cf_slow_check_needs_180s`; skill_fix=0.

## 80. m.xyuzhaiwu7.com 新御宅屋 — fixed (2026-07-29)

PO-family UTF-8 twin: POST `s=` alone → empty; need `type=articlename&s={{key}}`.
Also concurrentRate + verify timeout≥180s (debug ~124s). Device **校验成功** (~137s).
Trap `known:17mb_post_gbk_search`; skill_fix=0.

## 81. novel.cooks.tw#🎃 小说阅读网 — fixed (2026-07-29)

JSON API already worked for search/toc/content; bookInfo lacked name/author; old
`js失效` from template-literal `@js`; coverUrl after init stringify must `JSON.parse`.
Device **校验成功** (~4s). Novel trap `json_api_bookinfo_fields` → SKILL + diagnose_tips.
skill_fix=1.

## 82. wap.biquluo.info 壁落小说 — fixed (2026-07-29)

Search→www.biquluo.info already OK; debug ~2s full path. Fail was 45s timeout flake +
missing bookInfo name. Added `//div[@id='info']/h1` + concurrentRate → **校验成功** (~2s).
Trap `known:cf_slow_check_needs_180s`; skill_fix=0.

## 83. www.88xiaoshuo.net 宝贝小说 — skip (2026-07-29)

POST `/search.html` → HTTP 500 for all keywords (PC); phone list=0. Homepage browse still
has `/book/…`. **disable** per §16 (same cluster as m.88xiaoshuo §36). skill_fix=0.

## 84. www.biqugeabc.com — skip (2026-07-29)

Gate `l1_unreachable` (tcp timeout). `hunt --probe` → `action=empty` (no seeds).
**disable**. Trap `known:dead_skip_without_hunt` (hunt done); skill_fix=0.

## 85. www.noveltri.com/zh-cn/ 三叠书阁 — skip (2026-07-29)

Search `/zh-cn/search?q=` OK (36 hits). Book detail GET → **403** (phone HTTP log + PC);
TOC empty. **disable** (CF wall on detail, not selector). skill_fix=0.

## 86. www.pyzht.com — skip (2026-07-29)

L2 200 but body is SPA shell「精选推荐」(6944B all paths). Not a novel site; search list=0.
Comment’s `18shuwu.com` SSL dead. **disable** parked. skill_fix=0.

## 87. www.qiufengshuwu.com 秋风书屋 — skip (2026-07-29)

POST `/s.html` → **403** (phone+PC); sort/browse OK. **disable** §16. skill_fix=0.

## 88. www.tantanread.com/ 探探书屋 — fixed (2026-07-29)

TOC `href` fake → real URL in `data-c8dcb4a` base64 (gaysay); name `@data-cf3b593`.
Content plaintext `RBGsectionThree-content` (AES `@js` obsolete). Cleared `nextContentUrl`
(was chaining all chapters as pages). Device **校验成功** (~3s). skill_fix=0.

## 89. www.woo16.vip#🎃 原創市集 — fixed (2026-07-29)

GBK POST search OK; ~30s/stage CF-slow. concurrentRate + verify 180s → **校验成功** (~121s).
Trap `known:cf_slow_check_needs_180s`; skill_fix=0.

## 90. www.xguolu88.com#🎃 — skip (2026-07-29)

search.php **404**; browse OK. **disable** §16. skill_fix=0.

## 91. www.yodu.org##出版 — skip (2026-07-29)

POST `/sa` list=0 without login cookie; stripped stored password cookie. **disable**. skill_fix=0.

## 92. www.zei8.vip — skip (2026-07-29)

type=3 TXT download; `downloadUrls` JS IndexOutOfBounds. **disable**. skill_fix=0.

## 93. w.heiyan.com — fixed (2026-07-29)

Oneshot: layer=ok → device **校验成功** (~2s). Trap `known:layer_ok_device_verify`. skill_fix=0.

## 94. m2.tyvvxw.cc — skip (2026-07-29)

Search→bqg123 SPA; TocEmpty; webView single-quote JSON. **disable**. skill_fix=0.

## 95. wap.hanwujinian.com## — skip (2026-07-29)

Search API `uid=0` → list=0 (auth). **disable**. skill_fix=0.

## 96. www.178xs.cc — skip (2026-07-29)

`178yhr` search 404/empty → 178xs. **disable** §16. skill_fix=0.

## 97. www.1redbook.com 挺好的 — fixed (2026-07-29)

Schemeless URL + TOC href 伪装（`data-c9e6f9f` base64，tantan/gaysay 孪生）。
`chapterList=.BCsectionTwo-top-chapter@tag.a` + base64Decode；cover `@_src`。
Device **校验成功** (~3.2s). Trap `known:toc_href_obfuscation`; skill_fix=0.

## 98. www.fuxs1.com 腐小说网 — fixed (2026-07-29)

Empire CMS 模板换皮：`.atts`/`.co-by`→`.tbtls`/`.conbd`；搜索 POST 仍可用；分页 TOC JS OK。
补 `https://`。Device **校验成功** (~1.2s). skill_fix=0.

## 99. www.ijjxsxzw.com 爱久小说 — fixed (2026-07-29)

旧 `m.jjjjxs.com` 搜索规则失效；同域 POST `#searchList@.searchTopic`；详情/目录换皮（`.kv` + `.chapter-list`）；清 `nextContentUrl`（页=章）。
Device **校验成功** (~3.2s). skill_fix=0.

## 100. www.verint.com — skip (2026-07-29)

`bookSourceUrl` 是企业 Contact Center 站；搜索指向 8kana。**disable**. skill_fix=0.

## 101. www.kunnu8.com — skip (2026-07-29)

`?s=` **302→luoxiadushu.com**；分类可浏览。**disable** §16. skill_fix=0.

## 102. www.wwxsc.com 万相书城 — fixed (2026-07-29)

无 scheme 导致抓取失败；补 `https://` + `checkKeyWord=万相`。Device **校验成功** (~2.2s). skill_fix=0.

## 103. www.yybsw.com 夜伴书屋 — fixed (2026-07-29)

同 wwxsc：补 `https://`；清可能串章的 `nextContentUrl`。Device **校验成功** (~2.9s). skill_fix=0.

## 104. serial deep_batch100 — triage (2026-07-29)

`source-cli serial --urls-file deep_batch100.txt --limit 100` (~17.5 min)。
Reports **94/100**：disabled 59 / skipped 17 / failed 13 / migrated 4 / **fixed 1** (`m.shoujix.com#`)。
死站占比高（hunt_empty）；失败队列见 `temp/full_fix/queues/deep_batch100_failed.txt`。

## 105. m.tongrenquan.org/ 同人圈 — fixed (2026-07-29)

serial 报搜索失效；debug 实则 OK。`classid=`→`classid=0`（对齐表单）。Device **校验成功** (~2.2s). skill_fix=0.

## 106. www.shoujix.com / pilisf / po18yq — skip (2026-07-29)

- shoujix：www/m 搜索均空 → disable §16  
- pilisf：17mb articlename+GBK 仍 list=0  
- po18yq：L2 dead + hunt empty  

## Close-out 标准（每轮）


1. **诊断证据**：`diagnose` + phone `debug_source` / fetch → ledger + retro.msg
2. **反思**：`repair_retro.py append`（trap / harness / script_fix / **skill_fix 如实**）
3. **文档**：本节或 dated retro
4. **改进**：新 trap → patch SKILL + Rust/Python **再** next URL（2026-07-28 补：thread trap + `diagnose_tips.rs`）

