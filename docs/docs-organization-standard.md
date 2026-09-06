# 文档目录组织标准 (Documentation Organization Standard)

本项目遵循全局统一的文档治理规范（参考 `docs-organization.mdc` 与全局标准）：

---

## 一、目录分类原则

所有文档均放置于 `docs/` 目录下，且严格划分为以下五大核心目录：

```text
docs/
├── README.md               # 文档索引与导航总览
├── reference/              # [常驻活文档] 架构设计、接口规格、核心数据模型、长期有效的技术参考 (保持实时更新)
├── guides/                 # [操作指南] 面向开发与维护者的标准操作流程 (SOP)、排障步骤、上手指南 (How-to)
├── design/                 # [技术方案] 架构提议与设计方案，文件命名遵循 rfc-NNN-kebab-case.md
├── postmortem/             # [事后复盘] 故障排查、复盘与历史日志，文件命名遵循 YYYY-MM-DD-kebab-case.md
├── research/               # [前瞻调研] 技术选型、可行性探索与未落地的实验研究
└── archive/                # [归档陈旧] 历史阶段性报告、已被替代的旧方案 (镜像保持 active 目录结构)
    ├── guides/
    ├── postmortem/
    └── reports/
```

---

## 二、硬性门禁约束 (Hard Constraints)

1. **容量限制（米勒定律）**：每个处于活跃状态的目录内文件数量 **≤ 7 个**。一旦超出 7 个，必须将最旧或已完成的文档归档至 `archive/`。
2. **目录层级限制**：从 `docs/` 算起，最大嵌套深度 **≤ 3 级**。
3. **命名规范**：
   - `design/` 下的 RFC 方案：严格使用 `rfc-NNN-kebab-case.md`（如 `rfc-001-crate-consolidation.md`）。
   - `postmortem/` 下的复盘报告：严格使用 `YYYY-MM-DD-kebab-case.md`（如 `2026-07-26-domain-hunt-trial.md`）。
   - 全部目录名与文件名严禁空格与驼峰，统一使用全小写下划线或连字符 (`kebab-case`)。
4. **归档文档标识**：移入 `archive/` 的文档必须在开头附加历史归档提示横幅（Historical Banner），正文内容保持历史冻结，不随意修改。
5. **临时与草稿**：弱相关个人笔记一律放入 `.local-docs/`（在 `.gitignore` 中忽略，严禁入库）。

---

## 三、文档分类决策流

```text
新文档进入
  ├─ 是架构提案或RFC？ ───────> design/rfc-NNN-kebab-case.md
  ├─ 是长期维护的架构或规格？ ──> reference/
  ├─ 是面向步骤的操作手册？ ────> guides/
  ├─ 是未来探索或可行性调研？ ──> research/
  ├─ 是事故分析或历史复盘？ ────> postmortem/YYYY-MM-DD-kebab-case.md
  └─ 已完结/已过时的阶段报告 ───> archive/<category>/
```
