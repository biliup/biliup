# 斗鱼Cookie获取详细教程

## 📌 为什么需要Cookie

**重要变更通知：** 自2026年9月21日起，斗鱼平台对API进行了重大调整：

- ❌ **未登录用户**：只能获取超清及以下画质，且可能每5分钟自动断流
- ✅ **登录用户**：可以获取原画（1080P60/2K）、蓝光4M等高码率流，不再断流

因此，为了录制高质量的斗鱼直播，您需要提供登录后的Cookie信息。

## 🔧 准备工作

在开始之前，请确保：

1. ✅ 拥有一个斗鱼账号（可在 https://www.douyu.com 注册）
2. ✅ 使用现代浏览器（Chrome、Firefox、Edge等）
3. ✅ 了解基本的浏览器操作
4. ✅ 确保账号已成功登录

## 📖 详细步骤

### 方法一：使用Chrome/Edge浏览器

#### 步骤1：登录斗鱼
1. 打开Chrome或Edge浏览器
2. 访问 https://www.douyu.com
3. 点击右上角"登录"按钮
4. 使用您的账号密码登录（或扫码登录）

#### 步骤2：打开开发者工具
1. 登录成功后，**不要关闭当前页面**
2. 按下 `F12` 键（或右键点击页面，选择"检查"）
3. 开发者工具面板会在浏览器底部或右侧打开

#### 步骤3：切换到Network（网络）面板
1. 在开发者工具顶部，找到 **"Network"** （网络）标签
2. 点击切换到Network面板
3. 如果Network面板是空的，按 `Ctrl+R`（Mac用户按 `Cmd+R`）刷新页面

#### 步骤4：查看请求
1. 在Network面板中，会看到很多网络请求
2. 点击任意一个请求（建议选择 `www.douyu.com` 开头的请求）
3. 在右侧详情面板中，找到 **"Headers"**（请求头）标签

#### 步骤5：复制Cookie
1. 在Headers标签中，向下滚动找到 **"Request Headers"**（请求头）部分
2. 找到 **"cookie:"** 字段（注意是小写）
3. 该字段的值是一长串文本，类似：
   ```
   acf_username=xxx; acf_uid=123456; acf_auth=xxxxxx; acf_did=10000000000000000000000000001511; ...
   ```
4. **完整复制整个cookie值**（从第一个字符到最后一个字符）

**提示：** 如何复制整个cookie：
- 点击cookie值，会出现选中状态
- 按 `Ctrl+C`（Mac用户按 `Cmd+C`）复制
- 或者右键点击cookie值，选择"Copy value"（复制值）

#### 步骤6：配置到biliup
1. 打开biliup的Web管理界面
2. 找到您要录制的主播配置
3. 展开"斗鱼"平台设置
4. 在 **"登录 Cookie（douyu_cookie）"** 输入框中
5. 粘贴刚才复制的完整cookie字符串
6. 点击保存

### 方法二：使用Firefox浏览器

#### 步骤1-2：登录并打开工具
1. 访问 https://www.douyu.com 并登录
2. 按 `F12` 打开开发者工具

#### 步骤3：查看Cookie
1. 点击 **"存储"**（Storage）标签
2. 展开左侧的 **"Cookie"** 项
3. 点击 `https://www.douyu.com`
4. 右侧会显示所有cookie项

#### 步骤4：手动组装Cookie字符串

Firefox需要手动组装cookie字符串。按照以下格式：

```
acf_username=值1; acf_uid=值2; acf_auth=值3; acf_did=值4; ...
```

**重要的cookie字段：**
- `acf_username` - 用户名
- `acf_uid` - 用户ID
- `acf_auth` - 认证token
- `acf_did` - 设备ID
- `acf_stk` - Session token
- `acf_ltkid` - Long-term key ID

**组装方法：**
1. 在Storage面板中，找到上述每个cookie项
2. 复制其"值"列的内容
3. 按照格式 `名称=值; 名称=值; ...` 组装成完整字符串
4. 每个cookie之间用 `; `（分号+空格）分隔

**示例：**
```
acf_username=myname; acf_uid=123456; acf_auth=abc123def456; acf_did=10000000000000000000000000001511
```

