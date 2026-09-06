# Legado 订阅源 (RSS/Article) 规则开发指南

## 概述

Legado 除了小说/漫画书源外，还支持**订阅源**（对应 RSS、新闻资讯、专栏博客、公众号文章订阅）。订阅源与普通书源不同，不分卷/章节，只有文章列表与内容页。

---

## 订阅源类型与解析流程

Legado 内部解析订阅源时分三类：

1. **标准 RSS 源**：
   - 仅填写 `sourceName` 与 `sourceUrl`（RSS XML 链接），无需编写规则，App 采用内置标准 RSS 引擎解析。
2. **有列表规则和描述规则的源**：
   - 填写了 `ruleArticles`（列表）、`ruleTitle`（标题）、`ruleLink`（文章链接）、`ruleDescription`（摘要）。无需进入文章正文即可在列表预览摘要。
3. **有列表规则和正文内容规则的源**：
   - 填写了 `ruleArticles`、`ruleTitle`、`ruleLink` 以及 `ruleContent`（正文规则）。点击后加载完整正文。

---

## 订阅源字段规范 (SubscriptionSource)

```json
{
  "sourceName": "科技资讯",
  "sourceUrl": "https://example.com/feed",
  "sourceIcon": "https://example.com/icon.png",
  "sourceGroup": "科技",
  "enabled": true,
  "ruleArticles": "div.article-item",
  "ruleNextPage": "a.next@href",
  "ruleTitle": "h2.title@text",
  "rulePubDate": "span.date@text",
  "ruleImage": "img@src",
  "ruleLink": "a@href",
  "ruleDescription": "p.summary@text",
  "ruleContent": "div.entry-content@html"
}
```

---

## 规则细节与注意事项

1. **分页机制 (`ruleNextPage`)**：
   - 订阅源下拉刷新时拉取第一页；上拉加载时触发 `ruleNextPage`。
   - **注意**：订阅源通常不支持在 URL 中写 `{{page}}` 宏变量。若需动态分页，需在 `ruleNextPage` 中返回下一页的实际链接，或用 JavaScript 维护页码。
2. **正文净化**：
   - `ruleContent` 提取正文支持完整 HTML 标签（`@html`），可使用 `##regex##replace` 过滤底部广告或版权声明。
