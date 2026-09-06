# Legado 书源登录与鉴权检查指南 (Login & Auth Verification)

针对需要登录个人账号才能浏览的书源（如特定 VIP 章节、书架同步、防盗链限制），Legado 提供了内置的登录检查脚本与 Cookie 保持机制。

---

## 一、登录状态检查规则 (loginCheckJs)

在书源基本配置中，`loginCheckJs` 会在发起请求前或刷新书架时执行。
- 若检查失败，阅读 App 会提示用户重新输入账号密码或弹出登录网页。

```javascript
// 示例：检查 Cookie 中是否存在登录会话标识
function checkLogin() {
    var uid = cookie.getKey(baseUrl, "uid");
    var token = cookie.getKey(baseUrl, "auth_token");
    if (!uid || !token) {
        // 尝试发请求检测个人中心页面
        var res = java.ajax(baseUrl + "/user/profile");
        if (!res || res.includes("请登录") || res.includes("login.html")) {
            return false;
        }
    }
    return true;
}
checkLogin();
```

---

## 二、登录与自动登录 URL 配置

- **`loginUrl`**：输入登录网页地址，如 `https://example.com/login.html`。
- **WebView 模式**：
  若登录页面包含图形验证码或第三方 OAuth，可在 `loginUrl` 末尾添加参数 `{"webView": true}`，阅读 App 会弹出内置浏览器由用户完成交互并自动持久化 Cookie。
