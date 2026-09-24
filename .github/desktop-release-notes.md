<!-- biliup-desktop-notes：desktop-publish.yml 在 Release 创建时把本段追加到正文末尾，其中的占位符由 workflow 替换 -->
## Windows 桌面版

- **推荐下载 `biliup_*_x64-setup.exe`**：双击安装，默认装在当前用户目录下，不需要管理员权限，也不需要另装 Python 或 FFmpeg。装在系统盘时，第一次打开会询问数据和录像放在哪里。`.msi` 适合批量部署。
- 安装包暂未签名，Windows 可能会拦一下：
  - 浏览器提示「不常下载」「可能有害」时，在下载列表里点「…」→「保留」（Edge 还要再点「显示详细信息」→「仍然保留」）。
  - 第一次运行出现蓝色的「Windows 已保护你的电脑」（SmartScreen）时，点「更多信息」→「仍要运行」。
- 安装包内置 FFmpeg `{{FFMPEG_VERSION}}`（BtbN/FFmpeg-Builds `{{FFMPEG_TAG}}` 的 win64 GPL 静态构建），在安装目录的 `ffmpeg\` 下，按 GPLv3 分发，许可文本见同目录 `LICENSE.txt`。Docker 镜像用的是同一 tag 的 linux64 / linuxarm64 构建。对应源码由上游托管：
  - FFmpeg 源码：<{{FFMPEG_SOURCE_URL}}>
  - BtbN 构建脚本与补丁：<{{FFMPEG_BUILD_SCRIPTS_URL}}>（`scripts.d/` 固定了各依赖库的版本与源码地址，`download.sh` 可一次拉取全部依赖源码）
