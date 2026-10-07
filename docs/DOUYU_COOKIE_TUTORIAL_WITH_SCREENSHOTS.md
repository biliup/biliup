# 斗鱼Cookie获取图文教程

> 📸 **需要提供的截图列表在文档末尾**

## 📌 为什么需要Cookie

**重要变更通知：** 自2026年9月21日起，斗鱼平台对API进行了重大调整：

- ❌ **未登录用户**：只能获取超清及以下画质，且可能每5分钟自动断流
- ✅ **登录用户**：可以获取原画（1080P60/2K）、蓝光4M等高码率流，不再断流

因此，为了录制高质量的斗鱼直播，您需要提供登录后的Cookie信息。

---

## 🖥️ 方法一：Chrome/Edge浏览器（推荐）

### 步骤1：登录斗鱼

1. 打开Chrome或Edge浏览器
2. 访问 https://www.douyu.com
3. 点击右上角"登录"按钮
4. 使用您的账号密码登录（或扫码登录）

**📸 需要的截图：**
- `screenshot-01-douyu-homepage.png` - 斗鱼首页，突出显示"登录"按钮
- `screenshot-02-login-page.png` - 登录页面

![登录斗鱼](./images/douyu-cookie-tutorial/screenshot-01-douyu-homepage.png)
*图1：斗鱼首页，点击右上角"登录"*

![登录表单](./images/douyu-cookie-tutorial/screenshot-02-login-page.png)
*图2：输入账号密码登录*

---

### 步骤2：打开开发者工具

1. 登录成功后，**不要关闭当前页面**
2. 按下 `F12` 键（或右键点击页面，选择"检查"）
3. 开发者工具面板会在浏览器底部或右侧打开

**📸 需要的截图：**
- `screenshot-03-open-devtools.png` - 右键菜单显示"检查"选项
- `screenshot-04-devtools-opened.png` - 开发者工具已打开的界面

![打开开发者工具](./images/douyu-cookie-tutorial/screenshot-03-open-devtools.png)
*图3：右键点击页面，选择"检查"*

![开发者工具界面](./images/douyu-cookie-tutorial/screenshot-04-devtools-opened.png)
*图4：开发者工具已打开*

---

### 步骤3：切换到Network（网络）面板

1. 在开发者工具顶部，找到 **"Network"** （网络）标签
2. 点击切换到Network面板
3. 如果Network面板是空的，按 `Ctrl+R`（Mac用户按 `Cmd+R`）刷新页面

**📸 需要的截图：**
- `screenshot-05-network-tab.png` - Network标签位置（用箭头标注）
- `screenshot-06-network-panel.png` - Network面板显示请求列表

![切换到Network标签](./images/douyu-cookie-tutorial/screenshot-05-network-tab.png)
*图5：点击"Network"标签*

![Network面板](./images/douyu-cookie-tutorial/screenshot-06-network-panel.png)
*图6：Network面板显示所有网络请求*

---

### 步骤4：查看请求头

1. 在Network面板中，会看到很多网络请求
2. 点击任意一个请求（建议选择 `www.douyu.com` 开头的请求）
3. 在右侧详情面板中，找到 **"Headers"**（请求头）标签

**📸 需要的截图：**
- `screenshot-07-select-request.png` - 选中一个请求（用红框标注）
- `screenshot-08-headers-tab.png` - Headers标签位置

![选择请求](./images/douyu-cookie-tutorial/screenshot-07-select-request.png)
*图7：点击任意一个douyu.com的请求*

![Headers标签](./images/douyu-cookie-tutorial/screenshot-08-headers-tab.png)
*图8：切换到Headers（请求头）标签*

---

### 步骤5：复制Cookie

1. 在Headers标签中，向下滚动找到 **"Request Headers"**（请求头）部分
2. 找到 **"cookie:"** 字段（注意是小写）
3. 该字段的值是一长串文本，类似：
   ```
   acf_username=xxx; acf_uid=123456; acf_auth=xxxxxx; acf_did=10000000000000000000000000001511; ...
   ```
4. **完整复制整个cookie值**（从第一个字符到最后一个字符）

**📸 需要的截图：**
- `screenshot-09-find-cookie.png` - 找到cookie字段（用箭头和高亮标注）
- `screenshot-10-copy-cookie.png` - 复制cookie的操作（显示右键菜单或选中状态）

![找到Cookie](./images/douyu-cookie-tutorial/screenshot-09-find-cookie.png)
*图9：找到"cookie:"字段*

![复制Cookie](./images/douyu-cookie-tutorial/screenshot-10-copy-cookie.png)
*图10：右键复制或按Ctrl+C复制整个cookie值*

