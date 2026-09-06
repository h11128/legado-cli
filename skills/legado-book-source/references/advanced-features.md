# Legado 书源高级功能实战指南 (Advanced Features)

基于 134 个复杂生产书源（如喜漫漫画、霹雳书屋、3A小说、TapManga）的高阶技巧总结。

---

## 1. 全局 JavaScript 库 (`jsLib`)

`jsLib` 是书源根级别的字段，可注入通用的加解密算法（如 AES、DES、RC4、LZString、MD5）或公共工具函数，供各个阶段调用。

### 示例：注入 LZString 解压缩
```json
{
  "jsLib": "var LZString = { decompressFromBase64: function(str) { ... } };",
  "ruleSearch": {
    "bookList": "@js: LZString.decompressFromBase64(result)"
  }
}
```

### 适用场景：
- 图片解密：配合 `coverDecodeJs`（封面解密）与 `imageDecode`（正文图片解密）。
- API 响应解密：对服务器返回的 Base64 / AES 密文进行反解。

---

## 2. 动态源变量控制 (`source.getVariable()` / `source.setVariable()`)

允许用户在阅读 App 界面为该书源设置自定义参数（如“线路选择”、“倒序排序”、“高清画质开关”）。

### 示例：根据用户变量切换漫画目录排序 (TapManga)
```javascript
<js>
var z = source.getVariable(); // 读取用户在App设置的源变量
if (z === "目录乱序") {
    // 按集数序号正向数字排序
    items.sort(function(a, b) {
        var numA = parseInt((a.text.match(/\d+/) || [0])[0], 10);
        var numB = parseInt((b.text.match(/\d+/) || [0])[0], 10);
        return numA - numB;
    });
} else {
    items.reverse();
}
items;
</js>
```

---

## 3. 多接口/多域名自动轮询与故障切换

在搜索或正文规则中，通过 `java.ajaxAll` 或备选域名列表进行健康探测：

```javascript
@js:
var hosts = ["https://api1.example.com", "https://api2.example.com", "https://api3.example.com"];
var key = java.encodeURI(key, "UTF-8");
var foundUrl = "";
for (var i = 0; i < hosts.length; i++) {
    var check = java.ajax(hosts[i] + "/ping");
    if (check && check.indexOf("ok") !== -1) {
        foundUrl = hosts[i] + "/search?q=" + key;
        break;
    }
}
foundUrl || (hosts[0] + "/search?q=" + key);
```

---

## 4. 特殊选择器语法增强 (AllInOne / 区间切片)

- **列表反向**：在选择器开头加 `-`（如 `-ul.list li`），可直接反转获取到的节点列表。
- **数组切片**：`tag.div[-1:0]` 可实现倒序；`tag.li[1:10]` 截取指定区间。
- **排除语法**：`tag.li[!0:-1]` 排除首尾无用干扰项。
- **首规则省略**：`head@.1@text` 相当于 `head@children.1@text`。

---

## 5. 变量生命周期对照表 (Variable Scope & Lifecycle)

在书源开发与修复中，必须严防变量生命周期错乱导致的并发污染：

| 变量操作方法 | 作用域与生命周期 | 持久化方式 | 典型适用场景 | 风险防范 |
|---|---|---|---|---|
| **`source.put(k, v)`** / **`source.get(k)`** | **书源级 (Source-level)** | **持久化到数据库** | 存储书源全局配置、账号 Token、全局 Cookie、CF 拦截计数 | 跨书籍共享，**严禁存放单本书籍的临时状态** |
| **`book.putVariable(v)`** / **`book.getVariable()`** | **书籍级 (Book-level)** | **跟随书籍对象持久化** | 存放单本书特有的解密秘钥、特定分卷状态 | 切换书籍后自动隔离 |
| **`java.put(k, v)`** / **`java.get(k)`** | **请求/会话级 (Session-level)** | **保存在单次解析内存中** | 在详情页提取参数传递给后续目录/正文请求 | 多并发任务时容易串扰，解析完成后会被清空 |
| **`@put:{k: "rule"}`** | **单次规则执行级 (Rule-level)** | **即时临时求值** | 规则链条中上一节点向下一节点传递中间提取值 | 无法跨请求访问 |

> **注意：无头与自动化校验陷阱**  
> 在通过命令行或自动化引擎执行校验（如 MCP `start_check_sources` 或后台 CLI）时，`loginCheckJs` 或各阶段**严禁无限制调用阻塞式交互命令**（如 `java.startBrowserAwait`）。若必须调用，应先通过计数器拦截（如 `if (count <= 3)` 静默重试，超过则直接抛异常或退出），避免后台守护进程永远假死。

---

## 6. 大型目录展开与多 URL 分页 (`ruleToc.nextTocUrl`)

针对章节数上千或分页加载的超长目录，Legado 支持三种分页与数组返回格式：

### 方式 1：单一分页链接
```json
{
  "ruleToc": {
    "nextTocUrl": "a.next-page@href"
  }
}
```

### 方式 2：JavaScript 动态生成全量目录 URL 数组
当目录总页数已知时，可在 `nextTocUrl` 中直接返回所有分页 URL 的 JSON 数组，阅读客户端会并发自动拉取拼合：
```javascript
"nextTocUrl": "<js>\nvar list = [];\nvar totalPages = parseInt(result.match(/共 (\d+) 页/)[1]);\nfor (var i = 2; i <= totalPages; i++) {\n    list.push(baseUrl.replace(/page_1/, \"page_\" + i));\n}\nJSON.stringify(list);\n</js>"
```

---

## 7. 漫画图源输出模式规范 (Manga Source)

漫画源的正文规则（`ruleContent.content`）要求输出图片标签列表，常见以下两种规范：

1. **直接输出带有 `<img>` 的 HTML**（推荐，客户端解析最稳健）：
```javascript
"ruleContent": {
  "content": "@js:\nvar urls = JSON.parse(result).images;\nurls.map(function(u){ return \"<img src=\\\"\" + u + \"\\\">\"; }).join(\"\n\");"
}
```
2. **输出多行以换行符 `\n` 分隔的纯图片绝对 URL**（阅读 App 亦能原生自动渲染为漫画流）。

---

## 8. 动态加载 (`webView`) 规则注入语法规范

阅读 App 针对动态 JavaScript 渲染提供了 `webView` 支持，但**不同层级的拼接语法截然不同**：

1. **根级 URL 配置 (如 `searchUrl` / `bookInfoUrl`)**：直接在 URL 之后加逗号和 JSON：
   `https://example.com/search?q={{key}},{"webView": true}`

2. **二级字段规则 (如 `chapterUrl` / `nextTocUrl` / `content`)**：
   **严禁**直接写为 `tag.a@href,{"webView":true}`（语法错误）。必须采用以下合法形式：
   - **正则末尾追加**：`tag.a@href##$##,{"webView":true}`
   - **宏变量包裹**：`{{@@tag.a@href}},{"webView":true}`
   - **JS 动态追加**：`@js: result + ',{"webView":true}'`

