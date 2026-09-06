# ⚙️ LegadoSkill 配置文件指南 (Configuration Guide)

本项目 `config/` 目录存放 Rust 核心引擎 (`source-cli`) 与 MCP 真机闭环在运行期所必需的**事实源 (SOT)、门禁策略表、种子库以及数据契约 JSON Schema**。

---

## 📋 配置文件清单与职责说明

| 配置文件 / 目录 | 核心职责 | 消费方 (Rust 代码模块) | 修改与维护指引 |
|---|---|---|---|
| **`mcp_defaults.json`** | **真机 MCP 连接唯一事实源 (SOT)**。配置手机局域网 IP、端口（默认 `1236`）及各阶段超时。 | `source-mcp` (`endpoint.rs`, `discover.rs`)、`source-cli` | 手机 Wi-Fi IP 变动时必须在此更新。超时设定覆盖调试与真机校验。 |
| **`verify_skip_rules.json`** | **门禁策略与黑白名单库**。定义已知死域名、官方硬防屏蔽域及跳过规则，防止空耗真机校验配额。 | `source-gate` (`classify.rs`)、`source-types` | 发现确定无法修复的死站、非小说类域名或特定防御墙时在此追加。 |
| **`domain_hunt_seeds.json`** | **换站猎取种子库**。旧书站域名彻底失效时，提供同类搜书站、镜像站猎取候选种子。 | `source-hunt` (`seeds.rs`)、`source-cli` | 增补优质搜索引擎、泛书站入口用于新域名挖掘。 |
| **`site_candidates_publish.json`** | **精选候选站点预设库**。`source-cli site-probe --preset publish` 快速探针扫描时读取。 | `source-cli` (`cmds/site_probe/load.rs`) | 存放经过人工初筛的高质量书站候选清单。 |
| **`video_source_routes.json`** | **音视频源 (type 3/4) 路由分流表**。将影视、有声书等特殊源导向专用解析器。 | `source-video` (`route.rs`)、`source-cli` | 维护音频、视频聚合站点的域名路由与规则特征。 |
| **`repair_config.json`** | **修复引擎全局运行参数**。单源修复超时、重试上限与默认工作参数。 | `source-types` (`config.rs`) | 调整通用修复流的步长与容错参数。 |
| **`repair_db_defaults.json`** | **本地数据库存储默认值**。嵌入式 SQLite 数据库文件与缓存文件默认路径。 | `source-db` (`cfg.rs`)、`source-mcp` | 默认指向 `temp/` 目录下的 SQLite 数据库。 |
| **`repair_db_schema.sql`** | **SQLite 数据库表结构蓝图**。包括书源快照表、维修事件账本、域名频控统计表等 DDL。 | 开发者参考 / 数据库维护迁移 | 与 `source-db/src/schema.rs` 保持同步。 |
| **`repair_contracts/`** | **JSON 契约 Schema 校验目录**（包含 10 个 `*.schema.json` 文件）。 | `source-contracts` (`registry.rs`, `root.rs`) | 引擎在测试和运行期使用 `jsonschema` 强校验数据结构完整性。 |

---

## 🔍 关键配置文件详细解析

### 1. `mcp_defaults.json` (真机端点 SOT)

这是所有真机自动化操作（推送、调试、批检）的核心依据：

```json
{
  "device_ip": "192.168.1.100",    // 手机当前所处 Wi-Fi 局域网分配的 IPv4 地址
  "device_port": 1236,              // Legado 内置 MCP 服务端口（默认 1236）
  "http_timeout_s": 30,             // 基础网络连接超时（秒）
  "debug_timeout_s": 60,            // 单源调试 debug_source 等待超时（秒）
  "verify_timeout_ms": 90000        // 完整四环节校验 check_source 总超时（毫秒）
}
```

> **注意**：
> - 每次手机重新连接 Wi-Fi IP 变动时，仅需修改此文件，所有 CLI 与 Agent 工具会自动使用新端点。
> - 若真机运行较慢，可适当调大 `verify_timeout_ms`（慢速站建议 `180000`）。

---

### 2. `repair_contracts/` (JSON 契约校验)

目录内包含 10 项强校验 Schema，定义了引擎输入输出的规范：
1. `diagnose_result.schema.json` - 单源诊断链分层输出契约
2. `gate_result.schema.json` - L0~L2 门禁判定输出契约
3. `identify_result.schema.json` - 站点指纹识别结果契约
4. `patch_plan.schema.json` - 修复补丁操作计划契约
5. `optimize_plan.schema.json` - 规则优化计划契约
6. `merge_plan.schema.json` - 多源合并计划契约
7. `verify_result.schema.json` - 真机校验结果返回契约
8. `ledger_row.schema.json` - 维修事件台账记录契约
9. `pattern_cluster.schema.json` - 站群模式聚类契约
10. `report_json.schema.json` - 批处理报告通用外层契约

Rust 中的 `source-contracts` 模块在初始化时会自动扫描本目录，并在运行时对输入输出进行 Schema 验证，确保多模块协作的数据一致性。

---

## ⚠️ 维护与整洁纪律 (Discipline)

1. **单一点修改原则**：修改手机 IP 或端口仅在 `mcp_defaults.json` 中调整，严禁在代码中硬编码任何局域网 IP。
2. **严禁保存 `.bak` 文件**：临时备份必须立即清理，禁止将 `*.bak` 文件提交入库。
3. **保持 Schema 同步**：若修改 Rust 数据契约结构体，必须同步更新 `repair_contracts/` 对应的 Schema 文件。
