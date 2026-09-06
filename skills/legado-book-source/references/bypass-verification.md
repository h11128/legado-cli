# Legado 书源防爬验证绕过指南 (Bypass Cloudflare & Slider Captcha)

在部分小说/漫画网站中，搜索或正文请求可能会遭遇 Cloudflare 5秒盾拦截（HTTP 403/503/429）或滑块验证码（`_guard` 机制）。在 Legado 中，可以通过在书源规则中编写注入 JavaScript 调用阅读 App 的 `java.webView` 与内置加解密引擎实现半自动/自动绕过。

---

## 一、Cloudflare 5秒盾拦截自动处理 (bypassCloudflare)

当网站返回 403/503 或 HTML 正文包含 `Just a moment`、`Checking your browser` 时：
1. 先记录拦截次数 `cf_block_count`；
2. 若连续拦截 ≤ 3 次，调用 `java.webView(url, url, js)` 静默等待 5 秒获取渲染后带有有效 Cookie/Clearance 的页面；
3. 若自动驯服失败，调用 `java.startBrowserAwait(url, "CloudFlare验证")` 唤起手机原生浏览器等待用户点击一次通过。

```javascript
function bypassCloudflare(result) {
    try {
        var url = result.url();
        var body = result.body();
        var code = result.code();
        
        var isBlocked = (code === 403 || code === 503 || code === 429) ||
            (code === 200 && (body.match(/Just a moment/) || body.match(/Checking your browser/) || body.match(/DDoS protection/)));
        
        if (!isBlocked) {
            source.put("cf_block_count", "0");
            return result;
        }
        
        var count = parseInt(source.get("cf_block_count") || "0") + 1;
        source.put("cf_block_count", count.toString());
        
        if (count <= 3) {
            // 使用 WebView 自动静默渲染尝试获取 Cookie
            var html = java.webView(url, url, "setTimeout(function() { window.legado.getHTML(document.documentElement.outerHTML); }, 5000);");
            if (html && !html.match(/Just a moment|Checking your browser/) && java.connect(url).code() === 200) {
                source.put("cf_block_count", "0");
                return java.connect(url);
            }
        }
        
        // 自动绕过失败，唤起浏览器人工点击
        java.toast("需要Cloudflare验证，请在弹出的窗口中点击完成");
        java.startBrowserAwait(url, "Cloudflare验证");
        source.put("cf_block_count", "0");
        return java.connect(url);
    } catch(err) {
        java.log("CF验证处理异常: " + err);
        return result;
    }
}
```

---

## 二、滑动拼图/滑动验证自动生成轨迹 (Slider Bypass)

若目标站点采用基于 `_guard` 的滑块防护系统：
1. 从页面或 Cookie 中提取 `guard` 签名令牌；
2. 利用 `java.createSymmetricCrypto("AES/CBC/PKCS7Padding", key, iv)` 构建符合站点要求的加密器；
3. 生成带有时间戳漂移、贝塞尔加减速拟人化扰动的坐标轨迹数组 `[{x, y, timestamp}]`；
4. 将轨迹加密并作为 Cookie `guardret` 重放请求以完成自动验签。
