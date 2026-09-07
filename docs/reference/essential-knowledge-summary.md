# Legado书源精华知识汇总

> **来源**: 非官方精华文档（金黄级）
> **整理时间**: 2025-01-20
> **价值**: 💎💎💎💎💎（五星精华）

---

## 📋 文档清单

1. **legado知识库.md** (3009行) - 基于官方源码（约40万行）深度整合
2. **活力宝的书源日记231224.txt** (735行) - 实战技巧与经验
3. **阅读教程AI提取精华，人工润色 已矫正过.txt** (486行) - 通俗易懂教程
4. **天天的鸟蛋蛋的书源日记.md** (638行) - HTML基础与选择器详解

---

## 🚨 核心准则：必须基于真实 HTTP HTML 编写规则

编写书源规则前必须抓取未经浏览器美化渲染的原始 HTTP 响应，杜绝因 DevTools 动态修正或 JS 渲染造成规则实机失效。

详细核对清单与操作流程参见专用指南：[`docs/guides/html-authenticity-checklist.md`](../guides/html-authenticity-checklist.md)。

---

## 💎 核心精华提取

### 一、实战技巧（来自活力宝）

#### 1. CSS选择器简写方法

**完整写法** → **简写**
```
class.名字1@text          →  .名字1@text
class.名字1 名字2 名字3@text  →  .名字1.名字2.名字3@text
class.xxx@li@a@text      →  .xxx li a@text
id.最优选哦@text          →  #最优选哦@text
```

**技巧**：
- 优先使用id（因为id具有唯一性）
- 同一标签里有id和class时，优先选id
- 多个class名中，如果某个唯一出现，可以单独使用
- `@`连接上下层符号可以用空格代替（除非class有多个属性名）

#### 2. 解决搜索30秒超时问题

**方法1: 登录检查JS**
```javascript
cookie.removeCookie(source.getKey())
result
```

**方法2: 搜索地址清空cookie**
```
{{cookie.removeCookie(source.getKey())}}/search0f.html?searchkey={{key}}
```

#### 3. 请求头配置（防盗链处理）

**基本配置**：
```json
{
  "User-Agent": "Mozilla/5.0 (Linux; Android 11) Mobile Safari/537.36"
}
```

**防盗链处理**：
```json
{
  "User-Agent": "Mozilla/5.0 (Linux; Android 11) Mobile Safari/537.36",
  "Referer": "https://official.bkvvvvv.com/"
}
```

**动态Referer**：
```json
{
  "headers": {
    "Referer": "{{baseUrl}}"
  }
}
```

**或使用@js**：
```javascript
@js:JSON.stringify({"referer":baseUrl})
```

#### 4. 常用User-Agent

**小米浏览器UA**（百度流畅，不要全局使用）：
```
Mozilla/5.0 (Linux; Android 12) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.5414.86 Mobile Safari/537.36 XiaoMi/MiuiBrowser/17.2.79 swan-mibrowser
```

**苹果手机UA**：
```
Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1
```

**手机百度UA**：
```
Mozilla/5.0 (Linux; Android 11; PCAM10 Build/RP1A.200720.011; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/97.0.4692.98 Mobile Safari/537.36 T7/13.32 SP-engine/2.70.0 baiduboxapp/13.32.5.10 (Baidu; P1 11) NABar/1.0
```

**WebView模式**：
```json
{
  "webView": true,
  "header": {
    "referer": "https://mjjxs.net"
  }
}
```

---

### 二、通俗教程（来自阅读教程AI提取）

#### 1. 书源概念（冰箱比喻）

**书源 = 网址 + 规则**

- **冰箱地址** = 小说网站网址（如 www.xxx.com）
- **食物清单** = 告诉APP去哪里找书名、作者、章节、正文
- **规则** = 说明书，比如"书名藏在 <a class='book-name'> 这个盒子里"

#### 2. 检查元素三步走（核心技能）

```
第一步：右键点击想抓的东西，选择"检查"
第二步：看高亮的代码，记住标签、类名、id
第三步：写出规则
```

**规则示例**：
- 用类名：`.book-name`
- 用标签名：`a`
- 用id：`#book-name`
- 组合：`div.list li a`

#### 3. 四大规则详解

##### 4.1 搜索规则

**搜索地址**：
```
https://www.xxx.com/search?q={{key}}&page={{page}}
```

**书籍列表规则**：`.book-list li`

