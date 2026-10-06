//! 送给模型看的关键帧缩图：每 5 分钟一张，外加弹幕最强的 16 个高峰中心各一张；60 秒内的只留一张，
//! 每场最多 64 张、每窗最多 8 张。采样点先吸附到最近的关键帧，取帧时 ffmpeg 只解一个 I 帧；
//! 宽 512、JPEG `-q:v 5`，存在场次目录的 `thumbs/` 下，续跑时直接用。
//!
//! 开关：`thumbnails = auto` 时只有连通性测试确认当前 chat 模型能看图才发；`on` 总是发
//! （服务拒收图片时那一窗去掉图再问一次）；`off` 不发。

use super::danmaku::Peak;
use super::files::SessionFiles;
use super::probe::ProbeReport;
use super::settings::{AutoClipConfig, Thumbnails};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::thumb::{self, ThumbError};
use crate::server::workbench::session_keyframes;

pub const WIDTH: u32 = 512;
pub const JPEG_QUALITY: u32 = 5;
const BASE_EVERY_MS: i64 = 5 * 60_000;
const PEAK_SHOTS: usize = 16;
const MIN_GAP_MS: i64 = 60_000;
pub const MAX_PER_SESSION: usize = 64;
pub const MAX_PER_WINDOW: usize = 8;
/// 找最近关键帧时前后各看多远
const SNAP_REACH_MS: i64 = 30_000;

/// 一张缩图的采样点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    /// 场次时间（已吸附到关键帧）
    pub t_ms: i64,
    /// 排序用：高峰按强度排在前面（数字越小越优先），定时采样排在后面
    pub rank: usize,
}

/// 缩图开不开，以及不开时记给用户看的原因（`off` 不记）。
pub fn enabled(config: &AutoClipConfig, probe: Option<&ProbeReport>) -> (bool, Option<String>) {
    match config.thumbnails() {
        Thumbnails::On => (true, None),
        Thumbnails::Off => (false, None),
        Thumbnails::Auto => {
            let vision = probe
                .filter(|report| report.matches_chat(config))
                .and_then(|report| report.vision.capable);
            match vision {
                Some(true) => (true, None),
                Some(false) => (
                    false,
                    Some("连通性测试显示当前 chat 模型不能看图，这次没有发截图".into()),
                ),
                None => (
                    false,
                    Some(
                        "还没对当前 chat 模型做连通性测试，不确定能不能看图，这次没有发截图；在设置页点「测试连接」后再生成就会带上"
                            .into(),
                    ),
                ),
            }
        }
    }
}

/// 采样点（吸附前）：高峰中心按强度取前 16 个，再每 5 分钟一个；60 秒内的只留先选中的；最多 64 个。
pub fn plan(end_ms: i64, peaks: &[Peak]) -> Vec<Shot> {
    let mut strongest: Vec<&Peak> = peaks.iter().collect();
    strongest.sort_by(|a, b| b.ratio.total_cmp(&a.ratio).then(b.count.cmp(&a.count)));
    let peak_shots = strongest
        .into_iter()
        .take(PEAK_SHOTS)
        .map(|peak| peak.center_ms());
    let base_shots = (0..)
        .map(|i| i * BASE_EVERY_MS + BASE_EVERY_MS / 2)
        .take_while(|t| *t < end_ms);
    let mut chosen: Vec<Shot> = Vec::new();
    for (rank, t_ms) in peak_shots.chain(base_shots).enumerate() {
        if chosen.len() >= MAX_PER_SESSION {
            break;
        }
        if chosen.iter().all(|s| (s.t_ms - t_ms).abs() >= MIN_GAP_MS) {
            chosen.push(Shot { t_ms, rank });
        }
    }
    chosen
}

