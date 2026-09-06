# Legado CSS 与选择器规则速查 (Selector Reference)

## 核心口诀
> **"列表规则定容器，字段规则提内容；容器定位不加@，字段提取必带@；ownText排干扰，text取全部，html带标签；位置默认就是0，正则净化放最后；属性提取必须加，href与src不能忘。"**

---

## 一、四种核心提取类型对比

| 提取标识 | 行为说明 | 典型适用场景 |
|---|---|---|
| `@ownText` | **只提取当前标签自身直接包含的文字**，排除所有子标签（如 `<a>`、`<b>`、广告等） | **正文提取推荐**；列表简介中排除子标签干扰 |
| `@text` | **递归提取该元素及其所有子元素的文本**，拼接为整段字符串，不保留换行 | 提取书名、作者、最新章节标题等单行短文本 |
| `@html` | **提取完整 HTML 源码**，保留 `<p>`、`<img>`、`<a>` 等标签结构 | 正文保留特殊排版或包含行内插图时 |
| `@textNodes`| **按直接子文本节点分段提取**，保持原始分段结构 | 正文分段处理、多行列表文本 |

---

## 二、属性提取规则

在定位到具体标签后，通过 `@属性名` 获取属性值：
- **链接**：`a@href`
- **图片**：`img@src` 或 `img@data-original` / `img@data-src`
- **数据属性**：`div@data-bid`、`span@data-id`

---

## 三、常见选择器实战语法

### 1. 列表规则（bookList / chapterList）
- **原则**：只定位容器元素，**严禁添加 `@text` 或 `@元素`**。
- **示例**：
  - CSS 语法：`ul.book-list > li`、`div.box div.item`
  - 原生切片（强烈推荐）：`ul li!0`（排除第1项）、`ul li[1:10]`（截取区间）、`ul li.-1`（取最后1项）
  - 警告：Jsoup 对标准 CSS3 伪类（`:first-child` / `:last-child` / `:not()`）支持较弱且不稳定，优先使用阅读原生下标

### 2. 字段提取规则（name, author, bookUrl, tocUrl 等）
- **书名**：`h4.bookname a@text` 或 `h1@text`
- **链接**：`h4.bookname a@href`（必须带 `@href`，避免详情 URL 被误解析为 `baseUrl`）
- **作者**：`span.author@text` 或 `.info span:nth-child(2)@text`
- **最新章节**：`dd.latest a@text`

### 3. 正则净化（##）
在任何提取规则末尾可通过 `##匹配正则##替换内容` 净化广告或多余符号：
- 清理前后缀：`.content@text##本站域名.*|请收藏本站##`
- 替换多余空行：`.content@text##\s+##\n`
- 仅保留第一个匹配项：`##OnlyOne形式##...###`

---

## 四、绝对禁止的反模式 (Negative Constraints)

1. **严禁提取 `<select>` 标签的 `@value`**：
   - HTML 中 `<select>` 元素本身**没有** `value` 属性；
   - 必须通过子项提取：`select option@value` 或 `select option:not([selected])@value`。
2. **严禁在容器列表加 `@text` 或叶子标签**：
   - `chapterList` / `bookList` 必须只停留在条目容器（如 `ul.list li`），严禁写成 `ul.list li@tag.a`。

