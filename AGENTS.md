<!-- agent-memory:codex:start -->
# agent-memory Codex Harness

This project is connected to the agent-memory Harness. Use the synced Codex skills and runtime context before doing non-trivial work.

- At session start for meaningful work, run `audit-hooks codex context --workspace "$PWD" --max-chars 4000` and use the returned context silently.
- Before the final response after meaningful work, run `audit-hooks codex maybe-write-memory --workspace "$PWD" --summary "<concise summary>"` when a useful memory should be preserved.
- Prefer `l3-memory-client`, `agent-harness`, `harness-audit`, `agent-memory-ops`, and `agent-memory-reference` when their descriptions match the task.

Runtime enforcement: Codex hooks call `audit-hooks codex hook-runner` for SessionStart, PreToolUse, PostToolUse, and Stop. HookRule denies are active runtime gates, not advisory text.

Codex-loaded Harness rule digest:

Always-apply MDC excerpts:
- `mdc:book-source-repair-discipline` from `.cursor\rules\book-source-repair-discipline.mdc`; alwaysApply=`true`; Hard discipline for Legado book-source repair (anti fake-fixed / channel / infra)
  # Book-source repair discipline

  Standing rules for any 书源 fix / validate / bulk-check work in this workspace.

  ## Hard bans

  1. **Never claim `fixed` without device verify** (`source-cli repair` → 校验成功).
  2. **Never run fix while bulk owns MCP** — `source-cli check channel` must be idle.
  3. **One phone = one check/debug job** (MCP single-flight). Do **parallel PC** work
     (fetch HTML / triage / prepare patches) and **one batch** `start_check_sources` for many URLs.
     Do not run two `debug_source` / check jobs on the same device.
  4. **Do not repair 发现 (explore)** unless the user explicitly asks. Default verify uses
     `checkDiscovery=false` (MCP) and treats 发现-only failures as success for repair scoring.
  5. **Deep-fix checklist is mandatory** (skill SOT): channel → diagnose → branch → verify → ledger.
     Trust diagnose `layer`. If `fake_detail` (search page as 1 book, wmp8) → fix **search**, not toc.
     「搜索目录失效」is ambiguous — 画本=TOC, 御书屋=dead search; never guess from Chinese alone.
  6. **Wave is triage, not fix**: `repair_wave` may only apply *meaningful* smells; rate-only ≠ fixed.
  7. **Never rewrite search rules** on rate-limit HTML — rely on EWMA cooldown cache.
  8....[truncated]
- `mdc:communication-style` from `.cursor\rules\communication-style.mdc`; alwaysApply=`true`; All agents (Codex, Claude Code, Cursor, etc.) must answer this user in natural Simplified Chinese: conclusion first, short evidence, explain terms, avoid fluff.
  # Communication Style

  适用于所有 agent 与用户的自然语言沟通，包括 Codex、Claude Code、Cursor 和其他接入 Harness 的运行环境。重点场景：最终回复、状态更新、问题解释、风险提示和方案比较。

  ## 依据

  用户偏好来自两层依据：统计覆盖见 `docs/reference/communication-style-evidence.md`，人工阅读和归纳见 `docs/reference/communication-style-investigation.md`。目标不是单纯“短”，而是让用户先看清状态、保留自主判断、获得可用启发、信任证据，并贴合其审计/实现/验证/迭代的做事方式。

  ## 必须做

  1. **先给结论**：第一句回答“是/否/已经做了/还缺什么/建议做什么”。
  2. **说自然中文**：少用英文直译；少用只撑句子、不增加信息的“进行/相关/基于/维度”等词。
  3. **术语先解释**：第一次使用 `MCP`、`SSE`、`schema`、`projection`、`hook` 等词时，用一句话说明它在当前问题里的意思。
  4. **证据短而清楚**：说明“我看到 X，所以判断 Y；我验证了 Z”。区分事实、推测和建议。
  5. **少客套但有人味**：可以自然轻松，不用“当然可以/非常感谢/以下是详细说明”这类空开头。
  6. **自己完成可查证工作**：能查、能跑、能整理的，不要推给用户。确实需要用户选择时，给 2-3 个选项、取舍和建议默认项。
  7. **让启发可执行**：给框架或洞察时，说明它改变了什么判断或下一步动作。
  8. **规则文字也要好懂**：新增或改写规则时，用日常话写标题和正文，不用一个抽象词替换另一个抽象词。

  ## 按场景调整

  - **代码/审计**：先说结果、风险、验证情况，再解释原因。
  - **研究**：先说来源、日期和确定程度；不确定就标出来。
  - **学习解释**：先给结构，再给例子。
  - **决策建议**：先给建议，再说取舍。
  - **关系/情绪问题**：先理清感受、边界和沟通目的，再进入逻辑。

  ## 避免

  - 用黑话替代具体动作：`对齐`、`闭环`、`抓手`、`赋能`、`沉淀`、`心智`、`链路`、`范式`、`方法论`、`治理` 等。
  - 英文直译、翻译腔，或用“进行/相关/基于/维度”等词把句子撑长。
  - 大段堆术语但没有解释。
  - 开头绕圈：“下面我将……”“首先我们来……”“当然可以……”。
  - 规则标题本身用不常见词，例如“牵引力”“落地”。
  - 长篇大论但没有结构，让用户自己捞重点。
  - 把不确定性写成确定结论。
  - 让用户承担本该 Agent 自己完成的查证、运行、整理工作。
