# 📚 LegadoSkill 文档中心 (Documentation Index)

本项目文档严格遵循 [文档目录组织标准](docs-organization-standard.md) 与全局 `docs-organization.mdc` 规范进行归纳与维护。

---

## 一、目录结构导航

| 目录 | 定位说明 | 包含核心内容 |
|---|---|---|
| [`reference/`](reference/) | **常驻架构与技术规格 (SOT)** | 整体平台架构、维修适配器设计、核心知识库摘要、防卡死决策矩阵 |
| [`guides/`](guides/) | **实操手册与开发指南 (How-To)** | 书源创作 SOP、找站指南、MCP 编码使用、规则调试流程 |
| [`design/`](design/) | **技术提案与设计规约 (RFC)** | 核心重构与系统演化设计方案（遵循 `rfc-NNN-kebab-case.md`） |
| [`postmortem/`](postmortem/) | **事后复盘与排障日志 (Retros)** | 生产事故排查、批量跑批复盘记录（遵循 `YYYY-MM-DD-kebab-case.md`） |
| [`research/`](research/) | **技术选型与前瞻研究** | PC 校验引擎可行性探索、站内搜索延迟设计等 |
| [`archive/`](archive/) | **历史阶段性报告与旧版归档** | 2025~2026 历史总结、旧测试套件验收记录（保持历史冻结） |

---

## 二、维护原则

1. **米勒定律门禁**：每个活跃子目录内保持 ≤ 7 个有效文件，溢出时将旧文件归档至 `archive/`。
2. **零坏链要求**：引用文档前必须核实路径；不可臆测不存在的文件。