**字段规则**：
- 书名：`h2 a@text`
- 作者：`.author@text`
- 封面：`img@src`
- 详情页链接：`h2 a@href`

##### 4.2 详情页规则

**字段规则**：
- 书名：`h1@text`
- 作者：`span.author@text`
- 分类：`span.category@text`
- 简介：`div.intro@text`
- 封面：`img.cover@src`
- 目录页链接：`a.toc@href`

##### 4.3 目录规则

**字段规则**：
- 章节列表：`ul.chapter-list li`
- 章节名称：`a@text`
- 章节链接：`a@href`
- 目录下一页：`a.next@href`

**倒序技巧**（最新章在前）：
```
-ul.chapter-list li
```

##### 4.4 正文规则

**字段规则**：
- 正文（保留段落）：`div.content@html`
- 正文（只取文字）：`div.content@ownText`
- 正文下一页：`a.next-page@href`

#### 4. 提取类型详解

| 类型 | 作用 | 示例 | 结果 |
|------|------|------|------|
| `@text` | 取所有文字（包括子标签） | `h2@text` | "斗罗大陆" |
| `@ownText` | 只取当前标签文字，不要子标签 | `p@ownText` | 只取p标签直接包含的文字 |
| `@textNode` | 跟@ownText类似，但会分段 | `div@textNode` | ["Hello", "World"] |
| `@html` | 取完整HTML（保留标签） | `div.content@html` | 带标签的正文 |
| `@href` | 取链接地址 | `a@href` | "/book/123" |
| `@src` | 取图片地址 | `img@src` | "/cover/123.jpg" |
| `@data-xxx` | 取自定义属性 | `div@data-id` | "123" |

#### 5. 规则口诀

```
列表定位不加@，字段提取必带@。
位置默认就是0，不写也行。
text全都要，ownText最干净。
html带标签，属性加@不能忘。
净化放最后，网站千变要灵活。
```

**解释**：
- 列表规则（bookList）只写选择器，不加@
- 字段规则（name）必须写 选择器@提取类型
- 位置默认第一个，`li@text` 等价于 `li.0@text`
- @text最全，@ownText最干净（去广告）
- 提取属性必须加@，如`a@href`
- 净化广告的正则放在最后

---

### 三、HTML基础（来自天天的鸟蛋蛋）

#### 1. HTML层级关系示例

```html
<!-- 第1层: 最外层容器 -->
<div class="archive-posts">
    <!-- 第2层: 文章容器 -->
    <article id="post-6" class="archive-post">
        <!-- 第3层: 缩略图容器 -->
        <div class="thumbnail-box">
            <!-- 第4层: 图片 -->
            <img class="thumb-image" src="#" alt="图片">
        </div>
        <!-- 第3层: 内容容器 -->
        <div class="post-inner">
            <!-- 第4层: 标题 -->
            <h2 class="post-title">
                <!-- 第5层: 标题链接 -->
                <a href="#">标题</a>
            </h2>
        </div>
    </article>
</div>
```

#### 2. CSS选择器基础

**基础语法**：
```css
/* 元素选择器 */
div

/* 类选择器 */
.class-name

/* ID选择器 */
#id-name

/* 属性选择器 */
[type="text"]

/* 后代选择器 */
div p

/* 子元素选择器 */
div > p

/* 相邻兄弟选择器 */
h1 + p

/* 通用兄弟选择器 */
h1 ~ p
```

**常用选择器**：
- `*` - 选择所有元素（通用选择器）
- `elementname` - 选择所有指定类型的元素
- `.class-name` - 选择所有class包含该名称的元素
- `#id-name` - 选择id为该名称的元素

---

### 四、官方数据结构（来自legado知识库）

#### 1. BookSource（书源主类）

**核心标识**：
- `bookSourceUrl` - 书源地址（主键）
- `bookSourceName` - 书源名称
- `bookSourceGroup` - 书源分组

**书源类型**：
- `bookSourceType` - 0:文本, 1:音频, 2:图片, 3:文件, 4:视频

**规则配置**：
- `searchUrl` - 搜索URL
- `ruleSearch` - 搜索规则
- `ruleBookInfo` - 书籍信息规则
- `ruleToc` - 目录规则
- `ruleContent` - 正文规则

#### 2. TocRule（目录规则）