- `mdc:host-resource-budget` from `.cursor\rules\host-resource-budget.mdc`; alwaysApply=`true`; Host resource budget — prevent agent shells from freezing the machine with large local LLMs or parallel heavy builds
  # Host Resource Budget

  > 2026-07-19：多 Cursor 线程并行时，一线程拉起 Ollama `qwen3-coder:30b` + 重启 L3，另一线程连续 `cargo build/test`；Windows Event 2004 虚拟内存耗尽。

  ## Hard gates (audit-hooks)

  These fire on `beforeShellExecution` (Windows Cursor uses `audit-hooks.exe`):

  | Rule ID | Action | Blocks |
  |---|---|---|
  | `host_resource_large_local_llm` | DENY | `llama-server`, local models ≥30B (e.g. `qwen3-coder:30b`) |
  | `host_resource_l3_stack_restart` | ASK | `start-l3-services.sh` stop/restart |
  | `host_resource_cargo_release` | ASK | `cargo … --release` |
  | `host_resource_concurrent_heavy` | ASK | another rustc/cargo/llama-server already running |

  ## Before heavy work

  Heavy = `cargo build/test/clippy --release`, compile-triggering `cargo run`, Ollama/`llama-server`, L3 full restart, WhisperX / large Python models, full `gradle` test, long LLM pipelines.

  1. Check load: `tech/resource-control/diagnostics/check-heavy-procs.ps1` (or look for rustc/cargo/llama-server).
  2. If another heavy job is running or memory is tight: **do not start another**; reuse artifacts / small model / ask the user.
  3. Diagnose/analyze: **never** default to 30B local models; use cloud or ≤14B / statistical fa...[truncated]
- `mdc:jason-dev-practices` from `.cursor\rules\jason-dev-practices.mdc`; alwaysApply=`true`; Development baseline for coding tasks. Keep small; detailed rules live in docs/reference/dev-practices-reference.md.
  # Dev Practices

  This file is the always-loaded baseline. For non-trivial coding work, debugging, tests, interface/behavior changes, docs mismatches, or 3+ touched files, also read `docs/reference/dev-practices-reference.md`.

  MUST = required. SHOULD = recommended; skip only with a reason.

  ## Work Order (MUST)

  1. Read project work context. In this repo the source file is `.cursor/work-context.mdc`; Codex may receive it through `audit-hooks codex context`.
  2. Use `rg` to find any directly related shared rule, skill, or local doc; read only the matched file/section.
  3. Read local docs named by the task.
  4. Confirm facts with `rg`, config, tests, MCP tools, Bazel, or project scripts before editing.
  5. Choose action by risk, edit in small verified steps, then report evidence.

  Ask the user only when local files/tools cannot answer safely, direction is ambiguous, or the action is high-risk.

  ## Action Rules (MUST)

  - Low-risk reversible actions: run directly and inspect results.
  - Higher-risk actions: inspect structure and explain intent first.
  - If a change touches 3+ files or public behavior, share a short plan and proceed in small verified steps.
  - After one failure,...[truncated]
