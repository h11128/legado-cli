# RFC-001: Rust Workspace 聚合治理与 Skill 规范化重构

- **状态**: Implemented
- **创建日期**: 2026-09-06
- **责任主体**: LegadoSkill Team

---

## 一、背景与问题
随着 LegadoSkill 平台从早期的 Python 脚本全面切换为 Rust，系统在快速迭代过程中诞生了 22 个微型 crate，存在明显的过度拆分（8 个 crate 代码不足 400 行）。同时，`skills/` 根目录散落了 V0.1 到 V0.7 历史文档，缺少标准化的 `references/` 索引目录。

---

## 二、架构决策

### 1. 22 个微型 Crate 聚合为 6 大高内聚层级
- **`source-core`**：收拢 `types`, `ports`, `contracts`；
- **`source-storage`**：收拢 `db`, `cache`, `queue`；
- **`source-engine`**：收拢 `parse`, `diagnose`, `patch`, `pattern`, `identify`, `adapters`；
- **`source-flow`**：收拢 `gate`, `probe`, `hunt`, `migrate`, `video`, `spine`, `closeout`；
- **`source-mcp`**：独立维护，负责设备 MCP 协议与多端点自动故障切换；
- **`source-cli`**：统一顶层命令行与子命令路由。

### 2. Skill 结构自包含化
- 归档旧版 `SKILLV0.*.md` 至 `skills/archive/`。
- 新增 `references/css-rules.md` 与 `references/trap-catalog.md`，构建自闭环参考手册。

---

## 三、验收与兼容性
- 100% 保持既有单测与集成测试通过（0 失败）。
- 保持外部 CLI 命令与真机 MCP 行为兼容。
