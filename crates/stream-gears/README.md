# stream-gears

通过 PyO3 导出**上传** B 站与**下载** FLV、HLS 流的函数供Python调用,
支持时间或文件大小分段，同时解决拉取FLV流花屏的问题。

## Dev

1. Install the latest [Rust compiler](https://www.rust-lang.org/tools/install)
2. Install [maturin](https://maturin.rs/): `$ pip3 install maturin`

```shell
$python -m venv .env
$source .env/bin/activate
$maturin develop
```
