# 书源修复常见陷阱速查 (Trap Catalog)

## 1. 假详情页下沉 (fake_detail_empty_search)
- **现象**：搜索不存在的书名时返回 1 条结果，随后解析详情/目录报错（如 `TocEmptyException: 目录列表为空`）。
- **根因**：阅读 App 引擎 (`BookList.kt:100`) 规定：当搜索无结果且书源缺少 `bookUrlPattern` 时，会降级判定该搜索页面为 302 跳转后的书籍详情页，调用 `ruleBookInfo` 试图解析。若 `ruleBookInfo.name` 写得过宽（如全局匹配 `h1@text` 或 `<title>`），则会将搜索结果提示误当作真实书籍。
- **修复措施**：
  1. 必须配置准确的 `bookUrlPattern`。
  2. 修复 `ruleBookInfo.name`，使用详情页专有容器（如 `div.info h1@text`），绝不全局抓取裸 `h1` 或 `<title>`。
  3. 检查 `ruleSearch.bookUrl`，绝不在结尾添加 `||@js:baseUrl` 导致详情回退到搜索 URL。

## 2. 详情链接指回搜索页 (search_url_as_detail)
- **现象**：点击搜索结果书籍，页面显示为搜索结果列表，无法加载章节。
- **根因**：`ruleSearch.bookUrl` 漏写 `@href`，或末尾带了 `||@js:baseUrl` / `||baseUrl`，在提取失败时直接取当前搜索 URL 作为详情页。
- **修复措施**：显式指定 `a@href`，并确保选择器命中真正的详情页超链接。

## 3. 相对链接拼接异常 (relative_url_join)
- **现象**：章节或目录跳转到错误的域名，或带有双斜杠/缺少前缀。
- **根因**：使用了硬编码域名或拼接时丢掉路径层级。
- **修复措施**：直接返回相对路径，交给阅读 App 的 base URL 自动补齐；若需要 JS 补全，使用标准的阅读内置函数。