**核心字段**：
- `url` - 目录URL
- `chapterList` - 章节列表选择器
- `chapterName` - 章节名称
- `chapterUrl` - 章节URL
- `nextTocUrl` - 下一页目录URL（重要！）
- `reverseTocUrl` - 逆向目录URL

#### 3. ContentRule（正文规则）

**核心字段**：
- `url` - 正文URL
- `content` - 正文内容
- `replaceRegex` - 正则替换规则
- `nextContentUrl` - 下一页正文URL
- `webJs` - Web视图JS
- `sourceRegex` - 源正则

---

## 🎯 实战应用指南

### 快速上手流程

```
1. 打开目标网站
2. 按F12打开开发者工具
3. 右键点击书名，选择"检查"
4. 记住高亮的代码（标签、class、id）
5. 写出规则（.book-name@text）
6. 填入书源对应字段
7. 测试验证
```

### 常见问题解决

#### 问题1: 搜索30秒超时

**解决方案**：
- 登录检查JS: `cookie.removeCookie(source.getKey())`
- 搜索地址: `{{cookie.removeCookie(source.getKey())}}/search?key={{key}}`

#### 问题2: 防盗链

**解决方案**：
- 请求头添加Referer
```json
{
  "Referer": "{{baseUrl}}"
}
```

#### 问题3: 广告太多

**解决方案**：
- 使用`@ownText`代替`@text`
- 正则净化：`##广告词##`

#### 问题4: 章节顺序颠倒

**解决方案**：
- 列表规则前加`-`号
- `-ul.chapter-list li`

---

## 📚 推荐学习顺序

1. **第一步**：阅读"阅读教程AI提取精华"（通俗易懂）
2. **第二步**：学习"活力宝的书源日记"（实战技巧）
3. **第三步**：参考"天天的鸟蛋蛋的书源日记"（HTML基础）
4. **第四步**：查阅"legado知识库"（官方数据结构）

---

## ⭐ 关键记忆点

### 必记口诀

```
列表定位不加@，字段提取必带@。
位置默认就是0，不写也行。
text全都要，ownText最干净。
html带标签，属性加@不能忘。
```

### 核心规则

1. **id优先**：有id优先用id（唯一性强）
2. **简写原则**：`class.xxx@text` → `.xxx@text`
3. **空格替代**：`@`可用空格代替（多层选择时）
4. **防盗链**：添加Referer请求头
5. **去广告**：用`@ownText`+正则净化
6. **超时处理**：cookie.removeCookie()

### 提取类型速查

| 类型 | 用途 | 记忆 |
|------|------|------|
| `@text` | 取所有文字 | 全都要 |
| `@ownText` | 只取自己文字 | 最干净 |
| `@html` | 取完整HTML | 带标签 |
| `@href` | 取链接 | 链接@ |
| `@src` | 取图片 | 图片@ |

---

## 🔥 黄金技巧总结

1. **优先id**：唯一性最强，不易出错
2. **简写**：`class.xxx` → `.xxx`
3. **防盗链**：Referer请求头
4. **去广告**：`@ownText` + 正则
5. **超时**：cookie.removeCookie()
6. **倒序**：列表规则前加`-`
7. **分页**：nextTocUrl处理（包括select下拉菜单）⭐
8. **UA**：不要全局使用大厂UA（防盗版广告）

---

## 📄 下拉菜单分页与动态加载 (Select Pagination)
 
针对 `<select name="pageselect">` 下拉列表目录分页，切忌直接对 `select` 抽取 `@value`。
 
- 正确写法：`"nextTocUrl": "select[name='pageselect'] option:not([selected])@value"`
- 详细案例与边界陷阱参见：[`docs/guides/select-dropdown-pagination.md`](../guides/select-dropdown-pagination.md)
 
---
 
## 📖 核心参考文档体系
 
- 架构与流程：[`docs/reference/repair-adapter-architecture.md`](repair-adapter-architecture.md)
- 核心参考库：[`skills/legado-book-source/references/`](../../skills/legado-book-source/references/)
- 目录分页规则：[`docs/guides/toc-pagination-rules.md`](../guides/toc-pagination-rules.md)
- 排障陷阱速查：[`skills/legado-book-source-repair/references/trap-catalog.md`](../../skills/legado-book-source-repair/references/trap-catalog.md)

---

> **价值评估**: 💎💎💎💎💎（五星精华）
>
> **适用人群**: 初学者到进阶者全覆盖
>
> **学习建议**: 按推荐顺序学习，结合实战练习
