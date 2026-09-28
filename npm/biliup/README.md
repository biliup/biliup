# @biliup/cli

安装运行：`npx @biliup/cli server`（或 `npm i -g @biliup/cli` 后 `biliup server`），打开 http://127.0.0.1:19159 ；命令行参数与原生二进制完全相同。

支持平台：Linux x64（glibc / musl）、Linux arm64（glibc）、Linux armv6+（glibc，gnueabi）、macOS x64 / arm64、Windows x64；npm 按 `os` / `cpu` / `libc` 只装对应的 `@biliup/<平台>` 子包。

与其它发行方式的关系：子包里的二进制就是 [GitHub Release](https://github.com/biliup/biliup/releases) 同版本的 `biliupR-v<版本>-<平台>` 原文件（发布时校验 SHA-256）。PyPI 上只发源码包，`pip install biliup` 需要本机 Rust 工具链现场编译。
