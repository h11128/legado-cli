# Retro: Domain migrate + video/file repair (2026-07-26 evening)

Parent: [`f14f2834…`](file:///C:/Users/h1112/.cursor/projects/e-Projects-legado/agent-transcripts/f14f2834-eeb7-45bb-b325-9ba29e01c2db/f14f2834-eeb7-45bb-b325-9ba29e01c2db.jsonl)  
Companion docs: `source-repair-retrospective.md`, `domain-migrate-apply-2026-07-26.md`,  
`source-repair-session-phase-migrate-video-2026-07-26.md`

## Conclusion

**慢的原因不是选择器“特别难”，而是：范围一次铺太开 + 换域后还用旧规则盲校验 + debug成功却反复改错字段，没有立刻用 HTTP 日志对比 check。**

有效成果（设备校验成功）其实只要三步到位就会很快：换域 → 按新 HTML 改 search/list/content → verify。  
U 酷真正根因是 **搜索 bookUrl 空 → 搜索页被当成详情 infoHtml**；用 `get_http_logs` 看「只有 search、没有 detail」本可在 2 分钟内定性，却被烧在 `||` / `@css` / JS 试验上。

墙钟粗估（本阶段「继续」起）：约 **25–40 分钟**（含建脚本、多源并行试探）。其中 **≥40% 可避免**。

---

## Scope And Evidence

| Item | Evidence |
|------|----------|
| Migrates | `temp/full_fix/domain_migrate_*.json` |
| Verify | zxcs/aiqu/uku `校验成功` in MCP progress dumps |
| U酷 HTTP | `#3468` search-only vs `#3471+#3472` search+detail |
| Scripts added mid-flight | `repair_domain_hunt/migrate`, `video_*` |
| Type truth | App `BookSourceType`: 3=file, 4=video |

Token/wall estimates only — no billing export.

---

## Completed Work And Cost

| Work | Value | Cost feel |
|------|-------|-----------|
| Domain hunt seeds + L2 probe | 证明铁血无后继、zxcs/aiqu 可迁 | 低（应做） |
| `repair_domain_migrate.py` | 改 URL + 删旧 + verify | 中（应做，一次写好） |
| zxcs 规则适配新站 | **校验成功** | 中（必要） |
| aiqu search-card + body@text | **校验成功** | 中（必要） |
| Video skill + divert L0 | 分流正确方向 | 低 |
| U酷 downloadUrls + bookUrl | **校验成功** | 本应低，实际高 |

---

## Detours (为什么搞了这么久)

| # | Detour | What should have happened | Waste |
|---|--------|---------------------------|-------|
| 1 | 「继续」= 换域 + 建 video flow + 修淘片/南瓜 一把做 | 先闭环 migrate 两个小说源，再开 video | 范围膨胀 |
| 2 | Migrate 后立刻 verify，旧 selector 全废 | 换域后 **强制 PC/HTML 看搜索表单**，再 save | 一轮空校验 |
| 3 | 把 L2 200 当“站活了就能用旧规则” | L2 ≠ App 搜索/正文 | 认知错 |
| 4 | U酷：debug 有 m3u8，check 空，连改 downloadUrls/`\|\|`/`@css`/JS | **先 `get_http_logs`**：仅 search = bookUrl/infoHtml 问题 | ~10–15 min |
| 5 | 误认 type=3 为 video | 查 `BookSourceType.kt`：3=file 4=video | 概念混乱 |
| 6 | 大量 inline `python - <<PY` | 应固化 `repair_debug_vs_check` / session_log | 不可复用 |
| 7 | 三源已成功后仍铺淘片/南瓜 | 硬停或另开任务；本阶段只记 smell | 收尾拖延 |
| 8 | **过程中无实时 session ledger** | 每源 ≤5 行 timeline 落盘 | 你现在才看到“为什么久” |

---

## Principle Failures

1. **Target 2–5 min / hard stop 10 min** — 本阶段未按源计时，也未在 10 分钟切 skip。  
2. **Prefer repair_* over ad-hoc** — 仍大量一次性探针。  
3. **Device verify authoritative** — 做了，但 debug≠check 时未走最短诊断。  
4. **One job per wave** — migrate 与 video 基建与多源修复缠在一起。  
5. **No live process log** — 违反“每次要有记录”的用户预期（此前有总 retro，但本阶段缺增量）。

---

## Harness Component Gaps → Improvements

| Gap | Component | Change |
|-----|-----------|--------|
| No per-phase ledger | script + skill | `repair_session_log.py` + skill mandate |
| debug≠check 慢诊断 | script | `repair_debug_vs_check.py`（对比 HTTP：缺 detail 则标 `bookUrl_infoHtml_trap`） |
| Type 3/4 混淆 | skill | video skill 写明 file vs video |
| Migrate 后盲 verify | skill/discipline | 规则：migrate 后必须 HTML 对照 search 表单再 verify |
| 过程不落盘 | discipline | 每源结束必须 `session_log append` |

---

## Action Items (executable)

1. **Session log CLI** — `python scripts/repair_session_log.py append …`  
   Proof: `python scripts/repair_session_log.py show --tail 5`
2. **debug vs check** — `python scripts/repair_debug_vs_check.py --url URL --key KEY`  
   Proof: 对 U 酷类源应打印 `bookUrl_infoHtml_trap` 或 `ok` + HTTP 对
3. **Discipline + skills update** — migrate 后 HTML；debug≠check→先 HTTP logs  
   Proof: files exist under `.cursor/rules` + skills
4. **This retro + phase session log committed to legadoSkill docs**  
   Proof: paths below exist

---

## Remaining Unknowns

- 淘片 SSL 主机名不匹配是否仅 PC；手机能否稳定拉搜索 HTML  
- 南瓜 type=0 是否历史误标，还是故意用文本规则播视频  
- 本阶段精确 token 消耗（无导出）