### 方法三：使用浏览器插件（最简单）

**推荐插件：** EditThisCookie 或 Cookie-Editor

#### 使用EditThisCookie（Chrome/Edge）
1. 在Chrome Web Store搜索并安装 "EditThisCookie"
2. 登录斗鱼后，点击浏览器工具栏的饼干图标
3. 点击 **"Export"**（导出）按钮
4. 选择导出为 **"Netscape HTTP Cookie File"** 格式
5. 将导出的内容转换为标准cookie字符串格式

#### 使用Cookie-Editor（Firefox/Chrome）
1. 安装 "Cookie-Editor" 插件
2. 登录斗鱼后，点击插件图标
3. 点击 **"Export"** 导出所有cookie
4. 选择 **"Header String"** 格式
5. 直接复制生成的cookie字符串

## 🔍 Cookie字段说明

斗鱼的cookie包含多个字段，以下是重要字段的说明：

| 字段名 | 说明 | 是否必需 |
|-------|------|---------|
| `acf_username` | 斗鱼用户名 | ✅ 必需 |
| `acf_uid` | 用户唯一ID | ✅ 必需 |
| `acf_auth` | 认证token，用于验证登录状态 | ✅ 必需 |
| `acf_did` | 设备ID，标识访问设备 | ✅ 必需 |
| `acf_stk` | Session token | 推荐 |
| `acf_ltkid` | 长期密钥ID | 推荐 |
| `dy_did` | 斗鱼设备ID | 可选 |
| `acf_biz` | 业务参数 | 可选 |

**完整cookie示例：**
```
acf_username=example_user; acf_uid=123456789; acf_auth=1234567890abcdef1234567890abcdef; acf_did=10000000000000000000000000001511; acf_stk=abc123; acf_ltkid=xyz789; dy_did=1234567890abcdef
```

## ❓ 常见问题

### Q1: Cookie会过期吗？
**A:** 是的，斗鱼的cookie通常有效期为1-3个月。当cookie过期后，您需要：
- 重新登录斗鱼账号
- 重新获取新的cookie
- 更新biliup配置

**过期症状：**
- 录制时提示"未开播"（但实际在直播）
- 只能获取低画质
- 频繁出现5分钟断流

### Q2: 如何判断cookie是否正确？
**A:** 正确的cookie应该：
- 包含 `acf_username`、`acf_uid`、`acf_auth`、`acf_did` 这几个必需字段
- 总长度通常在300-800字符之间
- 格式为 `key=value; key=value; ...`

**验证方法：**
1. 配置cookie后，尝试录制一个正在直播的房间
2. 在录制设置中选择"原画"或"蓝光4M"画质
3. 如果能成功开始录制且画质正确，说明cookie有效

### Q3: 多个主播可以共用一个cookie吗？
**A:** 可以！一个cookie对应一个斗鱼账号，可以用于录制任意房间。

**建议：**
- 在biliup的全局配置中设置cookie
- 这样所有斗鱼主播都会使用同一个cookie
- 除非某个主播需要特定账号，才在主播配置中覆写

### Q4: Cookie被泄露了怎么办？
**A:** 如果怀疑cookie泄露，立即采取以下措施：
1. 登录斗鱼账号，修改密码
2. 在账号安全设置中，退出所有其他设备
3. 重新登录并获取新cookie
4. 更新biliup配置

### Q5: 复制的cookie太长，无法完整复制？
**A:** Chrome/Firefox的开发者工具通常会显示完整的cookie，但如果确实遇到问题：

**解决方法1：** 使用JavaScript控制台
1. 在开发者工具中切换到 **"Console"**（控制台）标签
2. 输入并回车执行：
   ```javascript
   document.cookie
   ```
3. 会输出完整的cookie字符串，右键选择"Copy string contents"

**解决方法2：** 分段复制
1. 在Network面板中，选中cookie的前半部分复制
2. 继续选中后半部分复制
3. 将两段拼接到一起（确保中间没有换行）

### Q6: 为什么配置了cookie还是录不到原画？
**A:** 可能的原因：

1. **Cookie已过期**
   - 解决：重新登录并获取新cookie

