//! Portable Uncut videoAnalysis.ts (MIT), without macOS hardware-specific decoding.
mod log;
mod phase;
mod windows;
use log::VideoLog;
use phase::{Motion, PhaseCorrelator};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;
use windows::{Sample, round};
const RAW_W: usize = 128;
const RAW_H: usize = 72;
const FRAME_BYTES: usize = RAW_W * RAW_H;
const MAX_FRAMES: usize = 4_000_000;
fn check(cancel: &CancellationToken) -> Result<(), String> {
    if cancel.is_cancelled() {
        Err("视频分析已取消".into())
    } else {
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Picture {
    motion: Option<Motion>,
    brightness: f64,
    dark: f64,
    bright: f64,
}
fn picture(frame: &[u8], phase: &mut PhaseCorrelator) -> Picture {
    let mut sum = 0u64;
    let mut dark = 0;
    let mut bright = 0;
    for &p in frame {
        sum += p as u64;
        if p <= 16 {
            dark += 1;
        } else if p >= 245 {
            bright += 1;
        }
    }
    Picture {
        motion: phase.push(frame, RAW_W, RAW_H),
        brightness: sum as f64 / FRAME_BYTES as f64 / 255.,
        dark: dark as f64 / FRAME_BYTES as f64,
        bright: bright as f64 / FRAME_BYTES as f64,
    }
}
fn nearest(times: &[f64], time: f64) -> usize {
    let at = times.partition_point(|&t| t < time).min(times.len() - 1);
    if at > 0 && (times[at - 1] - time).abs() <= (times[at] - time).abs() {
        at - 1
    } else {
        at
    }
}
fn result(
    pictures: Vec<Picture>,
    mut log: VideoLog,
    seconds: f64,
    cancel: &CancellationToken,
) -> Result<Value, String> {
    let count = pictures.len();
    if count == 0 {
        return Ok(json!({"hasVideo":false}));
    }
    let step = if seconds > 0. {
        seconds
    } else {
        log.duration.unwrap_or(count as f64 / 30.)
    } / count as f64;
    let mut times = Vec::with_capacity(count);
    for i in 0..count {
        times.push(
            log.times
                .get(i)
                .copied()
                .flatten()
                .unwrap_or_else(|| if i == 0 { 0. } else { times[i - 1] + step }),
        );
    }
    // Raw timestamps must be monotonic for phase/cut matching; never manufacture a clean result.
    if times.windows(2).any(|v| v[1] < v[0]) {
        return Err("视频时间戳倒退，无法可靠分析镜头".into());
    }
    let last = times[count - 1];
    let frame_duration = if count > 1 {
        (last - times[0]) / (count - 1) as f64
    } else {
        step
    };
    let end = (last + frame_duration).max(log.out_time);
    if !end.is_finite() || end <= 0. || end > 7. * 24. * 3600. {
        return Err("视频分析时长超出范围".into());
    }
    log.close_freeze(end);
    let mut gaps: Vec<_> = times
        .windows(2)
        .map(|v| v[1] - v[0])
        .filter(|v| *v > 0.)
        .collect();
    gaps.sort_by(f64::total_cmp);
    let typical = gaps.get(gaps.len() / 2).copied().unwrap_or(frame_duration);
    drop(gaps);
    let mut dropped = vec![false; count];
    let mut drops = vec![];
    for i in 1..count {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let gap = times[i] - times[i - 1];
        if typical > 0. && gap > 2.5 * typical {
            dropped[i] = true;
            drops.push(json!({"time":round(times[i],4),"missing":round(gap-typical,4)}));
        }
    }
    let cuts: Vec<_> = log
        .cuts
        .iter()
        .copied()
        .filter(|&(time, score)| {
            let at = nearest(&times, time);
            !dropped[at] && (score >= 40. || pictures[at].motion.is_none_or(|m| m.peak < 0.4))
        })
        .collect();
    let mut is_cut = vec![false; count];
    for &(time, _) in &cuts {
        is_cut[nearest(&times, time)] = true;
    }
    let samples: Vec<_> = pictures
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let motion = p
                .motion
                .filter(|m| m.peak >= 0.04 && !is_cut[i] && !dropped[i]);
            Sample {
                time: times[i],
                dx: motion.map(|m| m.dx),
                dy: motion.map(|m| m.dy),
                blur: log.blurs.get(i).copied().flatten(),
                brightness: p.brightness,
                dark: p.dark,
                bright: p.bright,
            }
        })
        .collect();
    let frozen: Vec<_> = log
        .frozen
        .iter()
        .map(|&(start, finish)| (round(start, 4), round(finish.min(end), 4)))
        .filter(|&(start, end)| end > start)
        .collect();
    let width = log
        .display
        .filter(|(w, h)| *w > 0. && *h > 0.)
        .map(|(w, h)| (RAW_H as f64 * w / h).max(RAW_W as f64))
        .unwrap_or(RAW_W as f64);
    let windows = windows::windows(&samples, &frozen, width, end, cancel)?;
    Ok(
        json!({"hasVideo":true,"seconds":round(end,3),"frames":count,"fps":round(count as f64/(end-times[0]).max(1e-6),3),"cutThreshold":10,"cuts":cuts.into_iter().map(|(time,score)|json!({"time":round(time,4),"score":round(score,2)})).collect::<Vec<_>>(),"drops":drops,"frozen":frozen.into_iter().map(|(start,end)|json!({"start":start,"end":end})).collect::<Vec<_>>(),"windows":windows}),
    )
}
pub fn valid(value: &Value) -> bool {
    if value["hasVideo"] == false {
        return true;
    }
    if value["hasVideo"] != true {
        return false;
    }
    let finite = |v: &Value| v.as_f64().is_some_and(f64::is_finite);
    let rows = |key: &str, fields: &[&str]| {
        value[key].as_array().is_some_and(|rows| {
            rows.len() <= MAX_FRAMES
                && rows
                    .iter()
                    .all(|row| fields.iter().all(|field| finite(&row[field])))
        })
    };
    finite(&value["seconds"])
        && value["frames"]
            .as_u64()
            .is_some_and(|n| n > 0 && n <= MAX_FRAMES as u64)
        && rows("cuts", &["time", "score"])
        && rows("drops", &["time", "missing"])
        && rows("frozen", &["start", "end"])
        && rows(
            "windows",
            &[
                "start",
                "end",
                "frames",
                "tracked",
                "brightness",
                "dark",
                "bright",
                "frozen",
            ],
        )
        && value["windows"].as_array().unwrap().iter().all(|w| {
            ["jitter", "motion", "blur"]
                .iter()
                .all(|key| w[*key].is_null() || finite(&w[*key]))
        })
}
pub async fn analyze(
    path: &Path,
    seconds: f64,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    check(cancel)?;
    let mut command = crate::tools::ffmpeg_command();
    command.args(["-hide_banner","-nostdin","-nostats","-progress","pipe:2","-i"]).arg(path).args(["-map","0:V:0","-an","-sn","-dn","-vf","scale=320:-2:flags=fast_bilinear,format=gray,scdet=threshold=10,freezedetect=n=-60dB:d=0.5,blurdetect,metadata=mode=print:key=lavfi.blur,scale=128:72:force_original_aspect_ratio=increase:flags=area,crop=128:72","-fps_mode","passthrough","-f","rawvideo","-pix_fmt","gray","pipe:1"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动视频分析：{e}"))?;
    let mut stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4);
    let worker_cancel = cancel.child_token();
    let worker_guard = worker_cancel.clone().drop_guard();
    let worker = tokio::task::spawn_blocking(move || {
        let mut phase = PhaseCorrelator::new();
        let mut out = vec![];
        while let Some(bytes) = rx.blocking_recv() {
            for frame in bytes.chunks_exact(FRAME_BYTES) {
                check(&worker_cancel)?;
                if out.len() >= MAX_FRAMES {
                    return Err("视频分析超过四百万帧，请拆分素材".into());
                }
                out.push(picture(frame, &mut phase));
            }
        }
        check(&worker_cancel)?;
        Ok::<_, String>(out)
    });
    let scan = async {
        let mut pending = vec![0; FRAME_BYTES * 8];
        let mut filled = 0;
        loop {
            let n = stdout
                .read(&mut pending[filled..])
                .await
                .map_err(|e| e.to_string())?;
            if n == 0 {
                if filled != 0 {
                    return Err("视频解码帧不完整".into());
                }
                break;
            }
            let end = filled + n;
            let complete = end / FRAME_BYTES * FRAME_BYTES;
            if complete > 0 {
                tx.send(pending[..complete].to_vec())
                    .await
                    .map_err(|_| "视频测量已停止".to_string())?;
            }
            pending.copy_within(complete..end, 0);
            filled = end - complete;
        }
        Ok::<_, String>(())
    };
    let drain = async {
        let mut reader = BufReader::new(stderr);
        let mut log = VideoLog::default();
        let mut tail = std::collections::VecDeque::new();
        loop {
            let mut bytes = vec![];
            let size = (&mut reader)
                .take(65537)
                .read_until(b'\n', &mut bytes)
                .await
                .map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            if size > 65536 {
                return Err("视频分析日志行过长".into());
            }
            let line = String::from_utf8_lossy(&bytes);
            log.line(line.trim_end())?;
            if line.starts_with("out_time_us=") {
                let total = if seconds > 0. {
                    seconds
                } else {
                    log.duration.unwrap_or(0.)
                };
                if total > 0. {
                    progress((log.out_time / total * 0.9).clamp(0., 0.9));
                }
            }
            tail.push_back(line.into_owned());
            while tail.len() > 20 {
                tail.pop_front();
            }
        }
        Ok::<_, String>((log, tail.into_iter().collect::<String>()))
    };
    let work = async {
        let (_, logs) = tokio::try_join!(scan, drain)?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((status, logs))
    };
    let started = Instant::now();
    let timeout = Duration::from_secs_f64((seconds * 4.).max(3600.));
    let decoded = tokio::select! {biased;_=cancel.cancelled()=>Err("视频分析已取消".into()),result=tokio::time::timeout(timeout,work)=>result.map_err(|_|"视频分析超时".to_string()).and_then(|v|v)};
    drop(tx);
    let (status, (log, tail)) = match decoded {
        Ok(v) => v,
        Err(e) => {
            drop(worker_guard);
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = worker.await;
            return Err(e);
        }
    };
    let pictures = worker.await.map_err(|e| e.to_string())??;
    drop(worker_guard);
    if !status.success() {
        if tail.contains("matches no streams")
            || tail.contains("does not contain any stream")
            || tail.contains("Failed to set value '0:V:0'")
        {
            return Ok(json!({"hasVideo":false}));
        }
        tracing::warn!(stderr=%tail,"视频分析解码失败");
        return Err("无法读取素材画面，请检查文件完整性及 FFmpeg 的 scdet、freezedetect、blurdetect 滤镜支持".into());
    }
    progress(0.92);
    check(cancel)?;
    let calculation_cancel = cancel.child_token();
    let guard = calculation_cancel.clone().drop_guard();
    let mut calculation =
        tokio::task::spawn_blocking(move || result(pictures, log, seconds, &calculation_cancel));
    let output = tokio::select! {biased;_=cancel.cancelled()=>Err("视频分析已取消".into()),v=tokio::time::timeout(timeout.saturating_sub(started.elapsed()),&mut calculation)=>v.map_err(|_|"视频分析汇总超时".to_string()).and_then(|v|v.map_err(|e|e.to_string())).and_then(|v|v)};
    drop(guard);
    if !calculation.is_finished() {
        let _ = calculation.await;
    }
    output
}
