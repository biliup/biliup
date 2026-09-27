//! 三节点联调（网络命名空间里的控制面 + 主机「本机」、备机、普通节点）的进程入口，只在测试构建里存在，
//! 平时的 `cargo test` 跳过它（`#[ignore]`）。联调脚本拿 `cargo test --no-run` 编出的测试二进制，
//! 带上环境变量只跑这一个测试：先装上 [`double`]，再按普通的 `biliup` 命令行起服务。配对里的投稿
//! 因此只写进本地的 JSONL 日志，不会有请求发往 B 站；发布构建里没有这个入口，也没有替身。
//!
//! - `BILIUP_H1_ARGS`：完整命令行（JSON 字符串数组，含程序名）；没有这个变量时什么都不做
//! - `BILIUP_H1_DOUBLE`：替身目录，日志写 `double.jsonl`，控制文件放 `control/`
//! - `BILIUP_H1_PREFIX`：替身给的稿件号前缀（区分哪台机器投的）
//! - `BILIUP_H1_RATE`：替身每秒推进的字节数，默认 0（瞬间传完）

use super::upload::double::{self, Double};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "三节点联调的进程入口，由 h1-evidence 的脚本带环境变量启动"]
async fn serve() {
    let Ok(args) = std::env::var("BILIUP_H1_ARGS") else {
        return;
    };
    let args: Vec<String> = serde_json::from_str(&args).expect("BILIUP_H1_ARGS 是 JSON 字符串数组");
    let dir = PathBuf::from(std::env::var("BILIUP_H1_DOUBLE").expect("缺 BILIUP_H1_DOUBLE"));
    let control = dir.join("control");
    std::fs::create_dir_all(&control).expect("建替身目录");
    let prefix = std::env::var("BILIUP_H1_PREFIX").unwrap_or_else(|_| "X".into());
    let rate = std::env::var("BILIUP_H1_RATE")
        .ok()
        .and_then(|rate| rate.parse().ok())
        .unwrap_or(0);
    double::install(Some(Arc::new(Double::new(
        dir.join("double.jsonl"),
        control,
        rate,
        &prefix,
    ))));
    let cli = crate::entry::parse(args).expect("命令行");
    crate::entry::run(cli).await.expect("biliup 退出时报错");
}
