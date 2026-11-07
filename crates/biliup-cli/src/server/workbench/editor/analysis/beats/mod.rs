//! Port of Uncut analysis/beats.ts, beats/params.ts and fft.ts (MIT; licenses/uncut).
mod envelope;
mod tempo;
#[cfg(test)]
mod tests;
use envelope::{Envelope, SAMPLE_RATE};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tempo::{RATE, check, quantile, vertex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;
fn rounded(v: f64) -> f64 {
    (v * 1000.).round() / 1000.
}
fn time(frame: f64) -> f64 {
    frame / RATE + 0.003
}
fn onsets(env: &[f32], cancel: &CancellationToken) -> Result<Vec<Value>, String> {
    let n = env.len();
    let w = (0.03 * RATE).round().max(1.) as usize;
    let pre = (0.1 * RATE).round() as usize;
    let post = (0.07 * RATE).round() as usize;
    let gap = (0.03 * RATE).round() as usize;
    let mut peaks = vec![];
    for i in 0..n {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let value = env[i];
        if value <= 0. {
            continue;
        }
        if (i.saturating_sub(w)..=(n - 1).min(i + w))
            .all(|j| env[j] < value || (env[j] == value && j >= i))
        {
            peaks.push(i);
        }
    }
    if peaks.is_empty() {
        return Ok(vec![]);
    }
    let heights: Vec<_> = peaks.iter().map(|&i| env[i] as f64).collect();
    let q = quantile(&heights, 0.95);
    let scale = if q > 0. {
        q
    } else {
        heights.iter().copied().fold(0., f64::max)
    };
    let mut sums = vec![0.; n + 1];
    for i in 0..n {
        sums[i + 1] = sums[i] + env[i] as f64;
    }
    let mut out = vec![];
    let mut last = None;
    for i in peaks {
        let a = i.saturating_sub(pre);
        let b = n.min(i + post + 1);
        let mean = (sums[b] - sums[a]) / (b - a) as f64;
        if (env[i] as f64) < mean + 0.07 * scale || last.is_some_and(|last| i - last < gap) {
            continue;
        }
        last = Some(i);
        let at = i as f64
            + if i > 0 && i + 1 < n {
                vertex(env[i - 1] as f64, env[i] as f64, env[i + 1] as f64)
            } else {
                0.
            };
        out.push(json!({"t":rounded(time(at).max(0.)),"strength":rounded((env[i] as f64/scale).min(1.))}));
    }
    Ok(out)
}
fn levels(beats: &[f64], power: &[f32]) -> Vec<f64> {
    if beats.is_empty() || power.is_empty() {
        return vec![];
    }
    let mut gaps: Vec<_> = beats.windows(2).map(|b| b[1] - b[0]).collect();
    gaps.sort_by(f64::total_cmp);
    let period = if gaps.is_empty() {
        0.5
    } else {
        gaps[gaps.len() / 2]
    };
    let frame = |t: f64| ((t - 0.003) * RATE).round().clamp(0., power.len() as f64) as usize;
    let raw: Vec<_> = beats
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let a = frame(b);
            let z = frame(beats.get(i + 1).copied().unwrap_or(b + period))
                .max(a + 1)
                .min(power.len());
            if z > a {
                (power[a..z].iter().map(|v| *v as f64).sum::<f64>() / (z - a) as f64).sqrt()
            } else {
                0.
            }
        })
        .collect();
    let top = raw.iter().copied().fold(0., f64::max);
    raw.iter()
        .map(|v| if top > 0. { rounded(v / top) } else { 0. })
        .collect()
}
fn result(
    env: &[f32],
    power: &[f32],
    seconds: f64,
    cancel: &CancellationToken,
) -> Result<Value, String> {
    check(cancel)?;
    let onsets = onsets(env, cancel)?;
    let estimated = if onsets.len() >= 4 {
        tempo::estimate(env, cancel)?
    } else {
        None
    };
    let (period, clarity) = estimated.unwrap_or((0., 0.));
    if period == 0. || clarity < 0.15 {
        return Ok(
            json!({"hasAudio":true,"seconds":rounded(seconds),"bpm":null,"confidence":(clarity*100.).round()/100.,"beats":[],"onsets":onsets}),
        );
    }
    let beats: Vec<_> = tempo::track(env, period, cancel)?
        .into_iter()
        .map(|f| time(f).max(0.))
        .collect();
    let bpm = tempo::bpm(&beats, 60. * RATE / period);
    let beat_levels = levels(&beats, power);
    let mut output = json!({"hasAudio":true,"seconds":rounded(seconds),"bpm":(bpm*100.).round()/100.,"confidence":(clarity*100.).round()/100.,"beats":beats.iter().copied().map(rounded).collect::<Vec<_>>(),"onsets":onsets});
    if !beat_levels.is_empty() {
        output["beatLevels"] = json!(beat_levels);
    }
    Ok(output)
}
pub fn valid(value: &Value) -> bool {
    if value["hasAudio"] == false {
        return true;
    }
    let numbers = |key: &str| {
        value[key].as_array().is_some_and(|values| {
            values
                .iter()
                .all(|v| v.as_f64().is_some_and(f64::is_finite))
        })
    };
    value["hasAudio"] == true
        && value["seconds"]
            .as_f64()
            .is_some_and(|v| v.is_finite() && v >= 0.)
        && (value["bpm"].is_null()
            || value["bpm"]
                .as_f64()
                .is_some_and(|v| v.is_finite() && v > 0.))
        && value["confidence"]
            .as_f64()
            .is_some_and(|v| (0. ..=1.).contains(&v))
        && numbers("beats")
        && value["onsets"].as_array().is_some_and(|items| {
            items.iter().all(|v| {
                v["t"].as_f64().is_some_and(f64::is_finite)
                    && v["strength"]
                        .as_f64()
                        .is_some_and(|v| (0. ..=1.).contains(&v))
            })
        })
        && (value.get("beatLevels").is_none() || numbers("beatLevels"))
}
pub async fn analyze(
    path: &Path,
    seconds: f64,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    check(cancel)?;
    let mut command = crate::tools::ffmpeg_command();
    command
        .args([
            "-hide_banner",
            "-nostdin",
            "-nostats",
            "-progress",
            "pipe:2",
            "-i",
        ])
        .arg(path)
        .args([
            "-t",
            &format!("{seconds:.3}"),
            "-map",
            "0:a:0",
            "-vn",
            "-sn",
            "-dn",
            "-ac",
            "1",
            "-ar",
            "11025",
            "-f",
            "s16le",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动节拍分析：{e}"))?;
    let mut stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    // A bounded channel keeps DSP off Tokio's executor without collecting whole decoded audio.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4);
    let worker_cancel = cancel.clone();
    let worker = tokio::task::spawn_blocking(move || {
        let mut envelope = Envelope::new();
        while let Some(chunk) = rx.blocking_recv() {
            check(&worker_cancel)?;
            envelope.push(&chunk);
        }
        check(&worker_cancel)?;
        Ok::<_, String>(envelope.finish())
    });
    let scan = async {
        let mut bytes = vec![0; 32768];
        let mut total = 0usize;
        loop {
            let size = stdout.read(&mut bytes).await.map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            total += size;
            if total as f64 > seconds * SAMPLE_RATE as f64 * 2. + 32768. {
                return Err("音频时长与登记信息不一致".into());
            }
            tx.send(bytes[..size].to_vec())
                .await
                .map_err(|_| "节拍分析已停止".to_string())?;
        }
        Ok::<_, String>(())
    };
    let drain = async {
        let mut lines = BufReader::new(stderr).lines();
        let mut tail = std::collections::VecDeque::new();
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if let Some(value) = line
                .strip_prefix("out_time_us=")
                .and_then(|v| v.parse::<f64>().ok())
            {
                progress((value / 1e6 / seconds * 0.85).clamp(0., 0.85));
            }
            tail.push_back(line);
            if tail.len() > 30 {
                tail.pop_front();
            }
        }
        Ok::<_, String>(tail.into_iter().collect::<Vec<_>>().join("\n"))
    };
    let work = async {
        let (_, tail) = tokio::try_join!(scan, drain)?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((status, tail))
    };
    let started = std::time::Instant::now();
    let timeout = Duration::from_secs_f64((seconds * 4.).max(1800.));
    let decoded = tokio::select! {biased;_=cancel.cancelled()=>Err("分析已取消".into()),value=tokio::time::timeout(timeout,work)=>value.map_err(|_|"节拍分析超时".to_string()).and_then(|v|v)};
    drop(tx);
    let decoded = match decoded {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = worker.await;
            return Err(error);
        }
    };
    let (env, power, duration) = worker.await.map_err(|e| e.to_string())??;
    if !decoded.0.success() {
        if decoded.1.contains("matches no streams")
            || decoded.1.contains("does not contain any stream")
        {
            return Ok(json!({"hasAudio":false}));
        }
        tracing::warn!(stderr=%decoded.1,"节拍解码失败");
        return Err("无法读取素材音频，请检查文件是否完整".into());
    }
    progress(0.9);
    check(cancel)?;
    let calculation_cancel = cancel.child_token();
    let calculation_guard = calculation_cancel.clone().drop_guard();
    let mut calculation =
        tokio::task::spawn_blocking(move || result(&env, &power, duration, &calculation_cancel));
    let output = tokio::select! {biased;_=cancel.cancelled()=>Err("分析已取消".into()),value=tokio::time::timeout(timeout.saturating_sub(started.elapsed()),&mut calculation)=>value.map_err(|_|"节拍分析超时".to_string()).and_then(|v|v.map_err(|e|e.to_string())).and_then(|v|v)};
    drop(calculation_guard);
    if !calculation.is_finished() {
        let _ = calculation.await;
    }
    output
}
