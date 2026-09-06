# 历史资产归档 (Legacy Assets Archive)

> **归档提示 (Historical Archive Banner)**：
> 本目录收纳自早期 Trae / Python 阶段（2025~2026 前期）的遗留素材、非结构化语料与历史对话记录。
> 本目录内容已永久冻结，仅作为技术演进历史存档，**严禁**在运行时引擎与核心流程中对其产生依赖。

---

## 一、高价值资产去向索引

原 `assets/` 目录中的高价值组件已被分别提升至工程化标准目录中：

| 原始资产 | 新规范位置 | 说明 |
|---|---|---|
| `assets/方法-JS扩展类.md` | `skills/legado-book-source/references/js-extensions.md` | 内置 JS 方法速查手册 |
| `assets/方法-加密解密.md` | `skills/legado-book-source/references/crypto-methods.md` | 加密/解密算法速查手册 |
| `assets/书源输出模板_严格模式.md` | `skills/legado-book-source/references/source-schema-template.md` | 严格模式书源字段规格模板 |
| `assets/css选择器规则.txt` | `skills/legado-book-source/references/css-rules.md` | 核心 CSS 与选择器口诀及速查表 |
| `assets/真实书源模板库.txt` | `fixtures/samples/real_source_templates.txt` | 优质静态书源模板样例库 |
| `assets/book_source_database/` | `fixtures/samples/book_source_database/` | 样本书源离线测试用例库 |

---

## 二、冗余淘汰项说明

以下巨型垃圾及调试中间件已被永久清除（减重约 22 MB）：
- `assets/阅读源码.txt`（15 MB 无格式源码堆砌，本地已有 `legado/` 官方 Git submodule/junction）
- `assets/元素选择浏览器参考.txt`（2.8 MB）
- `assets/核心视图工具.txt`（1.8 MB）
- `assets/eruda.js`、`assets/仿M浏览器元素审查.user.js`（移动端第三方调试器注入脚本）
