# Legado 书源编码与字符集处理指南

## 概述

开发 Legado 书源时，中文旧站点常采用 `GBK` / `GB2312` / `GB18030` 等非 UTF-8 编码。若未指定字符集，会导致请求乱码或解析失败。

---

## 常见编码类型

| 编码类型 | 特点与应用场景 |
|---|---|
| **UTF-8** | 默认编码，标准现代网站，配置中可省略 charset |
| **GBK** | 中文老网站最常见，覆盖汉字广泛 |
| **GB2312** | GBK 的子集，部分较早的小说站点使用 |
| **GB18030** | 最完整的中文字符集标准，兼容生僻字 |

---

## 如何判断网站编码

### 1. HTTP 响应头 (Content-Type)
在网络抓包或浏览器 Network 面板查看响应头：
```http
Content-Type: text/html; charset=GBK
```

### 2. HTML Meta 标签
网页源码头部中的 meta 声明：
```html
<meta charset="gbk" />
<!-- 或 -->
<meta http-equiv="Content-Type" content="text/html; charset=gb2312">
```

### 3. 乱码表征
- 搜索或正文返回为乱码拼音/符号（如 `ÎÒ°®ãá`），通常代表该站点为 GBK/GB2312。

---

## 书源配置方法

### 方式一：URL JSON 选项配置（推荐）

在 `searchUrl`、`bookInfoUrl` 等请求字段中，通过逗号附加 JSON 配置：

```json
{
  "searchUrl": "/modules/article/search.php,{\"method\":\"POST\",\"body\":\"searchkey={{key}}&searchtype=all\",\"charset\":\"gbk\"}"
}
```

- 使用逗号 `,` 分隔 URL 和 JSON 配置对象。
- `charset` 字段指定目标编码（`gbk` / `gb2312` / `gb18030`，不区分大小写）。
- 阅读客户端在发送请求及解析响应时，会自动按指定的 charset 转码。

### 方式二：JavaScript 动态构造

复杂站点需要动态计算参数时，在 `@js:` 块中返回带有配置对象的字符串：

```javascript
@js:
var ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
var headers = {"User-Agent": ua};
var body = "keyword=" + String(key) + "&page=" + String(page);
var option = {
  "charset": "gbk",
  "method": "POST",
  "body": String(body),
  "headers": headers
};
"https://www.example.com/search," + JSON.stringify(option)
```

---

## 内置编码与转码工具

Legado 在 JS 引擎中注入了辅助函数：

### `java.utf8ToGbk(str)`
将 UTF-8 字符串转换为 GBK 字符串：
```javascript
var gbkStr = java.utf8ToGbk("你好世界");
```

### `java.encodeURI(str, enc)`
对字符串进行指定编码的 URI 转义：
```javascript
// GBK URL 编码（常用于 GET 请求的 query 参数）
var encodedKey = java.encodeURI(key, "GBK");
var url = "/search.php?keyword=" + encodedKey + "&page=" + page;
```

---

## 典型示例

### 69书吧（经典 GBK POST 搜索）
```json
{
  "bookSourceName": "69书吧",
  "bookSourceUrl": "https://www.69shuba.com",
  "bookSourceType": 0,
  "searchUrl": "/modules/article/search.php,{\"method\":\"POST\",\"body\":\"searchkey={{key}}&searchtype=all\",\"charset\":\"gbk\"}",
  "ruleSearch": {
    "bookList": "class.newbox@tag.li",
    "name": "tag.a.0@text",
    "author": "tag.span.-1@text##.*：",
    "bookUrl": "tag.a.0@href",
    "coverUrl": "tag.img@src"
  }
}
```
