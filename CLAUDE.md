# LegadoSkill Claude Guidelines

## Project Context
This workspace provides the engine, CLI, skills, and references for developing and repairing Legado (阅读) book sources, integrated with real Android devices via MCP.

## Core Engineering & Repair Heuristics
1. **Strict Diagnostic Chain**: Always diagnose and repair in order: `Search -> Detail -> TOC -> Content`. Never modify TOC or Content rules before confirming search output.
2. **Instant Troubleshooting Heuristics**:
   - **Empty search with HTTP 200**: Check HTTP logs for rate-limits (`alert("搜索间隔")`), captcha, or Cloudflare before touching selectors.
   - **Ads in content**: Use `@ownText` to capture pure text and discard child-tag ads.
   - **Dynamic/JS-rendered content**: Append `,{"webView": true}` to the request URL (for sub-rules like `chapterUrl`, use `##$##,{"webView":true}`).
   - **Reversed catalog**: Prefix `-` to the selector (e.g. `-ul.list li`).
   - **Negative constraints**: Never output `ruleContent.prevContentUrl` (engine lacks this field); never extract `@value` directly from `<select>` (use `select option@value`).
3. **Pragmatic Simplicity**:
   - Prefer CSS over regex, regex over JS, and built-in `java.*` over custom classes.
   - Use `java.log(result)` to verify runtime values instead of guessing.
   - Always verify fixes against the real device via `source-cli` or MCP before claiming resolution.
