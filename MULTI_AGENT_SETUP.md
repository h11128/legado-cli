# Multi-agent setup — legado-book-source

## Source of truth

`E:/shared-skills/legado-book-source/SKILL.md`

Keep agent copies byte-identical to the SOT (same pattern as `pe-task-runner`).

## Installed copies

| Agent | Path |
|-------|------|
| Claude Code | `~/.claude/skills/legado-book-source/SKILL.md` |
| Codex | `~/.codex/skills/legado-book-source/SKILL.md` (+ `agents/openai.yaml`) |
| Cursor | `~/.cursor/skills/legado-book-source/SKILL.md` |
| Hermes | `%LOCALAPPDATA%/hermes/skills/software-development/legado-book-source/SKILL.md` |
| Hermes (import mirror) | `%LOCALAPPDATA%/hermes/skills/agent-memory-imports/legado-book-source/SKILL.md` |

## Device MCP (`legado`)

- **SOT:** `config/mcp_defaults.json` (update when phone LAN IP changes)
- Header: `X-Legado-Token` (same as Web token)
- Repair workflow: skill `legado-book-source-repair` + `source-cli dig / repair`

| Agent | Config entry |
|-------|----------------|
| Cursor | `~/.cursor/mcp.json` → `mcpServers.legado` |
| Codex | `~/.codex/config.toml` → `[mcp_servers.legado]` |
| Claude Code | `~/.claude.json` → `mcpServers.legado` (`type: http`) |
| Hermes | `%LOCALAPPDATA%/hermes/config.yaml` → `mcp_servers.legado` |

## Knowledge / Engine

- Repo: `E:/Projects/legadoSkill`
- Official app junction: `legadoSkill/legado` → `E:/Projects/legado`
- CLI Engine: `source-cli` (Rust CLI in `crates/source-cli`, install via `cargo install --path crates/source-cli` or `source-cli install`)
- References & Standards: `skills/legado-book-source/references/`

## Re-sync after SOT edits

```bash
# SOT in E:/shared-skills
for skill in legado-book-source legado-book-source-repair; do
  SOT="E:/shared-skills/$skill/SKILL.md"
  cp "$SOT" "$HOME/.claude/skills/$skill/SKILL.md" 2>/dev/null || true
  cp "$SOT" "$HOME/.codex/skills/$skill/SKILL.md" 2>/dev/null || true
  cp "$SOT" "$HOME/.cursor/skills/$skill/SKILL.md" 2>/dev/null || true
  cp "$SOT" "$LOCALAPPDATA/hermes/skills/software-development/$skill/SKILL.md" 2>/dev/null || true
  cp "$SOT" "$LOCALAPPDATA/hermes/skills/agent-memory-imports/$skill/SKILL.md" 2>/dev/null || true
done
```

When `audit-hooks sync` / `audit-hooks codex sync` is healthy again, prefer that for Claude/Cursor/Codex; Hermes still needs the manual copy above.
