# Repair close-out gate

## Rust SOT (2026-07-28 cutover)

Core: `crates/source-closeout/`  
CLI: `source-cli closeout` · `source-cli retro`

| 根因 | 机制 |
|------|------|
| Improve 不可验证 | `retro append` gate + `closeout pending` |
| retro 可撒谎（只改 SKILL） | **`skill_fix=1` ⇒ 必须有效 `script_fix`**（`improve.rs`） |
| novel trap 无 SKILL 行 | novel + no skill_fix → retro 拒绝 |
| 双份 SKILL | `skill_fix` → `closeout sync-skill` |
| progress 跳过收尾 | `progress next` 先跑 `closeout pending` |
| fail 重挑 | `retro append --status fail` seals `final:true` ledger |
| 迁域后仍挑旧 URL | `migrate` 写 `skip:migrated_to:` + 刷 `phone_source_index`；`progress` queue ∩ on_phone |

## Improve / script_fix（硬规则）

`skill_fix=1` 时，`--script-fix` 必须是其一：

1. **Harness 路径**：文本含 `source_patch` / `diagnose_tips` / `source_probe` / `crates/` 等  
   例：`source_patch/smells.rs:17mb_empty_index_tocUrl`
2. **显式不改代码**：`no_auto:<至少8个字符的理由>`  
   例：`no_auto: one-off CSS for this host only`

无效（会被拒）：空字符串、`MCP save_source…`、只写「手工补丁」而无 crate / `no_auto:`。

`skill_fix=0`（known trap）时不强制 script_fix 格式。

## 命令

```bash
source-cli closeout status
source-cli closeout pending
source-cli closeout gate --trap SLUG --skill-fix --script-fix 'source_patch/smells.rs'
source-cli closeout sync-skill
source-cli retro append --url URL --status fixed --trap SLUG \
  --script-fix 'source_patch/smells.rs:hint' --skill-fix
```

## fail/skip 封口

`source-cli retro append --status fail|skip`（默认 `--seal`）写 ledger：

```json
{"url":"…","step":"check","result":"fail:…","final":true,"note":"sealed by source-cli retro"}
```

排队判定：`crates/source-cli/src/cmds/progress.rs`（Rust 唯一实现）。

自测：`cargo test -p source_closeout` · `cargo test -p source_cli`
