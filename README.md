# 📚 legadoSkill

<p align="center">
  <b>现代化 Legado (开源阅读) 书源自动化开发、深度修复引擎与多 Agent 智能体技能标准体系</b><br>
  <b>A Modern Legado Book-Source Engineering Platform: Rust Engine, MCP Real-Device Automation & Multi-Agent Skills</b>
</p>

<p align="center">
  <a href="README.md"><b>简体中文</b></a> | <a href="README_EN.md"><b>English</b></a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75%2B-orange.svg?style=flat-square" alt="Rust Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://modelcontextprotocol.io/"><img src="https://img.shields.io/badge/Protocol-MCP-green.svg?style=flat-square" alt="MCP Protocol"></a>
  <a href="https://github.com/gedoor/legado"><img src="https://img.shields.io/badge/Target%20App-Legado%203.x-red.svg?style=flat-square" alt="Target Legado"></a>
</p>

---

## 📖 项目定位 (Overview)

`legadoSkill` 是专为 Android **[Legado (开源阅读 3.0+)](https://github.com/gedoor/legado)** 打造的现代化书源工程开发套件与自动化运维基础设施。

在 Legado 生态中，书源规则复杂且脆弱（涵盖 CSS/JQuery 选择器、XPath、JSONPath、正则提取、Rhino JS 引擎、加密解密、登录鉴权、反爬对抗等）。当目标网站改版、域名变更或增加防护盾时，传统的手工排查调试极度繁琐。

本项目将书源开发与修复演进为**工业级自动化工程体系**：
- **纯 Rust 全栈重写**：告别零散低效的脚本，提供毫秒级响应的高性能命令行工具 `source-cli`。
- **真机闭环联动**：通过 **MCP (Model Context Protocol)** 协议直连 Android 设备端 Legado 官方客户端，实现**“分析 -> 编写 -> 推送 -> 调试 -> 真机验证”**全自动化闭环。
- **多 Agent 智能体技能**：原生赋能 Claude Code、Codex、Cursor、Hermes 等主流 AI 编码助手，内置 9 大专业规约参考库与单向排障直觉。

---

## ⚡ 与上游 (Upstream) 的核心差异

本项目派生自 `rezmdie/legadoSkill`，但经过彻底的架构迭代与工程化重构，现已完全脱胎换骨：

| 维度 | 上游原版 (`rezmdie/legadoSkill`) | 本项目 (`h11128/legadoSkill`) |
|---|---|---|
| **技术栈底层** | Python (LangChain / LangGraph) + 散装脚本 | **全栈 Rust 1.75+**，6 大高内聚分层 Crates，零 Python 运行时依赖 |
| **执行效率** | 启动慢，高并发批量分析易崩溃 | 毫秒级启动，极低内存开销，支持多线程高并发探针与波次调度 |
| **调试验证方式** | 依赖本地模拟器或虚假规则推测，容易出现“本地成功、手机报错” | **MCP 真机自动化闭环**：直连 Android 真机 `debug_source` 与 `check_source`，以真机返回为唯一真理 |
| **排障认知机制** | 缺乏层级防守，常在搜索未通过时盲改正文或目录 | **严格单向诊断链** (`Search -> Detail -> TOC -> Content`)，自动识别频控告警、验证码、CF 盾并阻断无效改动 |
| **系统架构** | 混乱的脚本目录，包含大量冗余资产与已失效的 IDE 捆绑包 | 严格分层的 **6 大 Crate 模块化工作区**，内建 SQLite 状态持久化与信道防死锁保护 |
| **知识库与技能** | 非结构化的散落 txt/md，存在多处过时描述与死链接 | **9 大结构化规约手册**，覆盖加解密、选择器、Cookie 鉴权、过盾、订阅源规范，对齐最新 Legado 源码 |
| **智能体兼容性** | 仅支持单一 IDE / Prompt 粘贴 | 统一的 **Multi-Agent Skills 标准**，无缝适配 Cursor, Claude Code, Codex, Hermes |

---

## 🏗 系统架构 (System Architecture)

工程根目录位于 `crates/`，严格遵循分层解耦原则：

```text
crates/
├── source-core/        # [核心层] 领域实体、JSON 数据契约、站点指纹分类与视频规则
├── source-storage/     # [存储层] SQLite 事务数据库、EWMA 频控与域名状态缓存
├── source-engine/      # [引擎层] CSS/JS/正则解析、单向诊断链与动态探针测试
├── source-flow/        # [流程层] 补丁引擎、域名迁移、全网猎取、波次编排与关门门禁
├── source-mcp/         # [通信层] Legado 协议适配器、MCP 客户端与批检桥接器
└── source-cli/         # [交互层] 统一命令行控制台 (source-cli 二进制程序)
```

| Crate 模块 | 核心职责 | 子模块包含 |
|---|---|---|
| **`source-core`** | 业务契约与基础类型 | `source-types` (实体), `source-contracts` (Schema), `source-identify` (站点指纹), `source-pattern` (特征聚类), `source-video` (视音频流) |
| **`source-storage`** | 状态持久化与缓存 | `source-db` (嵌入式 SQLite), `source-cache` (EWMA 冷却时间与域名状态) |
| **`source-engine`** | 规则解析与探测 | `source-parse` (选择器/解析), `source-diagnose` (单向排障链), `source-probe` (网络与表单探针) |
| **`source-flow`** | 编排与工作流 | `source-patch` (补丁生成), `source-migrate` (域名替换), `source-hunt` (新站搜寻), `source-queue` (波次调度), `source-closeout` (收尾门禁) |
| **`source-mcp`** | 真机接口与通信 | `source-adapters` (Legado 接口映射), `source-mcp` (MCP 协议), `source-check` (真机批检桥接) |
| **`source-cli`** | 统一操作终端 | 诊断、修复、推源、探针、巡检、波次修复全部子命令 |

---

## 🚀 快速上手 (Quick Start & Onboarding)

### 1. 前置准备 (Prerequisites)

- **开发环境**：Windows / Linux / macOS，安装 **Rust 1.75+** (`rustup update stable`)
- **Android 设备**：
  - 安装 [Legado (开源阅读) 3.0+](https://github.com/gedoor/legado) 官方客户端
  - 手机与电脑处于**同一局域网 (Wi-Fi)**
  - 打开 Legado，进入 **「我的」->「Web服务」**，开启服务；或开启内置的 **MCP 服务**（默认端口 `1236`）

### 2. 编译与安装 CLI

```bash
# 克隆仓库
git clone https://github.com/h11128/legadoSkill.git
cd legadoSkill

# 编译并安装 source-cli
cd crates
cargo build --release -p source_cli
cargo install --path source-cli --force

# 验证安装（新开终端或确保 ~/.cargo/bin 在 PATH 中）
source-cli --help
```

### 3. 配置真机 MCP 连接

编辑项目根目录下的配置文件 [`config/mcp_defaults.json`](config/mcp_defaults.json)：

```json
{
  "device_ip": "192.168.1.100",    // 替换为您手机在局域网的真实 IP
  "device_port": 1236,              // Legado MCP 默认端口
  "http_timeout_s": 30,             // 基础 HTTP 请求超时
  "debug_timeout_s": 60,            // 单源调试超时时间
  "verify_timeout_ms": 90000        // 全链路真机校验超时时间
}
```

测试设备通信与通道状态：
```bash
# 检查手机 MCP 信道是否通畅与空闲
source-cli check channel
```

---

## 💡 典型使用场景 (Workflows & Usage)

### 场景一：深度诊断与修复失效书源 (Repair Workflow)

当某个书源失效（搜索不到、目录乱码、正文空白）时，遵循严格的诊断修复流程：

```bash
# 步骤 1：确认手机端通道空闲
source-cli check channel

# 步骤 2：自动运行单向诊断链 (自动嗅探 Search/Detail/TOC/Content 问题)
source-cli diagnose --url "https://failing-novel-site.com" --key "我的"

# 步骤 3：单步自动化修复并推送到真机即时验证
source-cli repair --mode oneshot --url "https://failing-novel-site.com"

# 步骤 4：记录维修日志与沉淀陷阱（遵循项目闭环纪律）
source-cli ledger append --url "https://failing-novel-site.com" --step check --result "校验成功"
source-cli retro append --url "https://failing-novel-site.com" --status fixed --trap "搜索页表单改为POST且需GBK转码" --skill-fix 0
```

### 场景二：从零创作新书源 (Create Workflow)

发现新的小说网站并快速生成书源：

```bash
# 步骤 1：对目标站点执行探针扫描，自动识别网站类型、编码与搜索表单
source-cli site-probe --url "https://new-novel-site.com"

# 步骤 2：基于探测结果生成书源脚手架模板 (例如笔趣阁通用架构)
source-cli source scaffold --host "new-novel-site.com" --type biquge --name "新站笔趣阁"

# 步骤 3：人工或由 AI Agent 微调规则后，一键直推至手机 Legado
source-cli source push --file temp/new_source.json

# 步骤 4：触发真机全链路校验（搜索、详情、目录、正文）
# 手机端自动运行并回传校验结果，只有显示“校验成功”方可交付
```

### 场景三：多站点批量巡检与波次修复 (Batch Wave)

```bash
# 批量检测并自动调度波次修复
source-cli wave --urls-file failing_urls.txt --thread-count 8
```

---

## 🤖 与 AI Agent 协同开发 (Multi-Agent Integration)

本项目专为 AI 辅助编程打造。您可以在 **Claude Code**, **Codex**, **Cursor**, **Hermes** 中直接调用本项目的技能：

- **技能入口**：`skills/legado-book-source` 与 `skills/legado-book-source-repair`
- **配置方法**：参考详细配置指南 [`MULTI_AGENT_SETUP.md`](MULTI_AGENT_SETUP.md)
- **排障心法与硬纪律**：
  1. **未在手机验证通过绝不宣称修复成功**：严禁凭“本地跑通”或猜测臆断结果。
  2. **严格单向诊断链**：`搜索 -> 详情 -> 目录 -> 正文`，搜索未通前严禁盲改后续规则。
  3. **频控保护与反爬嗅探**：遇到 `alert("搜索间隔")`、403、5秒盾时，依靠冷却机制，禁止乱改选择器。
  4. **动态内容加权**：遇到 JS 动态渲染时，使用 `,{"webView": true}`（二级规则使用 `##$##,{"webView":true}`）。
  5. **负向禁止约束**：引擎不存在 `ruleContent.prevContentUrl` 字段；禁止从 `<select>` 直接提取 `@value`（应写 `select option@value`）。

---

## 📚 9 大标准参考库索引 (Skills References)

位于 [`skills/legado-book-source/references/`](skills/legado-book-source/references/)：

1. 📑 **[书源核心 Schema 与模板](skills/legado-book-source/references/source-schema-template.md)** - 必填字段、类型约束与标准输出模板。
2. 🎯 **[CSS 选择器与伪类规范](skills/legado-book-source/references/css-rules.md)** - Legado 定制伪类（`:matches`, `:has` 等）与选择器提取语法。
3. ⚙️ **[JS 拓展函数与内置对象](skills/legado-book-source/references/js-extensions.md)** - `java.*` 内置桥接函数（`java.ajax()`, `java.base64Decode()`, `java.log()` 等）与执行生命周期。
4. 🔐 **[加解密与编码转换算法](skills/legado-book-source/references/crypto-methods.md)** - AES、DES、RSA、RC4、MD5 及自定义解密脚本。
5. 🛡️ **[反爬绕过与验证码防范](skills/legado-book-source/references/bypass-verification.md)** - Cloudflare 盾、字体混淆、验证码识别及滑块应对方案。
6. 🔑 **[登录状态维护与 Cookie 鉴权](skills/legado-book-source/references/login-and-auth.md)** - 登录表单构建、Cookie 跨步传递与自动续期。
7. 🔤 **[编码识别与乱码修复指南](skills/legado-book-source/references/encoding-guide.md)** - GBK / UTF-8 嗅探、URL 转码与响应解码实战。
8. 📡 **[订阅源完整开发规范](skills/legado-book-source/references/subscription-rules.md)** - 订阅源配置、发现页瀑布流与更新规则。
9. 💡 **[高级排障直觉与避坑速查](skills/legado-book-source/references/advanced-features.md)** - 倒序目录、瀑布流分页、动态目录合并与负向禁令速查。

---

## ❓ 常见问题 (FAQ & Troubleshooting)

### Q1: `source-cli check channel` 显示超时或连不上手机？
- 请检查手机与电脑是否处于同一 Wi-Fi，且路由器未开启 AP 隔离（Guest 隔离）。
- 打开手机浏览器，访问 `http://<手机IP>:1236` 确认 Legado 服务是否正常运行。
- 确认 `config/mcp_defaults.json` 中的 IP 与端口正确。

### Q2: 搜索结果明明页面有内容，但返回 0 条？
- 请检查 HTTP 返回日志中是否存在 `alert("搜索间隔")`、`频繁请求` 或验证码拦截。
- 如果网站有搜索间隔保护，切勿重写选择器，请等待冷却时间后再重试。

### Q3: 目录加载出来了，但章节顺序是反的？
- 在目录列表选择器前加上减号 `-` 即可自动倒序，例如 `-ul.chapters li`。

### Q4: 正文提取出来包含大段广告？
- 优先在规则末尾追加 `@ownText`，例如 `div#content@ownText`，可以自动剔除内部子标签包裹的嵌套广告。

---

## 📄 开源许可证 (License)

本项目遵循 [MIT License](LICENSE) 开源协议。
