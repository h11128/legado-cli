# Deep-diagnose anti-stall — full measure matrix (2026-07-29)

Problem: agent ends a turn mid-URL (background `diagnose` / MCP 10060), over-probes
auth/ad walls, or writes hedged ledger lines like `校验成功或见上` after failed migrate verify.

## Layered controls

| Layer | What | Enforcement |
|-------|------|-------------|
| **CliCommand / Rust** | `deep_active.json` claim on `diagnose` / `repair oneshot` | Soft claim always |
| **CliCommand / Rust** | `closeout pending` + `progress next` | **DENY** if unsealed claim |
| **CliCommand / Rust** | `retro append` fixed/skip/fail | Seals `deep_active` |
| **CliCommand / Rust** | `closeout claim\|heartbeat\|release\|clear-active` | Manual escape + heartbeat |
| **CliCommand / Rust** | `ledger append` | **DENY** hedged results (`ledger_gate.rs`) |
| **CliCommand / Rust** | Goal `fixed_count` | Ignores hedged 「校验成功*」 |
| **HookRule** | `legado_hedged_ledger_success` | beforeShell **deny** (legadoSkill) |
| **HookRule** | `legado_serial_long_await` | beforeShell **warn** on long sleep |
| **hooks.json prompt** | hedged ledger / long await / stop | deny / ask / stop ask |
| **MdcRule** | discipline §21–23 | Always-loaded agent guidance |
| **Skill** | traps `agent_turn_stall`, `hedged_ledger_success` | Repair playbook |
| **Work context** | deep anti-stall one-liner | Session SOT |

## Operator commands

```bash
# see if turn can pick next URL
source-cli closeout pending
source-cli closeout status

# after diagnose crash / MCP blip without finishing
source-cli closeout release --url 'https://…' --status skip
# then still write honest ledger + retro

# claim manually (rare)
source-cli closeout claim --url 'https://…' --note 'mcp-debug'
```

## Honest ledger rules

| Situation | Ledger result |
|-----------|---------------|
| Device check message `校验成功` | `校验成功` |
| migrate `verify_ok:false` | `fail:…` or `skip:…` — **never** success hedge |
| Auth / ad / type=2 out of scope | `skip:…` |
| MCP transport dead | `skip:mcp_transient` then next URL |

## Files

- `crates/source-closeout/src/active.rs`
- `crates/source-closeout/src/ledger_gate.rs`
- `crates/source-closeout/src/pending.rs`
- `.cursor/rules/book-source-repair-discipline.mdc` §21–23
- `skills/legado-book-source-repair/SKILL.md`
- `.cursor/hooks.json`
- `~/.cursor/audit-logs/custom_rules.json` (`legado_*`)
