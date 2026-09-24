# biliup 桌面版

Tauri 壳：在本进程内启动 biliup 服务（`biliup_cli::entry::serve`），监听 `127.0.0.1:19159`（被占用时换一个空闲端口），就绪后在窗口里打开 WebUI。没有 sidecar 子进程。

- 数据目录：Tauri 的 `app_data_dir()`（Windows 上是 `%APPDATA%\com.tauri-app.app`），`data/`、`ds_update.log`、`cookies.json` 和录像都在这里。首次启动时，若安装目录下有旧版留下的 `data/` 而新目录没有，会复制过去，原处保留。
- `static/index.html` 只是启动页，显示「正在启动」或启动失败的原因。

构建（需要先在仓库根目录 `npm run build` 生成 `out/`，WebUI 会被编进程序）：

```bash
npm install && npm run build   # 仓库根目录
cd tauri-app && npm ci && npm run tauri build
```
