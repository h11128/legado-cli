# Source Repair Session Log — 2026-07-26

Parent chat: [`f14f2834-eeb7-45bb-b325-9ba29e01c2db`](file:///C:/Users/h1112/.cursor/projects/e-Projects-legado/agent-transcripts/f14f2834-eeb7-45bb-b325-9ba29e01c2db/f14f2834-eeb7-45bb-b325-9ba29e01c2db.jsonl)

Device MCP (late session): `http://10.0.0.139:1236/mcp` (token `1234`). Early session often used `10.0.0.43`.

This file is the **canonical local record** of book-source repair work in this thread + related subagents. Pair with:
- `docs/source-repair-retrospective.md` (why slow / infra gaps)
- `temp/full_fix/repair_session_index.json` (machine index)

---

## 1. Parent-thread phases (repair-related)

| Phase | What happened | Artifacts |
|-------|----------------|-----------|
| Setup | Clone/link legadoSkill, Cursor MCP, legado-book-source skill | `legadoSkill/`, `~/.cursor/mcp.json` |
| Sample fix | Fixed `m.bqgcn.net` (@put JSON + kind) | `temp/bqgcn_fixed.json` |
| Group cleanup | User tags 资源/优质/字数/特别; cleared noise groups | `temp/group_plan.json` |
| Enabled validate | Subagent debug_source on user-group enabled sources | `temp/source_validate_progress.json` |
| Dead mirrors | Disabled ~20 dead DNS/mirrors | device state |
| Bulk checkalgo | OOM-safe check + AIMD/token/Bloom etc. in legado app | commits `849363314`, `588a8a51e` |
| Full check | Export 4741 → precheck 4300 alive; runners collided | `temp/full_check/` |
| Parallel fix waves | fix_a / fix_b claimed fixed without durable verify | `fix_log.jsonl`, `fix_log_b.jsonl` |
| Retest claimed | book18 OK; ijjj/pow still TOC fail | `verify_fixed.json` |
| Unfixed plan | P0–P3 triage (read-only audit agent) | `unfixed_repair_plan.md` |
| P0 re-fix | Dedicated agents: pow cleared tocUrl; ijjj narrowed tocUrl + name | `fix_pow.json`, `fix_ijjj.json` |
| Skill + scripts | repair skill + `repair_source.py` + first retro | skill SOT + `scripts/repair_*` |

---

## 2. Subagents that touched source repair / validate

| Agent id | Role | Outcome (as of session end) | Local log / notes |
|----------|------|-----------------------------|-------------------|
| [Validate user groups](e9f65a57-b7f6-4a40-8122-0c547455449d) | debug_source PASS/PARTIAL/FAIL on enabled tagged sources | Incomplete (~52/55); PASS21 / PARTIAL2 / FAIL29 | `temp/source_validate_*` |
| [Fix batch A](172eea06-bb58-47cd-8efc-1f6ddb188809) | Fix up to 6 + skip rest | Claimed 6 fixed + many skips → `fix_log.jsonl` | **No verify gate**; ijjj/pow later false; bengben/zxcs/aiqu still **unverified** |
| [Fix batch B](7f7191cd-e283-4312-aacd-ebff25beebb9) | Fix ≤6, no start_check (channel reserved) | Claimed ijjj/book18/pow fixed → `fix_log_b.jsonl` | Retest later: only book18 true |
| [Unfixed audit](ef09fe43-37c4-464d-9c6f-0e6058f20bd3) | Read-only plan for remaining fails | **Done** — 32-source table + deep steps for bengben/aiqu/zxcs | `unfixed_repair_plan.md` |
| [Fix powanjuan TOC](bfb9ff72-ffb3-4cf7-af2d-2d822e44b84e) | Single-URL fix + verify | **Pass** (~2.3s); clear `tocUrl` | `fix_pow.json` |
| [Fix ijjjxsw TOC](17dbd633-91a1-4d3e-a17d-2b7ee3468cdd) | Single-URL fix + verify | **Pass** (~3.25s); narrow tocUrl; name `\|\|`/`##` split; 20s search gap | `fix_ijjj.json` |

Non-repair subagents in same parent (checkalgo / APK / review) listed in `repair_session_index.json` for completeness only.

---

## 3. Per-source repair chronicle (verified truth)

### 3.1 https://www.book18.org/ — 情色小说

| Step | Actor | Result |
|------|-------|--------|
| fix_b save | [Fix batch B](7f7191cd-…) | searchUrl `page={{page}}`; name `tag.a@text` |
| Device retest | Parent | **校验成功** (`verify_fixed.json`) |
| Status | | **FIXED (verified)** |

### 3.2 https://www.powanjuan.cc — 破万卷

| Step | Actor | Result |
|------|-------|--------|
| Early debug | Parent / validate | Search/explore issues |
| fix_b save | Fix batch B | explore/checkKeyWord/toc tweaks; still tagged 搜索目录失效 |
| Device retest | Parent | **仍失败** 搜索目录失效 |
| Root cause | [Fix powanjuan TOC](bfb9ff72-…) | `tocUrl=span.read a@href` → content page `/…/1.html`, no `.catalog` |
| Fix | same | clear `tocUrl`; concurrentRate 1000; clean group |
| Verify | same | attempt1 search fail (post-debug); attempt2 **pass** 157 chapters, 2313ms |
| Status | | **FIXED (verified)** → `fix_pow.json` |

### 3.3 https://www.ijjjxsw.com — 爱久久网

| Step | Actor | Result |
|------|-------|--------|
| fix_b save | Fix batch B | claimed redesign selectors; group still had 目录失效 tags |
| Device retest | Parent | **仍失败** 搜索目录失效, 发现目录失效 |
| Root cause | [Fix ijjjxsw TOC](17dbd633-…) | (1) `tocUrl=a@href##…##` → homepage `/`; (2) name `\|\|`+`##` char-insert; (3) 20s search rate limit |
| Fix | same | narrow tocUrl; `h3:first@text##《|》`; author `.kv a@text`; concurrentRate 1000 |
| Verify | same | attempt1 搜索失效 (20s gap); attempt2 **pass** 108 chapters, 3250ms |
| Status | | **FIXED (verified)** → `fix_ijjj.json` |

### 3.4 Unverified batch-A “fixed” (do not trust yet)

From [Fix batch A](172eea06-bb58-47cd-8efc-1f6ddb188809) `fix_log.jsonl`, still **without** `repair_source.py verify`:

| URL | Claim | Audit ([Unfixed audit](ef09fe43-37c4-464d-9c6f-0e6058f20bd3)) |
|-----|-------|------|
| `https://www.bengben.com#🎃` | i7uu→bengben rewrite | Still top repair queue; detailed 6-step plan |
| `http://www.zxcs.info/` | → zxcs.click rewrite | Prefer 网盘/webview; not auto-download |
| `https://www.627txt.com##@尐哖` | download selectors | Migrate `aiqu226.com` + card search |

Next repair queue after P0: **bengben → aiqu → zxcs**, each with verify+log. Dead/WAF/non-book rows in plan → disable, do not thrash.

### 3.5 Skips (from fix logs; not re-verified individually)

Logged in `fix_log.jsonl` / `fix_log_b.jsonl`, including: trxs anti-bot, tiexue dead, shenmo 401, lmeee repurposed, wangshu safebrowse, UAA non-book, Maoyan/Jiuyue API dead, Jinjiang/QQ walls, timeouts, video sites (taopian/uku/ifun), etc.

**Caveat:** batch “fixed” rows for bengben / zxcs / 627txt in `fix_log.jsonl` were **not** confirmed by `verify_fixed.json` in this session — treat as **unverified claims** until `repair_source.py verify`.

---

## 4. Artifact index (`temp/full_fix/`)

| Path | Meaning |
|------|---------|
| `fix_log.jsonl` | Batch A claims + skips |
| `fix_log_b.jsonl` | Batch B claims + skips |
| `verify_fixed.json` | Post-zombie kill retest of 3 claimed fixes |
| `fix_pow.json` / `fix_ijjj.json` | Verified P0 fix logs |
| `unfixed_repair_plan.md` | P0–P3 plan (partially stale after P0 re-fix) |
| `src_*.json.txt`, `pow_*.html`, `*_live.html` | Ad-hoc HTML/source dumps |
| `apply_fixes_b.py` | One-off apply script (not reusable infra) |

Transcripts (Cursor):  
`…/agent-transcripts/f14f2834-…/subagents/<id>.jsonl`

---

## 5. Process bugs observed (must not repeat)

1. **save ≠ fixed** — fix_b wrote “fixed” while group still carried 目录失效; retest failed.
2. **Parallel owners on one MCP** — full_check zombies + fix agents → false timeouts / blocked verify.
3. **Stale MCP IP in skill** — `10.0.0.43` vs live `139` caused reconnect thrash.
4. **One-off `.local-scripts/inspect_*.py` explosion** — dozens of throwaway probes instead of `repair_source.py fetch` / debugger.
5. **Knowledge unused** — ESSENTIAL_KNOWLEDGE / CSS rules / TOC docs / local debugger barely consulted during P0 thrash.
6. **No single session ledger** until this file — outcomes scattered across jsonl + chat.

---

## 6. Required workflow going forward

```
triage → fetch (headers) → minimal save → verify (--cooldown) → log
```

Scripts: `scripts/repair_source.py` + `mcp_client.py` + `repair_helpers.py`  
Skill: `E:/shared-skills/legado-book-source-repair/SKILL.md`  
Before deep CSS work: skim `docs/ESSENTIAL_KNOWLEDGE_SUMMARY.md`, `docs/TOC_PAGINATION_RULES.md`, `assets/css选择器规则.txt`. Optional local dry-run: `debugger/test_universal.py` (not a substitute for device verify).
