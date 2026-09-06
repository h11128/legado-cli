# 🌊 业务流程全景架构与调度决策指南 (Flow Architecture & Dispatch Guide)

> **定位说明**：本文档定义了 `legadoSkill` 平台中四大核心工作流（Flow）的状态机架构、流转拓扑，以及**何时调用 Flow 与何时直调工具（无需 Flow）**的判定矩阵，为开发者与 AI Agent 提供调度决策的事实源（SOT）。

---

## 一、什么是 Flow？为什么需要 Flow？

在 Legado 书源工程中，一次操作往往不是单条命令就能解决的：
- 它涉及**外部网络波动**（Cloudflare 盾、反爬频控、字符乱码）；
- 它涉及**严格的上下游因果依赖**（搜索没通前改目录毫无意义；没在真机验证前宣称修复是假修复）；
- 它涉及**真机硬件资源互斥**（一台 Android 手机同时只能承载一个真机调试/批检作业，否则会导致信道死锁）。

**Flow（工作流）** 是跨越多个底层模块（Gate、Diagnose、Patch、MCP、Verify、Closeout），具备**前置门禁拦截、中间状态保护、真机闭环验证与最终台账收工**的事务性多阶段流水线。

---

## 二、何时调用 Flow vs 何时不需要 Flow（决策矩阵）

在接到人类指令或准备执行动作前，Agent 和开发者应先查阅下表决定是否唤起 Flow：

| 任务意图 / 场景 | 是否需要 Flow？ | 推荐调度路径 | 核心原因与设计考量 |
|---|---|---|---|
| **现有书源失效，需排查修复** | **必须调用 Flow** | **Flow 1: 单源深度修复流** (`source-cli diagnose` -> `repair`) | 必须执行 L0~L2 门禁与单向诊断链，杜绝盲目修改选择器；必须真机校验通过方可交付。 |
| **发现新小说站，需制作新书源** | **必须调用 Flow** | **Flow 2: 新书源创作流** (`source-cli site-probe` -> `scaffold` -> `push` -> `verify`) | 需完整经历原始 HTML 抓取、编码识别、脚手架生成、真机推送与全链路真机检验。 |
| **书源域名失效 (404/挂站/跳博彩)** | **必须调用 Flow** | **Flow 3: 域名猎取迁移流** (`source-cli hunt` -> `site-probe` -> `migrate`) | 不得胡乱改写解析规则，必须寻找镜像站，并在全源范围内递归替换主机路径。 |
| **书架大批量书源全面健康巡检** | **必须调用 Flow** | **Flow 4: 批量巡检波次流** (`source-cli wave` / `search-wave`) | 必须持有信道排他锁，执行多线程 PC 预检分流，并以单批次方式送入真机验证。 |
| **查询 CSS 语法、JS 拓展或加解密方法** | ❌ **不需要 Flow** | **直接查阅标准参考库** (`skills/legado-book-source/references/*.md`) | 纯只读知识检索，无需接触设备与运行时。 |
| **检查手机端 MCP 是否在线或空闲** | ❌ **不需要 Flow** | **直接执行单条状态查询** (`source-cli check channel`) | 纯状态查询工具，无多阶段依赖。 |
| **修改书源非功能性属性 (名称/分组/延时)** | ❌ **不需要 Flow** | **直接编辑 JSON 文件**，若需生效调 `source-cli source push` | 仅改变元数据或配置参数，不影响正文与目录抓取逻辑，无需启动整套诊断流水线。 |
| **发现页 (exploreUrl) 排障** | ❌ **默认不走 Flow** | 仅在用户**明确点名**要求时才介入 | 平台纪律：默认 `checkDiscovery=false`，发现页失效不影响正常阅书，杜绝浪费真机预算。 |
| **离线检测 JSON 结构是否符合 Legado 规范** | ❌ **不需要 Flow** | **本地单步 Schema 校验** (`source-contracts`) | 纯语法与契约校验，无需网络与手机通信。 |

---

## 三、4 大核心 Flow 全景图与执行规约

### Flow 1: 单源深度修复流 (Deep Repair Flow)

这是最高频的排障工作流，严格落实“单向诊断链”与“真机验证闭环”：

```mermaid
graph TD
    Start(["发起修复请求"]) --> Lock["1. 信道门禁检查<br/>source-cli check channel<br/>(确保手机未被其他批检占用)"]
    Lock --> Gate["2. L0~L2 连通性与存活门禁<br/>- L0 语法契约<br/>- L1 域名存活/解析<br/>- L2 反爬墙/停放页检测"]
    
    Gate --"域名已死/跳车"--> DivertHunt["分流至 Flow 3: 域名猎取迁移"]
    Gate --"视频/音频源"--> DivertMedia["分流至专用音视频路由"]
    Gate --"正常小说站"--> Diagnose["3. 严格单向诊断链<br/>source-cli diagnose<br/>① Search: 自动嗅探搜索间隔与CF盾<br/>② Detail: 真实详情 vs fake_detail<br/>③ TOC: 目录列表与倒序检测<br/>④ Content: 正文提取与广告清洗"]
    
    Diagnose --> Patch["4. 补丁计划生成<br/>匹配站点家族并生成 PatchPlan"]
    Patch --> Push["5. 推送真机并上锁<br/>source push (自动申明 deep_active 锁)"]
    Push --> Verify["6. 真机闭环全链路校验<br/>手机端执行 check_source"]
    
    Verify --"校验失败"--> RetroFail{"重试预算<br/>(≤ 2次)"}
    RetroFail --"有预算"--> Diagnose
    RetroFail --"超限"--> MarkFail["标记不可修，恢复原始状态"]
    
    Verify --"校验成功 (真机变绿)"--> Closeout["7. 收尾与台账关门<br/>- ledger append (写入事件台账)<br/>- retro append (复盘陷阱沉淀)<br/>- 解除 deep_active 状态锁"]
    Closeout --> End(["修复完成并汇报用户"])
```

