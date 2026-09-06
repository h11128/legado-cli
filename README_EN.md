# 📚 legadoSkill

<p align="center">
  <b>A Modern Legado Book-Source Engineering Platform: Rust Engine, MCP Real-Device Automation & Multi-Agent Skills</b><br>
  <b>现代化 Legado (开源阅读) 书源自动化开发、深度修复引擎与多 Agent 智能体技能标准体系</b>
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

## 📖 Overview

`legadoSkill` is a modern book-source development toolkit and automated maintenance infrastructure designed for the Android **[Legado (阅读 3.0+)](https://github.com/gedoor/legado)** reading application.

Legado book source rules are complex and delicate, spanning CSS/JQuery selectors, XPath, JSONPath, regular expressions, Rhino JS engines, encryption/decryption, login authentication, and anti-scraping challenges. When target websites update their DOM, rotate domains, or introduce bot protection, manual debugging becomes exhausting.

`legadoSkill` transforms book source development and repair into an **industrial-grade automated engineering pipeline**:
- **Full Rust Rewrite**: Replaces ad-hoc scripts with a high-performance, single-binary CLI (`source-cli`).
- **Real-Device MCP Automation**: Connects directly to the Android Legado app via the **Model Context Protocol (MCP)**, forming a complete closed loop: **Analyze -> Draft -> Push -> Debug -> Real-Device Verify**.
- **Multi-Agent Skills**: Equips AI coding assistants (Claude Code, Codex, Cursor, Hermes) with 9 standard reference manuals and strict diagnostic heuristics.

---

## ⚡ Comparison with Upstream

This repository originated from `rezmdie/legadoSkill` but has undergone a complete architectural rewrite:

| Dimension | Upstream (`rezmdie/legadoSkill`) | This Project (`h11128/legadoSkill`) |
|---|---|---|
| **Underlying Stack** | Python (LangChain / LangGraph) + loose scripts | **Pure Rust 1.75+**, 6 consolidated layer crates, zero Python runtime dependency |
| **Execution Performance** | Slow cold start, prone to out-of-memory errors during batch processing | Sub-millisecond startup, minimal footprint, concurrent multi-worker scanning & wave scheduling |
| **Verification Method** | Local DOM approximation / guessing ("worked locally, failed on phone") | **MCP Real-Device Closed Loop**: Direct connection to Android Legado `debug_source` & `check_source`; the device is the single source of truth |
| **Diagnostic Discipline** | Lacks layered isolation; frequently rewrote TOC or Content before search worked | **Strict Unidirectional Diagnostic Chain** (`Search -> Detail -> TOC -> Content`); automatically detects rate-limiting alerts, captchas, and Cloudflare challenges |
| **Codebase Quality** | Cluttered legacy assets, dead links, and broken Trae IDE download badges | Modular **6-layer crate workspace**, embedded SQLite persistence, and deadlock prevention channel locks |
| **Knowledge Base** | Unstructured, duplicated text files with obsolete guidelines | **9 comprehensive reference manuals** covering crypto, custom CSS pseudo-classes, cookie auth, bypasses, and subscription rules |
| **Agent Support** | Single-prompt copy-paste | Standardized **Multi-Agent Skills** compatible with Cursor, Claude Code, Codex, and Hermes |

---

## 🏗 System Architecture

The core workspace resides in `crates/`, partitioned into 6 high-cohesion layers:

```text
crates/
├── source-core/        # [Core] Domain types, JSON contracts, site fingerprinting & video rules
├── source-storage/     # [Storage] SQLite session DB, EWMA cooldown & domain cache
├── source-engine/      # [Engine] Selectors parser, unidirectional diagnostic chain & live probes
├── source-flow/        # [Flow] Patch generator, domain migration, site hunter & wave scheduler
├── source-mcp/         # [Protocol] Legado adapters, MCP client & real-device check bridge
└── source-cli/         # [CLI] Unified command-line interface (source-cli binary)
```

| Crate | Layer Role | Included Components |
|---|---|---|
| **`source-core`** | Domain contracts & types | `source-types` (entities), `source-contracts` (schemas), `source-identify` (fingerprints), `source-pattern` (clustering), `source-video` (media rules) |
| **`source-storage`** | Persistence & caching | `source-db` (embedded SQLite), `source-cache` (EWMA cooldown & domain statuses) |
| **`source-engine`** | Rule evaluation & diagnosis | `source-parse` (CSS/JS/regex parser), `source-diagnose` (diagnostic chain), `source-probe` (web & form probes) |
| **`source-flow`** | Workflow orchestration | `source-patch` (patch generator), `source-migrate` (domain rewrite), `source-hunt` (domain hunter), `source-queue` (wave scheduler), `source-closeout` (gatekeeper) |
| **`source-mcp`** | Device communications | `source-adapters` (Legado API bridge), `source-mcp` (MCP protocol client), `source-check` (device check bridge) |
| **`source-cli`** | User command interface | Unified commands for diagnose, repair, push, site-probe, and batch waves |

---

## 🚀 Quick Start & Onboarding

### 1. Prerequisites

- **Host Environment**: Windows, Linux, or macOS with **Rust 1.75+** (`rustup update stable`).
- **Android Device**:
  - Install [Legado (开源阅读) 3.0+](https://github.com/gedoor/legado).
  - Ensure the device is on the same local network (LAN/Wi-Fi) as your PC.
  - Enable **Web Service** or the built-in **MCP Service** in Legado (default port: `1236`).

### 2. Build and Install CLI

```bash
# Clone this repository
git clone https://github.com/h11128/legadoSkill.git
cd legadoSkill

# Build and install source-cli
cd crates
cargo build --release -p source_cli
cargo install --path source-cli --force

# Verify installation (ensure ~/.cargo/bin is in your PATH)
source-cli --help
```

### 3. Configure Real-Device MCP Connection

Edit [`config/mcp_defaults.json`](config/mcp_defaults.json) in the project root:

```json
{
  "device_ip": "192.168.1.100",    // Replace with your Android device LAN IP
  "device_port": 1236,              // Legado MCP default port
  "http_timeout_s": 30,             // HTTP request timeout in seconds
  "debug_timeout_s": 60,            // Single source debug timeout in seconds
  "verify_timeout_ms": 90000        // Full verification timeout in milliseconds
}
```

Check communication with the device:
```bash
# Verify the MCP channel is idle and operational
source-cli check channel
```

---

## 💡 Typical Workflows & Usage

### Scenario 1: Diagnose and Repair a Failing Book Source

When a book source breaks (empty search, broken TOC, or empty content), follow the unidirectional diagnosis workflow:

```bash
# Step 1: Ensure phone MCP channel is idle
source-cli check channel

# Step 2: Run diagnosis (automatically detects Search/Detail/TOC/Content bottlenecks)
source-cli diagnose --url "https://failing-novel-site.com" --key "我的"

# Step 3: Apply automatic patches and verify on real device
source-cli repair --mode oneshot --url "https://failing-novel-site.com"

# Step 4: Record repair ledger and lessons learned
source-cli ledger append --url "https://failing-novel-site.com" --step check --result "校验成功"
source-cli retro append --url "https://failing-novel-site.com" --status fixed --trap "Search form converted to POST with GBK encoding" --skill-fix 0
```

### Scenario 2: Create a New Book Source from Scratch

When discovering a novel site and building a rule set:

```bash
# Step 1: Probe the target site to inspect structure, encoding, and search forms
source-cli site-probe --url "https://new-novel-site.com"

# Step 2: Generate a scaffold JSON based on recognized patterns (e.g. Biquge architecture)
source-cli source scaffold --host "new-novel-site.com" --type biquge --name "Biquge Mirror"

# Step 3: Refine selectors and push directly to the Android device
source-cli source push --file temp/new_source.json

# Step 4: Trigger real-device full verification (Search -> Detail -> TOC -> Content)
# Claim success only when the phone returns "校验成功"
```

### Scenario 3: Batch Wave Triage

```bash
# Triage failing URLs in parallel and execute wave repair
source-cli wave --urls-file failing_urls.txt --thread-count 8
```

---

## 🤖 AI Agent Integration

This workspace is designed from the ground up for AI-assisted development (Claude Code, Codex, Cursor, Hermes):

- **Skills**: `skills/legado-book-source` and `skills/legado-book-source-repair`
- **Setup Guide**: See [`MULTI_AGENT_SETUP.md`](MULTI_AGENT_SETUP.md) for agent configuration details.
- **Core Engineering Disciplines**:
  1. **Never claim fixed without device verification**: Local parsing success is not proof that the Legado app can read the source.
  2. **Strict Unidirectional Diagnostic Chain**: `Search -> Detail -> TOC -> Content`. Do not edit TOC or Content rules before search produces verified book links.
  3. **Respect Rate Limits**: When encountering `alert("搜索间隔")`, 403 status, or Cloudflare challenges, honor cooldown periods instead of rewriting selectors.
  4. **Dynamic Webview Rendering**: Append `,{"webView": true}` for JS-rendered pages (for sub-rules like `chapterUrl`, use `##$##,{"webView":true}`).
  5. **Negative Constraints**: The Legado engine does NOT have `ruleContent.prevContentUrl`; never extract `@value` directly from `<select>` (use `select option@value`).

---

## 📚 9 Standard References

Located in [`skills/legado-book-source/references/`](skills/legado-book-source/references/):

1. 📑 **[Source Schema & Template](skills/legado-book-source/references/source-schema-template.md)** - Required fields, type specifications, and standard JSON template.
2. 🎯 **[CSS Selectors & Pseudo-Classes](skills/legado-book-source/references/css-rules.md)** - Legado custom pseudo-classes (`:matches`, `:has`) and extraction modifiers.
3. ⚙️ **[JS Extensions & Built-in Objects](skills/legado-book-source/references/js-extensions.md)** - `java.*` helper functions (`java.ajax()`, `java.base64Decode()`, `java.log()`) and lifecycle rules.
4. 🔐 **[Cryptographic Algorithms & Ciphers](skills/legado-book-source/references/crypto-methods.md)** - AES, DES, RSA, RC4, MD5, and custom decryption patterns.
5. 🛡️ **[Bypassing Anti-Scraping & Captchas](skills/legado-book-source/references/bypass-verification.md)** - Cloudflare challenges, custom font de-obfuscation, and captcha handling.
6. 🔑 **[Login & Cookie Authentication](skills/legado-book-source/references/login-and-auth.md)** - Building login requests, cross-step cookie preservation, and session renewal.
7. 🔤 **[Encoding Detection & Mojibake Fixes](skills/legado-book-source/references/encoding-guide.md)** - GBK / UTF-8 detection, query string encoding, and response decoding.
8. 📡 **[Subscription Source Specifications](skills/legado-book-source/references/subscription-rules.md)** - Subscription feeds, discovery waterfall layouts, and update rules.
9. 💡 **[Advanced Troubleshooting & Anti-Patterns](skills/legado-book-source/references/advanced-features.md)** - Reversed catalogs, waterfall pagination, dynamic TOC merging, and negative bans.

---

## ❓ FAQ & Troubleshooting

### Q1: `source-cli check channel` times out or fails to connect to the phone?
- Verify that your PC and phone are connected to the same Wi-Fi and that AP isolation (Guest Mode) is disabled on your router.
- Open your browser and visit `http://<phone-ip>:1236` to verify Legado's web service is responsive.
- Verify `device_ip` and `device_port` in `config/mcp_defaults.json`.

### Q2: Search produces 0 results even though the page has content?
- Check the HTTP debug logs for `alert("搜索间隔")`, frequency limits, or captcha challenges.
- If the target host has a rate-limit interval, do NOT alter the selectors; wait for the cooldown timer.

### Q3: The catalog is reversed?
- Prepend `-` to the TOC list selector (e.g. `-ul.chapters li`).

### Q4: Chapter content contains ads?
- Prefer appending `@ownText` to your selector (e.g. `div#content@ownText`) to automatically discard ads nested in child tags.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
