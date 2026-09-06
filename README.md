# 📚 legadoSkill

> **现代化 Legado (开源阅读) 书源自动化开发、深度修复引擎与多 Agent 智能体技能标准体系**  
> Modern Legado Book-Source Engineering Platform: Rust Engine, MCP Real-Device Automation & Multi-Agent Skills

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![MCP Protocol](https://img.shields.io/badge/Protocol-MCP-green.svg?style=flat-square)](https://modelcontextprotocol.io/)

---

## 📖 项目简介 (Overview)

`legadoSkill` 是面向 Android **Legado (开源阅读)** 应用的高性能书源工程开发与自动化运维套件。本项目从早期的探索性脚本演进为**工业级全栈 Rust 引擎 (`source-cli`)**，通过 **MCP (Model Context Protocol)** 协议与 Android 真实手机或模拟器深度互联，为 AI Agent（Claude Code, Codex, Cursor, Hermes 等）提供全套标准技能、排障直觉与严格规约。

---

## ✨ 核心特性 (Key Features)

- **🦀 纯 Rust 高性能架构**：22 个组件解耦并聚合为 6 大层级 Crate，消除脚本膨胀，毫秒级响应批量修复与规则解析。
- **📱 MCP 真机闭环验证**：直连 Android 设备端 MCP 服务，实现书源一键推送 (`source push`)、真实网络调试 (`debug_source`) 与真机全链路校验 (`check_source`)。
- **🔍 严格单向诊断链**：内建认知级诊断顺序 `Search -> Detail -> TOC -> Content`，杜绝在搜索未通前盲目改写目录或正文规则。
- **🧠 自动化修复与防死锁**：
  - 自动嗅探 HTTP 频控与反爬特征（`alert("搜索间隔")`、Cloudflare 盾、验证码），阻断无意义的选择器误改。
  - 具备信道锁保护机制 (`source-cli check channel`) 与防挂死心跳监控 (`hang guard`)。
- **📚 9 大标准参考库 (References)**：全面覆盖 CSS 选择器规范、订阅源规则、JS 拓展函数、加解密方案、过盾鉴权、编码检测等深水区知识。
- **🤖 多 Agent 技能标准 (Multi-Agent Ready)**：一套技能规约无缝适配 Claude Code、Codex、Cursor、Hermes 等不同运行环境。

---

## 🏗 系统架构 (Architecture)

工程采用分层工作区设计，位于 `crates/` 目录下：

| Crate | 层级职责 | 核心模块 |
|---|---|---|
| **`source-core`** | 核心规约与基础工具 | 数据契约 (`source-types`, `source-contracts`)、特征分类 (`source-identify`, `source-pattern`)、视频路由 (`source-video`) |
| **`source-storage`** | 存储层与状态管理 | 嵌入式 SQLite 会话与历史记录 (`source-db`)、EWMA 频控与域名缓存 (`source-cache`) |
| **`source-engine`** | 规则解析与探测排障 | CSS/JS/正则解析引擎 (`source-parse`)、深度单向诊断链 (`source-diagnose`)、探针测试 (`source-probe`) |
| **`source-flow`** | 工作流与批量编排 | 规则补丁生成 (`source-patch`)、域名迁移 (`source-migrate`)、暗网/书站猎取 (`source-hunt`)、波次队列调度 (`source-queue`)、收尾门禁 (`source-closeout`) |
| **`source-mcp`** | 真机通信与协议适配 | Legado 远程接口适配器 (`source-adapters`)、MCP 客户端与批检桥接 (`source-mcp`)、真实真机校验 (`source-check`) |
| **`source-cli`** | 统一命令行终端 | 门禁、单步诊断、一键修复、巡检、真机推送等全功能 CLI (`source-cli`) |

---

## 🚀 快速上手 (Quick Start)

### 1. 环境准备

- **Rust 工具链**：Rust 1.75 或更高版本 (`rustup update stable`)
- **Android 设备**：安装 Legado (阅读) 客户端，并在局域网内开启「Web服务」或内置 MCP 服务。

### 2. 编译与安装

```bash
# 进入 crates 目录并编译 CLI
cd crates
cargo build --release -p source_cli

# 确认安装或将 target/release 添加至系统 PATH
./target/release/source-cli --help
```

### 3. 配置 MCP 真机端点

在 `config/mcp_defaults.json` 中配置手机的局域网 IP 与端口（默认端口 `1236`）：

```json
{
  "device_ip": "10.0.0.43",
  "device_port": 1236,
  "http_timeout_s": 30,
  "debug_timeout_s": 60
}
```

---

## 💻 常用 CLI 命令 (CLI Usage)

`source-cli` 是整个开发与修复工作流的核心交互入口：

```bash
# 1. 检查手机 MCP 通道是否空闲（严禁在检查中占用通道）
source-cli check channel

# 2. 单源诊断：按诊断链自动嗅探 Search/Detail/TOC/Content 问题
source-cli diagnose --url "https://target-novel-site.com" --key "我的"

# 3. 单源修复与真机验证
source-cli repair --mode oneshot --url "https://target-novel-site.com"

# 4. 新站发现与探针扫描
source-cli site-probe --preset publish

# 5. 书源脚手架生成与真机推送
source-cli source scaffold --host "novel.example.com" --type biquge
source-cli source push --file novel_source.json

# 6. 运行全套单元测试与一致性测试
cargo test --workspace
```

---

## 📚 文档与参考中心 (Documentation & References)

- **开发与修复规范**：
  - [书源维修纪律门禁 (.cursor/rules/book-source-repair-discipline.mdc)](.cursor/rules/book-source-repair-discipline.mdc)
  - [多 Agent 协同配置指南 (MULTI_AGENT_SETUP.md)](MULTI_AGENT_SETUP.md)
  - [项目架构详细设计 (docs/reference/project-architecture.md)](docs/reference/project-architecture.md)
  - [文档中心索引 (docs/README.md)](docs/README.md)
- **9 大标准参考库 (Skills References)**：
  - [CSS 选择器与伪类规范](skills/legado-book-source/references/css-rules.md)
  - [书源核心 Schema 与模板](skills/legado-book-source/references/source-schema-template.md)
  - [JS 拓展函数与内置对象](skills/legado-book-source/references/js-extensions.md)
  - [加解密与编码转换算法](skills/legado-book-source/references/crypto-methods.md)
  - [反爬绕过与验证码防范](skills/legado-book-source/references/bypass-verification.md)
  - [登录状态维护与 Cookie 鉴权](skills/legado-book-source/references/login-and-auth.md)
  - [编码识别与乱码修复指南](skills/legado-book-source/references/encoding-guide.md)
  - [订阅源完整开发规范](skills/legado-book-source/references/subscription-rules.md)
  - [高级排障直觉与避坑速查](skills/legado-book-source/references/advanced-features.md)

---

## 📄 开源许可证 (License)

本项目遵循 [MIT License](LICENSE) 协议。