---

### Flow 2: 新书源创作流 (Source Creation Flow)

从零发现未知小说站点并生成工业级可用书源：

```mermaid
graph TD
    Start(["发现新小说站点 URL"]) --> Probe["1. 站点全要素探针扫描<br/>source-cli site-probe<br/>- 抓取原生 HTML (非浏览器DOM)<br/>- 探测 JS 动态写入表单<br/>- 嗅探字符集编码 (GBK vs UTF-8)"]
    
    Probe --> Identify["2. 站点家族特征识别<br/>- 笔趣阁家族 (SiteFamily::Biquge)<br/>- 杰奇小说系统 (Jieqi)<br/>- 独创架构站点 (Custom)"]
    
    Identify --> Scaffold["3. 生成书源脚手架模板<br/>source-cli source scaffold<br/>生成合规的 JSON 初稿"]
    
    Scaffold --> Refine["4. 选择器微调与增强<br/>- 动态加载追加 webView 属性<br/>- 正文尾部配置 @ownText 去广告<br/>- 倒序目录追加 - 前缀"]
    
    Refine --> Push["5. 一键推送到手机 Legado<br/>source-cli source push --file ..."]
    
    Push --> Verify["6. 触发真机真实网络全链路校验<br/>手机端执行完整搜索->正文走通"]
    
    Verify --"不通过"--> Refine
    Verify --"真机显示校验成功"--> Closeout["7. 台账登记与版本归档<br/>- ledger append<br/>- 规则入库"]
    Closeout --> End(["新书源交付成功"])
```

---

### Flow 3: 域名猎取与全量迁移流 (Domain Hunt & Migration Flow)

当目标站点由于不可抗力更换域名、原域名解析失败或跳转博彩站时触发：

```mermaid
graph TD
    Start(["书源域名失效 (404/挂站/跳转)"]) --> Seeds["1. 加载搜书与镜像种子库<br/>config/domain_hunt_seeds.json"]
    
    Seeds --> Hunt["2. 并发猎取候选域名<br/>source-cli hunt<br/>在搜索引擎与同源镜像中挖掘替代 host"]
    
    Hunt --> CandidateProbe["3. 候选域名连通性探针<br/>source-cli probe<br/>验证候选站内容是否与原站一致"]
    
    CandidateProbe --"无有效镜像"--> MarkDead["标记域名彻底死亡<br/>写入 verify_skip_rules.json<br/>停用该书源"]
    
    CandidateProbe --"找到有效新域名"--> Migrate["4. 全源路径递归替换迁移<br/>source-cli migrate<br/>- 替换 bookSourceUrl<br/>- 替换封面/详情/搜索绝对路径<br/>- 刷新 hostKey"]
    
    Migrate --> PushVerify["5. 推送真机并全链路验证<br/>在新域名下完整跑通校验"]
    
    PushVerify --"通过"--> Closeout["6. 沉淀域名迁移日志<br/>ledger append (action: migrate)"]
    Closeout --> End(["域名迁移完成"])
```

---

### Flow 4: 批量巡检与波次调度流 (Batch Wave Triage Flow)

针对几十到上百个书源的高并发、大批量体检与快速分流：

```mermaid
graph TD
    Start(["传入书源清单 URLs File"]) --> Lock["1. 申请全局排他信道锁<br/>source-cli check channel"]
    
    Lock --> SkipFilter["2. 本地快速门禁过滤<br/>比对 config/verify_skip_rules.json<br/>直接跳过已知死站与豁免站"]
    
    SkipFilter --> Triage["3. 多线程 PC 预检分流 (Triage)<br/>- 搜索正常但目录失败 -> toc 队列<br/>- 搜索失败且 404 -> dead 队列<br/>- 搜索返回 alert 提示 -> 频控等待队列<br/>- 媒体类 -> 视频音频分流队列"]
    
    Triage --> BatchPatch["4. 批量生成针对性补丁<br/>针对各分流队列应用轻量针对性修复"]
    
    BatchPatch --> SingleBatchVerify["5. 单批次手机端真机校验<br/>一次性打包下发给手机 Legado<br/>(严禁在同一设备上并发多任务)"]
    
    SingleBatchVerify --> Aggregate["6. 汇总生成校验报告<br/>输出 REPORT_JSON 与失败清单"]
    
    Aggregate --> Unlock["7. 释放全局信道锁"]
    Unlock --> End(["波次巡检收工"])
```

---

## 四、核心防护与避坑纪律 (Hard Rules)

1. **信道互斥（One Device, One Flight）**：
   - 手机端 Legado 是单体进程，无法同时处理两个批检或高频并发调试任务。
   - 进入任何包含真机操作的 Flow 前，**必须先执行信道检查**；退出 Flow 时必须保证释放占用。
2. **频控严禁改写选择器（Rate Limit Shield）**：
   - 当诊断在 Search 层发现 `alert("搜索间隔")`、403 或 5 秒盾时，Flow 会立即将该任务置入冷却队列或挂起，**严禁在此类网络频控状态下改动原本正确的选择器**。
3. **关闭非必要环节（Discovery Off by Default）**：
   - Legado 书源的“发现”页规则（`ruleExplore`）仅用于书城信息流展示，与读者阅读核心流程无关。
   - 所有自动化 Flow 均强制配置 `checkDiscovery=false`，以节省珍贵的真机调试与网络流量预算。
