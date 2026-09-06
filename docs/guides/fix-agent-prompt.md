# Fix agent prompt (paste into repair threads)

- **Default:** `source-cli` only (build: `cd crates && cargo build -p source_cli`)
- batch: `source-cli repair --mode batch --urls-file … --limit N`
- wave/harvest/serial: `source-cli wave|harvest|serial --urls-file …`

```text
1. cd E:/Projects/legadoSkill && source-cli check channel   # must idle
2. source-cli progress next                                 # closeout pending auto
3. source-cli diagnose --url URL --key 我的
4. source-cli repair --mode oneshot --url URL
5. source-cli ledger append --url URL --step check --result '校验成功' --note '…'
6. source-cli retro append --url URL --status fixed --trap '…' --skill-fix 0
7. If novel trap → patch skills/legado-book-source-repair/SKILL.md → skill-fix 1
8. git commit skill/docs/scripts/rust as needed → progress next
```

MCP discover: `source-cli discover --write`

Parity: `source-cli parity` or `cargo test --workspace`