2. **Cookie不完整**
   - 解决：确保复制了完整的cookie字符串，包含所有必需字段

3. **账号权限不足**
   - 某些特殊直播间可能有额外限制
   - 尝试在网页上用该账号能否观看原画

4. **房间确实不提供原画**
   - 部分小主播可能只提供较低画质
   - 尝试录制大主播的房间测试

5. **配置格式错误**
   - 确保cookie字符串没有多余的换行、空格
   - 不要包含 `Cookie:` 前缀，只要值部分

### Q7: 可以使用子账号的cookie吗？
**A:** 可以。只要是已登录的斗鱼账号，即使是新注册的账号也可以获取原画流。

**注意事项：**
- 确保账号已通过手机验证
- 某些高级功能可能需要实名认证
- 建议使用稳定的主账号

## 🔒 安全提示

### ⚠️ Cookie是敏感信息

Cookie包含您的登录凭证，相当于账号密码。请注意：

1. **不要分享给他人**
   - Cookie可以用来登录您的账号
   - 泄露后他人可以冒充您进行操作

2. **妥善保管配置文件**
   - biliup的配置文件包含cookie
   - 不要将配置文件上传到公开的代码仓库
   - 如果使用Git，将配置文件加入 `.gitignore`

3. **定期更换**
   - 建议每1-2个月更换一次cookie
   - 修改密码后，旧cookie会失效，需要获取新的

4. **使用独立账号**
   - 建议使用专门的账号用于录制
   - 不要使用主力游戏账号或绑定重要信息的账号

5. **注意操作环境**
   - 不要在公共电脑上获取cookie
   - 不要在不信任的网站输入cookie
   - biliup是开源项目，可以审查代码确保安全

### 🛡️ 如何保护Cookie安全

**在biliup中：**
- Cookie存储在本地配置文件中
- 不会上传到任何服务器
- 只用于向斗鱼API发送认证请求

**最佳实践：**
```bash
# 1. 设置配置文件权限（Linux/Mac）
chmod 600 ~/.config/biliup/config.toml

# 2. 使用环境变量（推荐高级用户）
export DOUYU_COOKIE="your_cookie_here"

# 3. 定期检查登录设备
# 在斗鱼账号安全设置中查看登录记录
```

## 📚 相关资源

- [biliup项目主页](https://github.com/ForgQi/biliup-rs)
- [斗鱼开放平台](https://open.douyu.com/)
- [斗鱼Cookie功能实现文档](../DOUYU_COOKIE_IMPLEMENTATION.md)

## 💡 提示与技巧

### 技巧1：快速测试Cookie是否有效

创建一个测试脚本：

```bash
# test_douyu_cookie.sh
curl -H "Cookie: YOUR_COOKIE_HERE" \
     "https://www.douyu.com/betard/YOUR_ROOM_ID" \
     | jq .
```

如果返回正常的JSON数据，说明cookie有效。

### 技巧2：批量配置多个主播

如果要录制多个斗鱼主播，建议：
1. 在全局配置中设置cookie（一次性配置）
2. 各个主播会自动继承全局cookie
3. 避免重复配置

### 技巧3：使用配置模板

创建一个cookie配置模板：

```toml
# config.toml 全局配置段
[global]
douyu_cookie = "acf_username=xxx; acf_uid=xxx; acf_auth=xxx; acf_did=xxx"

# 或者在streamers中为每个主播单独配置
[[streamers]]
url = "https://www.douyu.com/ROOM_ID"
douyu_cookie = "specific_cookie_for_this_streamer"
```

## 📞 获取帮助

如果您在获取或配置Cookie时遇到问题：

1. **查看日志**
   - biliup的日志通常会显示认证失败的原因
   - 关键词：`401 Unauthorized`、`cookie`、`auth`

2. **检查格式**
   - 确保没有多余的空格、换行
   - 确保分隔符是 `; `（分号+空格）
   - 确保没有包含 `Cookie:` 前缀

3. **社区求助**
   - GitHub Issues: 报告问题或查看已有解决方案
   - 讨论区：与其他用户交流经验

---

**更新日期：** 2026-10-07  
**版本：** v1.0  
**适用于：** biliup-rs v1.2.11+