- `mdc:memory-rules` from `.cursor\rules\memory-rules.mdc`; alwaysApply=`true`; Minimal knowledge routing rules for all agents: decide whether durable knowledge belongs in work context, .mdc, docs/skills, or L3 memory.
  # Memory Rules

  ## 适用范围

  这些规则适用于 Codex、Claude Code、Cursor 和其他接入 Harness 的 agent。记忆系统负责存储和搜索，hook 负责拦截危险写法；本文件只负责判断“这条知识该不该保存、保存到哪里、保存前查什么”。

  ## 共享源和投影

  `shared-rules/*.mdc` 是共享规则源，不是 Cursor 专用规则。`.mdc` 是本项目沿用的规则文件格式；不同工具用不同方式读取同一份源规则。

  | 目标 | 读取方式 |
  |------|----------|
  | Cursor 项目 | 同步到项目 `.cursor/rules/*.mdc` |
  | Claude Code | 同步到 `~/.claude/rules/*.mdc` |
  | Roo Code | 同步到 Roo 全局 rules 目录 |
  | Codex | 投影到 `AGENTS.md`、`agent-memory-reference` 和 Codex context |

  写规则时先写通用行为；只有路径、hook 事件、Windows bridge、CLI 入口这类工具差异，才单独点名 Codex、Claude Code 或 Cursor，并链接到对应文档。

  ## 不用内置聊天记忆

  记忆写入只走本项目管理的文件和 CLI。

  - 不要调用内置 memory 工具。
  - L3 写入必须使用 `audit-hooks l3 write`；不要直接调用 MemPalace、ChromaDB 或旧的 `claude-mem` 写入命令。
  - 在其他 repo、WSL 或 Windows 相关工作区，先用 `command -v audit-hooks` 或 `audit-hooks l3 test` 确认 CLI 可用。
  - Windows 入口必须按目标工具验证：Codex Windows home 用 `docs/guides/codex-harness-global.md` 的 `wsl.exe` bridge；Cursor Windows hooks 先看 `docs/reference/windows-hook-entrypoints.md`，并跑 `audit-hooks doctor cursor-hooks --project <repo>`。

  ## 存到哪里

  只保存长期有用、会被复用的知识；临时输出、一次性操作、代码里已经清楚表达的信息不要保存。

  | 类型 | 存储位置 | 例子 |
  |------|---------|------|
  | 当前活跃任务 | 项目 work context；源文件是 `.cursor/work-context.mdc` | 当前正在重...[truncated]
