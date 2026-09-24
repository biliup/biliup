# biliup 桌面版

Tauri 壳：在本进程内启动 biliup 服务（`biliup_cli::entry::serve`），监听 `127.0.0.1:19159`（被占用时换一个空闲端口），就绪后在窗口里打开 WebUI。没有 sidecar 子进程。

- 数据目录（`data/`、`ds_update.log`、`cookies.json` 和录像都在这里）：
  - Windows，程序不在系统盘（`%SystemDrive%`）上：就是安装目录，与旧版相同。
  - Windows，程序在系统盘上：首次启动弹窗让用户选——默认目录（Tauri 的 `app_data_dir()`，即 `%APPDATA%\com.biliup.desktop`）、安装目录或其他目录。选择保存在 `app_config_dir()` 下的 `desktop.json`（`{"data_dir": "..."}`），之后不再询问；删掉这个文件即可重新选择。
  - 其他平台：`app_data_dir()`，不询问（`desktop.json` 里写了 `data_dir` 则用它）。
  - 所选目录还没有 `data/` 时，会把旧版留下的 `data/` 复制过去，原处保留。旧版的位置依次找：安装目录、Windows 上旧 `bbup-app` 安装程序记录的安装目录（注册表）和 `%LOCALAPPDATA%\bbup-app`。
- `static/index.html` 只是启动页，显示「正在启动」或启动失败的原因。

构建（需要先在仓库根目录 `npm run build` 生成 `out/`，WebUI 会被编进程序）：

```bash
npm install && npm run build   # 仓库根目录
cd tauri-app && npm ci && npm run tauri build
```