**💡 提示：如何完整复制cookie**
- 点击cookie值，会出现选中状态
- 按 `Ctrl+C`（Mac用户按 `Cmd+C`）复制
- 或者右键点击cookie值，选择"Copy value"（复制值）

---

### 步骤6：配置到biliup

1. 打开biliup的Web管理界面
2. 找到您要录制的主播配置
3. 展开"斗鱼"平台设置
4. 在 **"登录 Cookie（douyu_cookie）"** 输入框中
5. 粘贴刚才复制的完整cookie字符串
6. **点击"测试Cookie"按钮验证**
7. 如果显示✅成功，点击保存

**📸 需要的截图：**
- `screenshot-11-biliup-ui.png` - biliup的斗鱼配置界面
- `screenshot-12-paste-cookie.png` - 粘贴cookie到输入框
- `screenshot-13-test-success.png` - 测试成功的提示
- `screenshot-14-save-config.png` - 保存配置

![biliup配置界面](./images/douyu-cookie-tutorial/screenshot-11-biliup-ui.png)
*图11：biliup的斗鱼配置页面*

![粘贴Cookie](./images/douyu-cookie-tutorial/screenshot-12-paste-cookie.png)
*图12：粘贴完整的cookie字符串*

![测试Cookie](./images/douyu-cookie-tutorial/screenshot-13-test-success.png)
*图13：点击"测试Cookie"，显示✅ Cookie有效*

![保存配置](./images/douyu-cookie-tutorial/screenshot-14-save-config.png)
*图14：保存配置*

---

## 🦊 方法二：Firefox浏览器

### 步骤1-2：登录并打开工具

1. 访问 https://www.douyu.com 并登录
2. 按 `F12` 打开开发者工具

### 步骤3：查看Cookie存储

1. 点击 **"存储"**（Storage）标签
2. 展开左侧的 **"Cookie"** 项
3. 点击 `https://www.douyu.com`
4. 右侧会显示所有cookie项

**📸 需要的截图：**
- `screenshot-15-firefox-storage.png` - Firefox的存储标签
- `screenshot-16-firefox-cookies.png` - Cookie列表

![Firefox存储标签](./images/douyu-cookie-tutorial/screenshot-15-firefox-storage.png)
*图15：Firefox的"存储"标签*

![Firefox Cookie列表](./images/douyu-cookie-tutorial/screenshot-16-firefox-cookies.png)
*图16：显示所有cookie项*

---

### 步骤4：组装Cookie字符串

Firefox需要手动组装cookie字符串。重要的cookie字段包括：

- `acf_username` - 用户名
- `acf_uid` - 用户ID
- `acf_auth` - 认证token
- `acf_did` - 设备ID
- `acf_stk` - Session token

**组装格式：**
```
acf_username=值1; acf_uid=值2; acf_auth=值3; acf_did=值4; ...
```

**组装步骤：**
1. 在Storage面板中，找到上述每个cookie项
2. 复制其"值"列的内容
3. 按照格式组装成完整字符串
4. 每个cookie之间用 `; `（分号+空格）分隔

**📸 需要的截图：**
- `screenshot-17-firefox-cookie-value.png` - 单个cookie的值

![Cookie值](./images/douyu-cookie-tutorial/screenshot-17-firefox-cookie-value.png)
*图17：复制单个cookie的值*

---

## 🧩 方法三：浏览器插件（最简单）

### 推荐插件

**Chrome/Edge：EditThisCookie**
**Firefox/Chrome：Cookie-Editor**

### 使用EditThisCookie

1. 在Chrome Web Store搜索并安装 "EditThisCookie"
2. 登录斗鱼后，点击浏览器工具栏的饼干图标
3. 点击 **"Export"**（导出）按钮
4. 选择 **"Header String"** 格式
5. 直接复制生成的cookie字符串

**📸 需要的截图：**
- `screenshot-18-plugin-icon.png` - 插件图标位置
- `screenshot-19-plugin-export.png` - Export导出选项
- `screenshot-20-plugin-result.png` - 导出的cookie字符串

![插件图标](./images/douyu-cookie-tutorial/screenshot-18-plugin-icon.png)
*图18：点击工具栏的EditThisCookie图标*

![导出选项](./images/douyu-cookie-tutorial/screenshot-19-plugin-export.png)
*图19：选择"Export" → "Header String"*

![导出结果](./images/douyu-cookie-tutorial/screenshot-20-plugin-result.png)
*图20：复制生成的cookie字符串*

---

## ✅ 验证Cookie有效性

配置cookie后，使用biliup内置的"测试Cookie"功能：

