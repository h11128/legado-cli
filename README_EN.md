# 📚 legado-cli

<p align="center">
  <b>A Modern Legado Book-Source Engineering CLI & Automated Repair Platform: Rust Engine, MCP Automation & Multi-Agent Skills</b><br>
  <b>现代化 Legado (开源阅读) 书源工程自动化 CLI、深度修复引擎与多 Agent 智能体技能标准体系</b>
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

`legado-cli` is a modern book-source development CLI and automated maintenance infrastructure designed for the Android **[Legado (阅读 3.0+)](https://github.com/gedoor/legado)** reading application.

Legado book source rules are notoriously delicate (spanning CSS/JQuery selectors, XPath, JSONPath, regular expressions, Rhino JS engines, encryption/decryption, login authentication, and anti-scraping challenges). When target websites update their DOM, rotate domains, or introduce bot protection, manual debugging becomes exhausting.

`legado-cli` transforms book source engineering into an **industrial-grade automated system**:
- **Full Rust Rewrite**: Replaces ad-hoc Python scripts with a high-performance, single-binary CLI (`source-cli`).
- **Real-Device MCP Automation**: Connects directly to the Android Legado app via the **Model Context Protocol (MCP)**, forming a complete closed loop: **Analyze -> Draft -> Push -> Debug -> Real-Device Verify**.
- **Multi-Agent Skills**: Equips AI coding assistants (Claude Code, Codex, Cursor, Hermes) with 9 standard reference manuals and strict diagnostic heuristics.
- **Effortless for Humans**: Human developers do not need to memorize or type complex CLI commands! Simply talk to your AI Agent in natural language while your phone handles live verification in the background.

---

## ⚡ Evolution from Upstream

This repository originally forked from `rezmdie/legadoSkill` but has completely evolved into an independent, pure-Rust CLI platform:

| Dimension | Upstream (`rezmdie/legadoSkill`) | This Project (`h11128/legado-cli`) |
|---|---|---|
| **Underlying Stack** | Python (LangChain / LangGraph) + loose scripts | **Pure Rust 1.75+**, 6 consolidated layer crates, zero Python runtime dependency |
| **Execution Performance** | Slow cold start, prone to out-of-memory errors during batch processing | Sub-millisecond startup, minimal footprint, concurrent multi-worker scanning & wave scheduling |
| **Verification Method** | Local DOM approximation / guessing ("worked locally, failed on phone") | **MCP Real-Device Closed Loop**: Direct connection to Android Legado `debug_source` & `check_source`; the device is the single source of truth |
| **Diagnostic Discipline** | Lacks layered isolation; frequently rewrote TOC or Content before search worked | **Strict Unidirectional Diagnostic Chain** (`Search -> Detail -> TOC -> Content`); automatically detects rate-limiting alerts, captchas, and Cloudflare challenges |
| **Codebase Quality** | Cluttered legacy assets, dead links, and broken Trae IDE download badges | Modular **6-layer crate workspace**, embedded SQLite persistence, and deadlock prevention channel locks |
| **Knowledge Base** | Unstructured, duplicated text files with obsolete guidelines | **9 comprehensive reference manuals** covering crypto, custom CSS pseudo-classes, cookie auth, bypasses, and subscription rules |
| **Agent Support** | Single-prompt copy-paste | Standardized **Multi-Agent Skills** compatible with Cursor, Claude Code, Codex, and Hermes |

---

## 🏗 System Architecture Diagram

```mermaid
flowchart TD
    %% High-contrast accessible theme (clean in both dark and light modes)
    classDef human fill:#1e3a8a,stroke:#60a5fa,stroke-width:2px,color:#ffffff;
    classDef agent fill:#581c87,stroke:#c084fc,stroke-width:2px,color:#ffffff;
    classDef direct fill:#0f766e,stroke:#2dd4bf,stroke-width:2px,color:#ffffff;
    classDef flow fill:#1e293b,stroke:#38bdf8,stroke-width:2px,color:#ffffff;
    classDef device fill:#064e3b,stroke:#34d399,stroke-width:2px,color:#ffffff;
    classDef decision fill:#78350f,stroke:#fbbf24,stroke-width:2px,color:#ffffff;

    Human["👤 Human Developer<br/>(Express intent in plain text)"]:::human
    Agent["🤖 AI Agent (Cursor / Claude / Codex)<br/>(Loads Skills & recognizes intent)"]:::agent
    Human -->|"① Chat interaction"| Agent

    Dispatch{"② Dispatcher Decision<br/>(Is a Flow needed?)"}:::decision
    Agent --> Dispatch

    %% Direct / Flow-free paths
    Dispatch -->|"Query syntax / specs"| Ref["📚 9 Standard Reference Manuals<br/>(Directly reads Markdown docs)"]:::direct
    Dispatch -->|"Check status / metadata"| DirectTool["⚡ source-cli Direct Tools<br/>(Channel status / offline validation)"]:::direct

    %% 4 Dedicated Flow paths
    Dispatch -->|"Source broken"| F1["🔧 Flow 1: Deep Repair<br/>(Diagnose chain ➔ Patch ➔ Live verify)"]:::flow
    Dispatch -->|"Create new source"| F2["✍️ Flow 2: Source Creation<br/>(Probe ➔ Scaffold ➔ Push & verify)"]:::flow
    Dispatch -->|"Dead host / redirected"| F3["🌐 Flow 3: Domain Hunt & Migration<br/>(Hunt mirror ➔ Recursive path replace)"]:::flow
    Dispatch -->|"Batch shelf triage"| F4["🌊 Flow 4: Batch Wave Triage<br/>(Exclusive lock ➔ PC split ➔ Single batch)"]:::flow

    %% Live phone verification
    DirectTool -.->|"Single push"| Phone
    F1 -->|"Live debug & verify"| Phone["📱 Android Real Device Legado<br/>(:1236 MCP / Web service)"]:::device
    F2 -->|"Full-chain live check"| Phone
    F3 -->|"Verify on new domain"| Phone
    F4 -->|"Single-batch verify"| Phone

    Phone -->|"③ Returns live network verification result"| Agent
    Agent -->|"④ Reports final outcome to Human (Verified = Done)"| Human
```

### 6 Layer Crates Breakdown

| Crate | Layer Role | Included Components |
|---|---|---|
| **`source-core`** | Domain contracts & types | `source-types` (core entities), `source-ports` (trait interfaces), `source-contracts` (schema contracts) |
| **`source-storage`** | Persistence & queue cache | `source-db` (embedded SQLite), `source-cache` (rate-limit / EWMA cache), `source-queue` (wave scheduling & retry queue) |
| **`source-engine`** | Rule evaluation & diagnosis | `source-parse` (selector/rule parser), `source-diagnose` (diagnostic chain), `source-patch` (patch generator), `source-pattern` (clustering), `source-identify` (fingerprints), `source-adapters` (source adapters) |
| **`source-flow`** | Workflow orchestration | `source-gate` (health gates), `source-probe` (form probes), `source-hunt` (domain hunter), `source-migrate` (domain rewrite), `source-video` (video routes), `source-spine` (orchestration spine), `source-closeout` (gatekeeper) |
| **`source-mcp`** | Device communications | `source-mcp` (real-device SSE client & connection pool), `source-check` (device check bridge & URL sharding) |
| **`source-cli`** | User command interface | Unified commands for diagnose, repair, push, site-probe, and batch waves |

---

## 🌊 Flow Architecture & Dispatch Strategy

When handling book-source tasks, the system coordinates multi-stage actions through **Flows (transactional pipelines)** to ensure device isolation, rate-limit protection, and state-machine consistency.

### 1. When to Invoke a Flow vs. When NOT to Invoke a Flow

| Task / Scenario | Invoke a Flow? | Recommended Dispatch Path | Rationale & Architectural Design |
|---|---|---|---|
| **Fixing a broken book source** | **Must Invoke Flow** | **Flow 1: Deep Repair Flow** (`diagnose` -> `repair`) | Enforces `Search -> Detail -> TOC -> Content` diagnostic chain; must pass real-device verify. |
| **Creating a source for a new site** | **Must Invoke Flow** | **Flow 2: Source Creation Flow** (`site-probe` -> `scaffold` -> `push`) | Fully executes raw HTML probing, encoding detection, scaffold drafting, and live phone verification. |
| **Target domain is dead / redirected** | **Must Invoke Flow** | **Flow 3: Domain Hunting Flow** (`hunt` -> `probe` -> `migrate`) | Never blindly rewrite selectors; find active mirrors via seed DB and rewrite all absolute URLs. |
| **Auditing / checking entire source list** | **Must Invoke Flow** | **Flow 4: Batch Wave Flow** (`wave` / `search-wave`) | Must acquire exclusive channel lock; multi-worker PC precheck followed by single-batch device verify. |
| **Looking up CSS syntax, JS methods, or crypto** | ❌ **No Flow Needed** | **Read standard reference library** (`skills/references/`) | Pure read-only knowledge retrieval; no runtime or device access required. |
| **Checking if Android MCP is alive & idle** | ❌ **No Flow Needed** | **Run single status check** (`source-cli check channel`) | Pure read-only ping; no pipeline dependency. |
| **Tweaking book source name, group, or intervals** | ❌ **No Flow Needed** | **Edit JSON directly** and push to device | Pure metadata tweak without impacting parsing logic; skips diagnostic pipeline. |
| **Adjusting Explore (exploreUrl) rules** | ❌ **No Flow by default** | Intervene only if explicitly requested by user | Platform discipline: default `checkDiscovery=false` to avoid wasting device quotas. |

### 2. 4 Core Pipeline Cards

- 🔧 **Flow 1: Deep Repair Flow**
  > `[Channel Lock Gate]` ➔ `[L0~L2 Health Gate]` ➔ `[Unidirectional Diagnostics]` ➔ `[Patch Planning]` ➔ `[Push & Device Verify]` ➔ `[Ledger Closeout]`
  - **Trigger**: Search fails, detail error, TOC mangled, or content empty.
  - **Golden Rule**: Never modify TOC/Content before Search succeeds; never claim fixed without real-device green verify.

- ✍️ **Flow 2: Source Creation Flow**
  > `[Raw HTML Probe]` ➔ `[Site Family ID]` ➔ `[Scaffold Generation]` ➔ `[Selector Enhancement]` ➔ `[Push & All-Green Verify]` ➔ `[Commit to Repo]`
  - **Trigger**: New novel site discovered, building book source from scratch.
  - **Golden Rule**: Real-device all-green pass across all 4 stages is the sole acceptance criteria.

- 🌐 **Flow 3: Domain Hunting & Migration Flow**
  > `[404/Parked Alert]` ➔ `[Seed Search for Mirrors]` ➔ `[Candidate Connectivity Probe]` ➔ `[Global URL Rewrite]` ➔ `[Re-verify on Device]`
  - **Trigger**: Site 404, parked, ad-hijacked, or redirected.
  - **Golden Rule**: Never rewrite selector rules on dead sites; hunt active mirrors and rewrite absolute URLs globally.

- 🌊 **Flow 4: Batch Wave Triage Flow**
  > `[Exclusive Channel Lock]` ➔ `[Dead Gate Filter]` ➔ `[PC Multi-Worker Triage]` ➔ `[Single Batch to Device]` ➔ `[Aggregated Report]`
  - **Trigger**: Routine health check and batched triage across dozens of sources.
  - **Golden Rule**: Never run concurrent check batches against the same phone; single-batch execution is mandatory.

---

## 🚀 Quick Start & Onboarding

### 👤 For Humans: Zero Memorization, Natural Language Only

**You do not need to memorize or type complex CLI commands!** The core philosophy of this project is to let AI Agents handle the heavy engineering lifting. You only need two simple steps:

#### Step 1: Connect Phone and Configure IP
1. Ensure your phone and computer are connected to the **same Wi-Fi network**.
2. Open **Legado (开源阅读)** on your phone, go to **"My" -> "Web Service"** and enable it; or enable the built-in **MCP Service** (default port: `1236`).
3. Open [`config/mcp_defaults.json`](config/mcp_defaults.json) in this repository and update `device_ip` to your phone's LAN IP (e.g. `192.168.1.100`).

#### Step 2: Talk to Your AI Assistant
In Cursor, Claude Code, Codex, or Hermes, chat directly using natural language:
- 🗣️ **"Fix this book source, search is broken: `https://www.example-novel.com`"**
- 🗣️ **"I found a new novel site `https://novel.sample.com`, please create a working book source and push it to my phone."**
- 🗣️ **"Check all book sources in my library and auto-repair any broken ones."**

Your AI Agent will automatically invoke skills, run `source-cli` in the background, push candidate rules to the phone, verify them in live execution, and report back once verified!

---

### 🤖 For AI Agents & CLI Developers: `source-cli` Command Matrix

`source-cli` is the unified engineering backbone of this project, handling the entire lifecycle of Legado book sources.

#### 1. Complete CLI Command Matrix

| Category | Subcommand | Description & Primary Use Case | Example Command |
|---|---|---|---|
| **🔍 Diagnostics & Repair** | `diagnose` | Executes the unidirectional chain (`Search ➔ Detail ➔ TOC ➔ Content`), auto-detecting rate limits & WAFs | `source-cli diagnose --url "https://site.com" --key "fantasy"` |
| | `repair` | Generates a targeted `PatchPlan`, pushes to device, and performs step verification | `source-cli repair --mode oneshot --url "https://site.com"` |
| | `dig` | Official end-to-end entrypoint: channel probe ➔ gates ➔ diagnose ➔ oneshot repair | `source-cli dig --url "https://site.com"` |
| | `gate` | Pre-checks L0~L2 health gates (syntax / DNS / 404 / parked domains / Cloudflare challenge) | `source-cli gate --url "https://site.com"` |
| **🌐 Probing & Creation** | `site-probe` | Raw HTML fetch, charset encoding detection, and JS-written form extraction | `source-cli site-probe --url "https://new-site.com"` |
| | `source scaffold` | Generates a book source draft based on identified site patterns | `source-cli source scaffold --url "http://www.site.com" --name "NewSite"` |
| | `source push` | Writes book source rules directly into Legado memory and acquires `deep_active` lock | `source-cli source push --file source.json` |
| **📱 Device Comms & Channel** | `check channel` | Checks Android Legado MCP connection, guards against deadlocks, supports forced clearing | `source-cli check channel --force-clear` |
| | `check clear-cookies` | Clears accumulated stale session cookies and bot-detection challenges from the device | `source-cli check clear-cookies --url "https://site.com"` |
| | `mcp` | Manages remembered real-device MCP endpoints (list, probe, switch, add, remove) | `source-cli mcp list` / `source-cli mcp probe` |
| **🦅 Domain Hunting & Migration**| `hunt` | High-concurrency discovery of working mirror domains from search engines and seed catalogs | `source-cli hunt --url "https://site.com"` |
| | `migrate` | Recursively rewrites all absolute URLs, covers, and hostkeys within the book source | `source-cli migrate --from-url "http://old.com" --to-url "http://new.com"` |
| **🌊 Batch Triage & Waves** | `wave` | Concurrent multi-worker pre-triage with single-batch real-device verification dispatch | `source-cli wave --urls-file list.txt --thread-count 8` |
| | `search-wave` | Rapid batch check across entire book collections to spot search disruptions & dead sources | `source-cli search-wave --urls-file list.txt` |
| | `serial` | Watchdog-guarded single-channel serial queue scheduler with automatic timeouts | `source-cli serial --urls-file list.txt --url-timeout-s 120` |
| **📊 Ledger & Closeout Gates** | `ledger append` | Records every diagnostic/repair step and live verification result for auditing | `source-cli ledger append --url "..." --step check --result "校验成功"` |
| | `retro append` | Records novel traps and lessons learned into the durable retrospective store | `source-cli retro append --url "..." --status fixed --trap "..."` |
| | `closeout` | Verification gatekeeper: halts workflow if the source has not succeeded on live device | `source-cli closeout pending` |
| **⚙️ Cache & Rule Parsing** | `cache` / `ewma` | Domain rate-limit EWMA cooldown cache manager to prevent spamming blocked endpoints | `source-cli cache cooldown --url "https://site.com"` / `source-cli ewma` |
| | `parse` | Offline evaluation of CSS selectors, JS scripts, or regex extraction expressions | `source-cli parse rule --rule "@css:div#content@text"` |

#### 2. Local Compilation & Global Installation

```bash
# Navigate to the Rust workspace
cd crates

# Compile optimized release binary
cargo build --release --bin source-cli

# Install globally to your Cargo PATH
cargo install --path source-cli --force

# View all CLI commands and options
source-cli --help
```

#### 3. Typical Real-World Workflows

##### Scenario 1: Deep Diagnostic & Live Device Repair
```bash
# 1. Verify channel is idle
source-cli check channel

# 2. Run diagnosis along the strict unidirectional chain
source-cli diagnose --url "https://target-site.com" --key "fantasy"

# 3. Generate patch, push to real device, and verify
source-cli repair --mode oneshot --url "https://target-site.com"

# 4. Record ledger entry and lessons learned upon success
source-cli ledger append --url "https://target-site.com" --step check --result "校验成功"
source-cli retro append --url "https://target-site.com" --status fixed --trap "Search converted to POST with GBK"
```

##### Scenario 2: Create a Book Source from Scratch
```bash
# 1. Probe target site structure, encoding, and search forms
source-cli site-probe --url "https://new-site.com"

# 2. Generate scaffold based on recognized patterns
source-cli source scaffold --url "https://new-site.com" --name "Biquge Mirror"

# 3. Push to real device Legado (claims deep_active lock)
source-cli source push --file temp/new_source.json
```

##### Scenario 3: Batch Wave Triage
```bash
# 8 concurrent workers pre-triage, single-batch device verification
source-cli wave --urls-file failing_urls.txt --thread-count 8
```

---

## 🤖 AI Agent Integration

This workspace is natively tailored for AI pair-programming:

- **Skill Entries**: `skills/legado-book-source` (creation) and `skills/legado-book-source-repair` (repair).
- **Multi-Agent Setup**: See [`MULTI_AGENT_SETUP.md`](MULTI_AGENT_SETUP.md) for configuration details.
- **Core Engineering Disciplines**:
  1. **Never claim fixed without device verification**: Local parsing success is not proof that the Legado app can read the source.
  2. **Strict Unidirectional Diagnostic Chain**: `Search -> Detail -> TOC -> Content`. Never alter TOC or Content rules before search produces verified book links.
  3. **Respect Rate Limits**: When encountering `alert("搜索间隔")`, 403 status, or Cloudflare challenges, honor EWMA cooldown periods instead of rewriting selectors.
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

## ⚙️ Configuration Guide

Detailed configuration docs can be found in [`config/README.md`](config/README.md). Device endpoints are maintained in [`config/mcp_defaults.json`](config/mcp_defaults.json), gate skipping rules in [`config/verify_skip_rules.json`](config/verify_skip_rules.json), and JSON contract schemas in [`config/repair_contracts/`](config/repair_contracts/).

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

## 🙏 Acknowledgments & Heritage

- This repository was originally forked from [rezmdie/legadoSkill](https://github.com/rezmdie/legadoSkill). We are deeply grateful to the original author for pioneering the concept of using AI to assist Legado book-source generation.
- With evolving real-world requirements and systematic engineering, this project has since been completely rewritten into a modular, pure-Rust layered architecture with Android real-device MCP verification loops and disciplined diagnostic pipelines. It is now maintained and developed as an independent production codebase.
- We also pay tribute to the incredible [Legado (开源阅读)](https://github.com/gedoor/legado) mobile reading engine and its open-source community for providing such an extensible and empowering platform.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

