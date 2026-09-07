# 书源修复常见陷阱速查全集 (Trap Catalog)

本文档是 Legado 书源修复与工程自动化中的核心陷阱全景参考库，按处理生命周期与解析层级系统分类。

---

## 一、 搜索链路与假详情陷阱 (Search Layer & Fake Detail)

### 1. 假详情页下沉 (fake_detail_empty_search / wmp8)
- **现象**：搜索不存在的书名时返回 1 条假结果，随后解析详情或目录报 `TocEmptyException: 目录列表为空`。
- **根因**：阅读 App 引擎 (`BookList.kt`) 规定：当搜索无结果且书源未配置 `bookUrlPattern` 时，会自动将该搜索页面当作 302 跳转后的书籍详情页，调用 `ruleBookInfo` 尝试解析。如果 `ruleBookInfo.name` 规则写得过宽（如全局抓取 `h1@text` 或 `<title>`），就会将搜索结果提示（如 `搜索结果: 0条`）误当成真实书名。
- **修复措施**：
  1. 必须配置准确的 `bookUrlPattern`（如 `https?://www\\.site\\.com/book/\\d+\\.html`）。只要配置了非空值，引擎在空搜索时便不会误触发详情回退。
  2. 收窄 `ruleBookInfo.name` 的容器范围（如 `div.book-info h1@text`），严禁使用全局裸 `h1` 或 `<title>` 兜底。
  3. 检查 `ruleSearch.bookUrl`，严禁在末尾追加 `||@js:baseUrl`。

### 2. 搜索提示行被误当作书籍条目 (search_notfound_row_matches_bookList)
- **现象**：搜索无结果时，页面中出现一条书名为“没有找到相关小说”的假数据。
- **根因**：站点（如 DedeCMS）将“未找到”提示渲染在与真实数据相同结构的表格行内（如 `<tbody><tr><td>没有找到...</td></tr></tbody>`），导致 `bookList` 选择器命中该提示行。
- **修复措施**：在 `bookList` 选择器中增加链接存在性断言，如 `tbody tr:has(a)`。注意：Legado 对 `tag.tr:has(a)` 的支持不完整，应使用标准 CSS 标签名 `tr:has(a)`。

### 3. 作者提取错误：高亮标签干扰 (search_author_highlight_span)
- **现象**：搜索出来的作者名变成了搜索关键词本身。
- **根因**：标题中的关键词被后端包裹了 `<span style="color:red">关键词</span>` 进行高亮，而原规则简单粗暴地提取了第一个 `span`（如 `span.0@text`）。
- **修复措施**：将选择器精确定位到真实作者节点，如 `span[itemprop=author]@text` 或 `div.author@text`。

### 4. 搜索结果为加密/Base64 壳 (inte_base64_search / App JSON)
- **现象**：返回体形如 `inte_base64:{"c":"..."}`，常规 CSS 选择器全部失效。
- **修复措施**：在 `ruleSearch.bookList` 前置 `@js:` 预处理脚本，先剥离前缀，调用 `JSON.parse` 并配合 `java.base64Decode` 解码后，通过 `java.setContent` 重新注入 HTML 进行选择器提取。

---

## 二、 目录与正文抓取陷阱 (TOC & Content Layer)

### 1. 详情链接指回搜索页 (search_url_as_detail / bookUrl class-space)
- **现象**：点击书籍详情直接重载为搜索列表页，无法进入章节目录。
- **根因**：`ruleSearch.bookUrl` 提取属性遗漏了 `@href`，或类名选择器中间含空格未正确分词（如 `class.book-item a` 未写为 `class.book-item@tag.a@href`）。
- **修复措施**：显式声明属性抽取 `@href`，并清理尾部无效的回退链接。

### 2. 假目录与多目录容器混淆 (Multi-TOC Containers)
- **现象**：章节列表为空，或者抓取到的全部是“最新章节”前 10 章而非全量目录。
- **根因**：笔趣阁等站点页面内存在两套目录容器（一套为顶部“最新章节”，另一套为“全部章节”列表）；如果规则写成 `.list-charts.0` 会误选最新章节。
- **修复措施**：使用 Rhino `@js:` 动态比对容器子链接数量，取包含超链接数量最多的容器作为目录根节点。

### 3. 正文 Base64 与混淆加密 (content_qsbs_bb_base64)
- **现象**：正文提取为空，或仅抓取到类似 `qsbs.bb('...')` 的加密函数调用字符串。
- **修复措施**：在 `ruleContent.content` 中使用 `@js:`，通过 `indexOf` 截取密文字符串，直接调用 `java.base64Decode()` 解码并输出纯文本。

---

## 三、 域名迁移与信道防卡死 (Migration & Anti-Stall)

### 1. 盲目迁移至无关 CMS 假站点 (migrate_false_friend_cms)
- **现象**：站点 301 重定向到新域名，迁移后搜索或目录全报 404。
- **根因**：旧域名被抢注或转手挂靠了完全无关的博客/影视 CMS（如 Z-Blog），原路径根本不存在。
- **修复措施**：迁移前必须嗅探新域名的 CMS 架构与测试路径；若为无关壳站点，直接标记 `disable` 并搜寻真正的小说孪生镜像。

### 2. 假成功汇报阻断 (check_search_discovery_both_off_vacuous)
- **现象**：批量校验耗时只有 1~5ms 且返回成功，但实际上根本没有请求任何网页。
- **根因**：MCP 校验参数中同时设置了 `checkSearch=false` 与 `checkDiscovery=false`，阅读 App 没有任何待校验项，直接空转返回。
- **修复措施**：平台门禁强制校验项不得全为空；没有搜索功能时必须开启 `checkDiscovery=true`。

### 3. 真机排他锁僵尸与看门狗超时 (mcp_lock_zombie / serial_url_timeout)
- **现象**：前序任务异常退出导致信道锁被死进程占用，后续全部命令报 `channel busy` 退出。
- **修复措施**：`source-cli check channel` 自动检测并释放超时 15 分钟的陈旧锁；在调度长队列时使用 `source-cli serial --url-timeout-s 120`，超时由看门狗直接杀死子进程并推进下一 URL。