1. 在cookie输入框下方，点击 **"🔍 测试Cookie"** 按钮
2. 等待几秒钟
3. 查看测试结果：
   - ✅ **Cookie有效** - 可以正常使用
   - ❌ **Cookie无效或已过期** - 需要重新获取
   - ❌ **Cookie缺少必需字段** - 需要复制完整的cookie

**测试成功示例：**
- ✅ Cookie有效

**测试失败示例：**
- ❌ Cookie无效或已过期
- ❌ Cookie缺少必需字段（acf_uid, acf_auth）

---

## 📊 Cookie字段说明

| 字段名 | 说明 | 是否必需 |
|-------|------|---------|
| `acf_username` | 斗鱼用户名 | ✅ 必需 |
| `acf_uid` | 用户唯一ID | ✅ 必需 |
| `acf_auth` | 认证token，用于验证登录状态 | ✅ 必需 |
| `acf_did` | 设备ID，标识访问设备 | ✅ 必需 |
| `acf_stk` | Session token | 推荐 |
| `acf_ltkid` | 长期密钥ID | 推荐 |

---

## 🔒 安全提示

⚠️ **Cookie是敏感信息**

Cookie包含您的登录凭证，相当于账号密码。请注意：

1. **不要分享给他人**
2. **妥善保管配置文件**
3. **定期更换**（建议每1-2个月）
4. **使用独立账号**（不要使用主力账号）
5. **注意操作环境**（不要在公共电脑上获取）

---

## 📸 截图清单

### 方法一：Chrome/Edge（14张）

| 编号 | 文件名 | 说明 |
|------|--------|------|
| 1 | `screenshot-01-douyu-homepage.png` | 斗鱼首页，突出"登录"按钮 |
| 2 | `screenshot-02-login-page.png` | 登录页面 |
| 3 | `screenshot-03-open-devtools.png` | 右键菜单"检查"选项 |
| 4 | `screenshot-04-devtools-opened.png` | 开发者工具已打开 |
| 5 | `screenshot-05-network-tab.png` | Network标签位置 |
| 6 | `screenshot-06-network-panel.png` | Network面板请求列表 |
| 7 | `screenshot-07-select-request.png` | 选中一个请求 |
| 8 | `screenshot-08-headers-tab.png` | Headers标签 |
| 9 | `screenshot-09-find-cookie.png` | 找到cookie字段 |
| 10 | `screenshot-10-copy-cookie.png` | 复制cookie操作 |
| 11 | `screenshot-11-biliup-ui.png` | biliup配置界面 |
| 12 | `screenshot-12-paste-cookie.png` | 粘贴cookie |
| 13 | `screenshot-13-test-success.png` | 测试成功提示 |
| 14 | `screenshot-14-save-config.png` | 保存配置 |

### 方法二：Firefox（3张）

| 编号 | 文件名 | 说明 |
|------|--------|------|
| 15 | `screenshot-15-firefox-storage.png` | Firefox存储标签 |
| 16 | `screenshot-16-firefox-cookies.png` | Cookie列表 |
| 17 | `screenshot-17-firefox-cookie-value.png` | 单个cookie值 |

### 方法三：浏览器插件（3张）

| 编号 | 文件名 | 说明 |
|------|--------|------|
| 18 | `screenshot-18-plugin-icon.png` | 插件图标 |
| 19 | `screenshot-19-plugin-export.png` | Export导出选项 |
| 20 | `screenshot-20-plugin-result.png` | 导出结果 |

---

## 📁 截图保存位置

请将所有截图保存到：
```
/Users/mostly_harmless/Desktop/biliup/docs/images/douyu-cookie-tutorial/
```

### 截图要求

- ✅ 格式：PNG
- ✅ 分辨率：至少1920x1080
- ✅ 标注：使用红框、箭头或高亮标注关键位置
- ✅ 脱敏：如有个人信息，请打码处理
- ✅ 清晰度：确保文字清晰可读

### 创建目录

```bash
mkdir -p /Users/mostly_harmless/Desktop/biliup/docs/images/douyu-cookie-tutorial
```

---

## 📚 相关资源

- [biliup项目主页](https://github.com/ForgQi/biliup-rs)
- [纯文字Cookie获取教程](./DOUYU_COOKIE_GUIDE.md)
- [斗鱼Cookie功能实现文档](../DOUYU_COOKIE_IMPLEMENTATION.md)

---

**更新日期：** 2026-10-07  
**版本：** v1.0（图文版）  
**适用于：** biliup-rs v1.2.11+

---

**注意：** 本文档需要提供截图才能完整。请按照上述清单提供20张截图，放置在指定目录后，图片链接会自动生效。