- `mdc:work-context` from `.cursor\rules\work-context.mdc`; alwaysApply=`true`; Current active task for this project. Read on conversation start.
  # Work Context

  当前活跃任务：L3 记忆系统重构与 Dashboard SSE 面板

  ## 最近改动

  - **Agent Harness 全栈自循环审计系统**：`audit-hooks harness` 命令组（register/audit/apply/diagnose），实现组件注册表、9 项健康检查、会话评分卡、自动修复，并集成至 Dashboard 和 `cron harness-audit`。
  - **Harness L2 + RFC-003 修正队列**：`harness diagnose --queue-correction` / `harness queue-correction` 写入 `correction.mdc` 与 `harness_audit_fix`；Shell 执行含 `audit-hooks` 的命令可清 pending；`harness propose` 输出新组件模板；L2 成功时向 `rule_changes` 写入 `ExperimentStarted` 记录。
  - **L3 记忆系统自动化维护**：`audit-hooks l3 gc` (定时合并碎片) 和 `audit-hooks l3 promote` (基于 Hook 拦截的高频记忆升维)，并集成至 `audit-hooks cron gc`。
  - **LLM Gateway embedding 集成 + Docker 桥接**：LLM Gateway 新增 `/v1/embeddings` 路由；Graphiti Docker 通过 socat 桥接（WSL :9101→Windows :9100）访问 LLM Gateway embedding；`start-l3-services.sh` 自动启动桥接；Ollama proxy 降为 legacy
  - **L3 PID 进程管理 + 动态端口**：`pid_util.rs` 共享模块（PID 文件 + /proc/cmdline 身份验证 + 优雅 SIGTERM→SIGKILL）；`l3 stop` 子命令；`l3 export-config` 导出配置给 Windows consumer；`load_ports` 改用 jq 解析；所有端口变量加引号
  - **L3 搜索上下文传递 + 代码质量修复**：`build_search_args` 接收 `SearchContext`，按工具名传递 `group_ids`（Graphiti search_memory_facts/search_nodes）和 `dataset_name`（M-flow）；sync export_graphiti 静默失败改为 eprintln；ingest conversa...[truncated]

Other MDC references:
- `mdc:coding-patterns` from `.cursor\rules\coding-patterns.mdc`; alwaysApply=`false`; 并发编程和系统设计模式、调试方法论 — 滑动窗口、环形缓冲区、共享状态、线程安全、时间窗口过期、缓存淘汰、subagent 并行、测试隔离、错误诊断、输出路径gitignore同步。Concurrency, sliding window, ring buffer, thread safety, cache eviction, parallel subagent, test isolation, error diagnosis, output pa...[truncated]
- `mdc:dev-tool-tips` from `.cursor\rules\dev-tool-tips.mdc`; alwaysApply=`false`; 开发工具技巧和环境配置 — WSL、CRLF、换行符、Vitest、Mermaid、StrReplace、Cursor hooks、preToolUse、Grafana CLI、标准工具优先、schtasks、Task Scheduler、否定断言验证。Development tool tips, WSL line endings, Mermaid validation, StrReplace strategies, Cursor hook fo...[truncated]
- `mdc:docs-organization` from `.cursor\rules\docs-organization.mdc`; alwaysApply=`false`; Documentation directory structure standard. Read when creating, moving, or auditing docs/ in any project.

User rule messages:
- `user_rule:forbidden_jargon`: 回复含禁用词（user_rules + communication-style: 中文语境沟通）
- `user_rule:no_mdc_read`: 本轮没有读取 work-context.mdc（user_rules: MANDATORY first read）
- `user_rule:not_chinese`: 回复可能不是中文（user_rules）
- `user_rule:update_memory_forbidden`: 使用了 update_memory（user_rules）

<!-- agent-memory:codex:end -->

## Legado Book-Source Engineering & Repair Heuristics

When designing, auditing, or repairing Legado book sources:
1. **Diagnosis Chain**: Always diagnose in strict order: `Search -> Detail -> TOC -> Content`. Never jump to fixing TOC or Content before verifying search responses.
2. **Instant Heuristics**:
   - Empty search with HTTP 200: Check for frequency limit (`alert("搜索间隔")`), captcha, or Cloudflare challenge before rewriting selectors.
   - Ads inside content: Prefer `@ownText` to automatically discard child-tag ads.
   - Dynamic/Blank pages: Append `,{"webView": true}` to the request URL (for sub-rules like `chapterUrl`, use `##$##,{"webView":true}`).
   - Reversed catalog: Prepend `-` to the list selector (e.g. `-ul.chapters li`).
   - Negative constraints: Never output `ruleContent.prevContentUrl` (engine lacks this field); never extract `@value` directly from `<select>` (use `select option@value`).
3. **Simplicity First**: Never over-engineer. Prefer CSS over regex, and regex over complex JavaScript. Use `java.log(result)` to trace real execution values rather than guessing.
