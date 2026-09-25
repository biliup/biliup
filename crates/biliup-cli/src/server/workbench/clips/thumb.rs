//! 取帧：场次时间 `t` 那一帧解成 JPEG，给发布封面和预览用。
//!
//! 按快速剪的规则取出覆盖 `t` 的那个关键帧区间，经管道喂给 ffmpeg，解码到 `t` 再输出一帧
//! （`-ss` 放在输入之后，逐帧精确）。要用 ffmpeg；没有 ffmpeg 时只能用直播间封面或上传图片。

use super::export::{ffmpeg_message, legacy_hevc_flv};
use super::plan::{self, PlanError};
use super::remux;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::index::Container;
use std::process::Stdio;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;

/// 同时最多几个取帧（每个起一个 ffmpeg）。
const SLOTS: usize = 2;
/// `t` 之后的关键帧最多等这么久（取正在写的尾巴时）。
const WAIT: Duration = Duration::from_secs(5);
/// 一次取帧最长多久。
const TIMEOUT: Duration = Duration::from_secs(30);
pub const MIN_WIDTH: u32 = 160;
pub const MAX_WIDTH: u32 = 1920;
/// 封面上限：B 站封面最大 2 MB 左右，1920 宽的 JPEG 远小于它。
pub const MAX_JPEG_BYTES: usize = 5 * 1024 * 1024;

static PERMITS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(SLOTS));

#[derive(Debug, thiserror::Error)]
pub enum ThumbError {
    /// ffmpeg 不可用。
    #[error("{0}")]
    NoFfmpeg(String),
    /// 这个位置取不了（没有画面、已清理等）。
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Failed(String),
}

/// 剪辑计划的报错说的是「范围」「剪」，取帧只有一个时间点。
fn for_frame(message: &str) -> String {
    message
        .replace("所选范围里 ", "")
        .replace(
            "（可能整段落在断流空档里）",
            "（落在断流空档里或超出了录像）",
        )
        .replace("所选范围里", "这个时间点")
        .replace("剪不了", "取不了画面")
        .replace("把范围挪开这一段后重试", "换个时间点再取")
        .replace("换个范围再剪", "换个时间点再取")
}

/// 取 `t_ms`（场次时间）那一帧，缩到最宽 `width` 像素（不放大），返回 JPEG。
pub async fn frame(
    pool: &ConnectionPool,
    session_id: i64,
    t_ms: i64,
    width: u32,
) -> Result<Vec<u8>, ThumbError> {
    let status = crate::tools::ffmpeg_status().await;
    if !status.available {
        return Err(ThumbError::NoFfmpeg(format!(
            "取帧要用 FFmpeg，但{}；可以改用直播间封面或上传图片",
            status.error.unwrap_or_else(|| "找不到 FFmpeg".into())
        )));
    }
    let _permit = PERMITS
        .acquire()
        .await
        .map_err(|e| ThumbError::Failed(e.to_string()))?;
    let t_ms = t_ms.max(0);
    let mut waited = false;
    let plan = plan::resolve(pool, session_id, t_ms, t_ms + 1, WAIT, || waited = true)
        .await
        .map_err(|e| match e {
            PlanError::Unavailable(_) if waited => {
                ThumbError::Unavailable("这个时间点还没录到画面，往前挑一个时间点再取".into())
            }
            PlanError::Unavailable(m) => ThumbError::Unavailable(for_frame(&m)),
            PlanError::Io(e) => ThumbError::Failed(format!("读录像出错：{e}")),
            PlanError::Db(e) => ThumbError::Failed(format!("读数据库出错：{e}")),
        })?;
    let at = plan.output_ms(t_ms);
    let width = width.clamp(MIN_WIDTH, MAX_WIDTH);
    let mut child = crate::tools::ffmpeg_command()
        .args(["-hide_banner", "-loglevel", "error", "-nostats"])
        .args(["-f", remux::ffmpeg_format(plan.container), "-i", "pipe:0"])
        .args(["-ss", &format!("{}.{:03}", at / 1000, at % 1000)])
        .args(["-map", "0:v:0", "-frames:v", "1"])
        .args(["-vf", &format!("scale='min({width},iw)':-2")])
        .args(["-c:v", "mjpeg", "-q:v", "3", "-f", "image2pipe", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| ThumbError::Failed(format!("启动 FFmpeg 失败：{e}")))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let feed = async {
        let mut ignore = |_: u64| {};
        // ffmpeg 取到一帧就退出，之后写管道会断，不算错
        let _ = remux::to_pipe(&plan, &mut stdin, &mut ignore).await;
        drop(stdin);
    };
    let read = async {
        let mut out = Vec::new();
        let _ = (&mut stdout)
            .take(MAX_JPEG_BYTES as u64 + 1)
            .read_to_end(&mut out)
            .await;
        out
    };
    let errors = async {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).into_owned()
    };
    let run = async {
        let ((), jpeg, stderr) = tokio::join!(feed, read, errors);
        let status = child.wait().await.ok();
        (jpeg, stderr, status)
    };
    let (jpeg, stderr, status) = tokio::time::timeout(TIMEOUT, run)
        .await
        .map_err(|_| ThumbError::Failed("取帧超时（FFmpeg 半分钟没出结果）".into()))?;
    if status.is_some_and(|s| s.success()) && jpeg.starts_with(&[0xff, 0xd8]) {
        if jpeg.len() > MAX_JPEG_BYTES {
            return Err(ThumbError::Failed("取出的图片太大".into()));
        }
        return Ok(jpeg);
    }
    if plan.container == Container::Flv && legacy_hevc_flv(&plan.pieces[0].path).await {
        return Err(ThumbError::Failed(
            "这段录像是国内平台在 FLV 里写的 HEVC（codec id 12），服务器上的 FFmpeg 读不了；\
             换一个支持它的 FFmpeg（配置里的 ffmpeg_path），或者改用直播间封面、上传图片"
                .into(),
        ));
    }
    Err(ThumbError::Failed(
        ffmpeg_message(&stderr, status).replace("转码", "取帧"),
    ))
}