/// 把采样点吸附到最近的关键帧；附近没有关键帧（断流、分段已删）的丢掉，吸附后重合的只留一个。
pub async fn snap(pool: &ConnectionPool, session_id: i64, shots: &[Shot]) -> Vec<Shot> {
    let mut snapped: Vec<Shot> = Vec::new();
    for shot in shots {
        let keyframes = session_keyframes(
            pool,
            session_id,
            shot.t_ms - SNAP_REACH_MS,
            shot.t_ms + SNAP_REACH_MS,
        )
        .await
        .unwrap_or_default();
        let Some(nearest) = keyframes
            .iter()
            .min_by_key(|k| (k.t_ms - shot.t_ms).abs())
            .map(|k| k.t_ms)
        else {
            continue;
        };
        if snapped.iter().all(|s| s.t_ms != nearest) {
            snapped.push(Shot {
                t_ms: nearest,
                rank: shot.rank,
            });
        }
    }
    snapped.sort_by_key(|s| s.t_ms);
    snapped
}

/// 取出（或从缓存读出）每张缩图，返回取到的与出错说明。没有 ffmpeg 时第一张就停。
pub async fn grab(
    pool: &ConnectionPool,
    session_id: i64,
    files: &SessionFiles,
    shots: &[Shot],
) -> (Vec<Shot>, Vec<String>) {
    let mut ok = Vec::new();
    let mut failures = Vec::new();
    for shot in shots {
        let path = files.thumb(shot.t_ms);
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            ok.push(*shot);
            continue;
        }
        match thumb::frame_with_quality(pool, session_id, shot.t_ms, WIDTH, JPEG_QUALITY).await {
            Ok(jpeg) => {
                let saved = async {
                    tokio::fs::create_dir_all(files.thumbs_dir()).await?;
                    tokio::fs::write(&path, &jpeg).await
                };
                match saved.await {
                    Ok(()) => ok.push(*shot),
                    Err(error) => failures.push(format!("存缩图出错：{error}")),
                }
            }
            Err(ThumbError::NoFfmpeg(message)) => {
                failures.push(message);
                break;
            }
            Err(error) => failures.push(error.to_string()),
        }
    }
    (ok, failures)
}

/// 一窗里要发的缩图：高峰的优先，最多 8 张，按时间排。
pub fn for_window(shots: &[Shot], from_ms: i64, to_ms: i64) -> Vec<Shot> {
    let mut inside: Vec<Shot> = shots
        .iter()
        .filter(|s| s.t_ms >= from_ms && s.t_ms < to_ms)
        .copied()
        .collect();
    inside.sort_by_key(|s| s.rank);
    inside.truncate(MAX_PER_WINDOW);
    inside.sort_by_key(|s| s.t_ms);
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(center_s: i64, ratio: f64) -> Peak {
        Peak {
            from_ms: center_s * 1000 - 10_000,
            to_ms: center_s * 1000 + 10_000,
            count: 50,
            baseline: 5.0,
            ratio,
            samples: Vec::new(),
        }
    }

    #[test]
    fn peaks_come_first_and_close_shots_are_merged() {
        // 30 分钟：定时采样在 2:30、7:30 … 27:30；高峰在 2:50（离 2:30 不到 60 秒）和 12:00
        let shots = plan(30 * 60_000, &[peak(170, 3.0), peak(720, 8.0)]);
        let times: Vec<i64> = shots.iter().map(|s| s.t_ms / 1000).collect();
        assert_eq!(times, vec![720, 170, 450, 1050, 1350, 1650]);
        assert_eq!(shots[0].rank, 0, "最强的高峰排第一");
    }

    #[test]
    fn a_long_session_is_capped() {
        let peaks: Vec<Peak> = (0..40).map(|i| peak(100 + i * 400, i as f64)).collect();
        let shots = plan(12 * 3_600_000, &peaks);
        assert_eq!(shots.len(), MAX_PER_SESSION);
        assert_eq!(
            shots.iter().filter(|s| s.rank < PEAK_SHOTS).count(),
            PEAK_SHOTS,
            "高峰只取最强的 16 个"
        );
        let window = for_window(&shots, 0, 3_600_000);
        assert!(window.len() <= MAX_PER_WINDOW);
        assert!(window.windows(2).all(|w| w[0].t_ms < w[1].t_ms));
    }
}
