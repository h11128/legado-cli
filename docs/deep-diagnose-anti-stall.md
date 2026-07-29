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
| **HookRule** | `legado_hedged_ledger_success` | beforeShell **deny** |
| **HookRule** | `legado_l0_only_live_repair` | beforeShell **deny** (§15) |
| **HookRule** | `legado_serial_long_await` | beforeShell **ask** on long sleep |
| **hooks.json prompt** | `stop`: unsealed `deep_active` reminder | ask at turn end |
| **MdcRule** | discipline §21–23 | Always-loaded agent guidance |
| **Skill** | traps `agent_turn_stall`, `hedged_ledger_success` | Repair playbook |
| **Work context** | deep anti-stall one-liner | Session SOT |

## HookRule wiring (verified 2026-07-29)

Rules live in **`.cursor/audit-hooks/custom_rules.json`** (tracked in git, loaded by
`audit-hooks hook` for this workspace). The global `~/.cursor/audit-logs/custom_rules.json`
is **not** read by the Windows `audit-hooks.exe hook` runner — rules placed only there never
fire. Rule objects need the full schema: `intercept.events` must repeat the event name, or
the rule records but never blocks.

Verify a rule instead of assuming it works — feed a payload straight into the hook runner:

```bash
# payload file avoids the agent's own command text tripping the matcher
audit-hooks hook < .local-scripts/probe.json
# {"hook_event_name":"beforeShellExecution","command":"…","cwd":"…","workspace_roots":["…"]}
```

Expected: hedged ledger → `deny`, `--l0-only` on live repair → `deny`, long `sleep` → `ask`,
plain `校验成功` ledger → `allow`.

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
- `.cursor/audit-hooks/custom_rules.json` (`legado_*` HookRules, tracked)
- `.cursor/hooks.json` (machine-local wiring only; gitignored)

## Harness registration

```bash
audit-hooks harness register --auto        # picks up mdc / skill / sop_doc changes
audit-hooks harness audit --changed        # 0 error expected
audit-hooks harness verify-change --full-audit
audit-hooks harness review mdc:book-source-repair-discipline
```

`verify-change` runs `cargo fmt` / `cargo test` from the repo root, but this repo's Rust
workspace lives in `crates/`, so those two steps report `os error 267`. Run them manually:

```bash
cd crates && cargo fmt --all -- --check && cargo test -p source_closeout
```
