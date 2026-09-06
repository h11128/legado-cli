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
