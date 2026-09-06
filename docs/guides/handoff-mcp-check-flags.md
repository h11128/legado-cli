# Handoff prompt — MCP check flags + continue source repair

Paste the block below into another agent thread.

```
## Role
You work in two repos on the same machine:
- App: E:/Projects/legado  (Android Legado fork, applicationId MUST stay com.legado.app)
- Companion: E:/Projects/legadoSkill  (PC repair scripts + MCP defaults)

Read first:
- E:/Projects/legado/.cursor/rules/work-context.mdc
- E:/Projects/legado/.cursor/rules/book-source-repair-discipline.mdc
- E:/Projects/legadoSkill/skills/legado-book-source-repair/SKILL.md
- E:/Projects/legadoSkill/config/mcp_defaults.json  (active phone MCP)

## Goal A — Finish MCP check-option overrides (App + PC)

App already partially done:
- McpCheckTools.start_check_sources accepts checkDiscovery, checkSearch
- McpSourceCheckJob.start accepts those two overrides and restores after job

COMPLETE this so ALL CheckSource toggles can be overridden per MCP call
(same pattern: save previous → apply override if non-null → restore in finally):

From io.legado.app.model.CheckSource:
| Param (MCP JSON)   | Field                 | Meaning                         | Repair-wave default |
|--------------------|-----------------------|---------------------------------|---------------------|
| checkDomain        | checkDomain           | probe domain reachable          | false (unless asked)|
| checkSearch        | checkSearch           | search path                     | true                |
| checkDiscovery     | checkDiscovery        | explore/发现                    | false (unless asked)|
| checkInfo          | checkInfo             | book info page                  | true                |
| checkCategory      | checkCategory         | TOC / 目录                      | true                |
| checkContent       | checkContent          | chapter content / 正文          | true                |
| timeoutMs          | (already)             | per-source timeout              | keep                |
| keyword            | (already)             | search keyword                  | 我的                |
| threadCount        | (already)             | phone worker threads            | keep / ~8           |
| enabledOnly        | (already)             |                                 | false for repair    |

Also expose optional wSourceComment if cheap; not required.

Requirements:
1. Extend McpCheckTools inputSchema + McpSourceCheckJob.start for ALL of the above bools.
2. Restore ALL previous CheckSource values in job finally (even on cancel/error).
3. Snapshot/start message should echo which flags are active.
4. Update PC scripts to pass defaults above:
   - E:/Projects/legadoSkill/scripts/repair_check.py  (check_args)
   - repair_wave.py / repair_one.py / repair_domain_migrate.py / repair_debug_vs_check.py / repair_search_wave.py / repair_bench10.py / repair_source.py verify
5. Document in legadoSkill/legado/api.md (MCP start_check_sources params).
6. After App change: build/install APK to the phone that serves MCP (10.0.0.139:1236) OR tell user to install — do not claim MCP flags work until device has the build.
7. Do NOT change applicationId.

## Goal B — Continue repairing ~20 failing sources (after A or with PC scoring)

Policy (hard):
- Do NOT repair 发现/explore unless user explicitly asks. checkDiscovery=false.
- Parallel = PC HTML/patch workers + ONE start_check_sources batch (phone multi-thread inside). NEVER two debug_source / check jobs on same phone.
- Time: known 1–2 field fix ≤1 min (aim 30–45s); HTML/search form 2–3 min; hard stop 5 min/source; log every attempt via repair_session_log.py.
- Never claim fixed without device verify success (or is_repair_success with ignore_discovery).
- Never invent inspect_*.py; use repair_* scripts.

State / artifacts:
- URL list: E:/Projects/legadoSkill/temp/full_fix/bench20_urls.txt
- Last auto wave: temp/full_fix/wave20_report.json (mostly 搜索失效)
- Search-form wave: temp/full_fix/wave20_search.json (many 403/404; patched 123du + ihuaben still not green)
- Ledger: temp/full_fix/repair_session_ledger.jsonl
- Seeds/migrate: scripts/repair_domain_hunt.py, repair_domain_migrate.py

Suggested execution order:
1. Finish Goal A + install APK.
2. python scripts/repair_source.py channel   # must be idle
3. Re-pick 20 unique-host enabled sources with 搜索/目录/正文 fails; exclude L0 denylist (config/verify_skip_rules.json), exclude official walls, skip hosts that 403/404 on homepage.
4. python scripts/repair_wave.py --urls-file … --patch-workers 4 --thread-count 8
   (checkDiscovery false; other flags as table)
5. For needs_deep with 搜索失效: python scripts/repair_search_wave.py (parallel PC form hunt) then ONE batch verify.
6. For 搜索目录失效 / 正文失效 with search hits: clear bad tocUrl / content selectors; batch verify.
7. Divert video/file (type 3/4) to legado-video-source-repair; hunt dead domains via repair_domain_hunt — do not burn budget on unfixable hosts.
8. Append ledger + short phase note under docs/.

Success definition (default repair):
- check.success == true, OR
- message only contains 发现* failures (PC helper repair_check.is_repair_success)

## Out of scope unless asked
- Fixing exploreUrl / 发现 rules
- Full library bulk check while repair holds channel
- Parallel MCP check jobs on one phone

## Deliverables
1. App MCP supports full check flag overrides; PC scripts use repair defaults.
2. Wave report JSON with fixed / needs_deep / wall_s.
3. Brief Chinese or English note of what passed device verify.
```
