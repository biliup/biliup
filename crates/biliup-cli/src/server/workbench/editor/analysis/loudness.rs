//! Uncut's D66/D74 algorithm: EBU R128, -40 dB / 300 ms silence, 100 Hz RMS envelope.
//! Adapted from apps/server/src/analysis/loudness/loudness.ts (MIT; licenses/uncut).
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Envelope {
    data: Vec<u8>,
    sum: f64,
    count: usize,
    samples: usize,
    carry: Option<u8>,
}
impl Envelope {
    fn sample(&mut self, sample: i16) {
        self.sum += (sample as f64 / 32768.).powi(2);
        self.count += 1;
        self.samples += 1;
        if self.count == 160 {
            self.flush();
        }
    }
    fn flush(&mut self) {
        if self.count > 0 {
            let db = 10. * (self.sum / self.count as f64 + 1e-12).log10();
            self.data
                .push(((db + 100.) / 0.5).round().clamp(0., 255.) as u8);
        }
        self.sum = 0.;
        self.count = 0;
    }
    fn push(&mut self, bytes: &[u8]) {
        let mut at = 0;
        if let (Some(lo), Some(hi)) = (self.carry, bytes.first()) {
            self.carry = None;
            self.sample(i16::from_le_bytes([lo, *hi]));
            at = 1;
        }
        while at + 1 < bytes.len() {
            self.sample(i16::from_le_bytes([bytes[at], bytes[at + 1]]));
            at += 2;
        }
        if at < bytes.len() {
            self.carry = Some(bytes[at]);
        }
    }
    fn finish(mut self) -> Value {
        self.flush();
        json!({"rate":100,"db0":-100,"step":0.5,"data":STANDARD.encode(self.data)})
    }
}
#[derive(Default)]
struct Log {
    open: Option<f64>,
    ranges: Vec<Value>,
    summary: bool,
    section: u8,
    integrated: Option<f64>,
    range: Option<f64>,
    peak: Option<f64>,
}
fn number(value: &str) -> Option<f64> {
    value
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}
fn rounded(value: f64) -> f64 {
    (value * 1000.).round() / 1000.
}
impl Log {
    fn line(&mut self, line: &str) {
        if let Some((_, tail)) = line.split_once("silence_start:") {
            self.open = number(tail).map(|v| v.max(0.));
            return;
        }
        if let Some((_, tail)) = line.split_once("silence_end:") {
            if let Some(end) = number(tail) {
                self.ranges.push(
                    json!({"start":rounded(self.open.take().unwrap_or(0.)),"end":rounded(end)}),
                );
            }
            return;
        }
        if line.contains("Summary:") {
            self.summary = true;
            return;
        }
        if !self.summary {
            return;
        }
        let line = line.trim();
        if line == "Integrated loudness:" {
            self.section = 1;
        } else if line == "Loudness range:" {
            self.section = 2;
        } else if line == "True peak:" {
            self.section = 3;
        } else if line.ends_with(':') {
            self.section = 0;
        } else if self.section == 1 {
            if let Some(value) = line.strip_prefix("I:") {
                self.integrated = number(value);
            }
        } else if self.section == 2 {
            if let Some(value) = line.strip_prefix("LRA:") {
                self.range = number(value);
            }
        } else if self.section == 3
            && let Some(value) = line.strip_prefix("Peak:")
        {
            self.peak = number(value);
        }
    }
    fn finish(mut self, seconds: f64, envelope: Value) -> Value {
        if let Some(start) = self.open
            && seconds > start
        {
            self.ranges
                .push(json!({"start":rounded(start),"end":rounded(seconds)}));
        }
        json!({"hasAudio":true,"seconds":rounded(seconds),"integratedLufs":self.integrated,"loudnessRangeLu":self.range,"truePeakDbtp":self.peak,
            "silence":{"thresholdDb":-40,"minSeconds":0.3,"ranges":self.ranges},"envelope":envelope})
    }
}

/// Caller owns the encoding permit and recording lifecycle lock. No whole PCM file is buffered.
pub async fn analyze(
    path: &Path,
    seconds: f64,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    if cancel.is_cancelled() {
        return Err("分析已取消".into());
    }
    let mut command = crate::tools::ffmpeg_command();
    let duration = format!("{seconds:.3}");
    command.args(["-hide_banner","-nostdin","-nostats","-progress","pipe:2","-i"]).arg(path)
        .args(["-filter_complex","[0:a:0]asplit=2[m][e];[m]silencedetect=noise=-40dB:d=0.3,ebur128=peak=true:framelog=quiet[mo];[e]aresample=16000,aformat=sample_fmts=s16:channel_layouts=mono[eo]",
            "-map","[mo]","-t",&duration,"-f","null","-","-map","[eo]","-t",&duration,"-f","s16le","-acodec","pcm_s16le","pipe:1"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动音频分析：{e}"))?;
    let mut stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let scan = async {
        let mut envelope = Envelope::default();
        let mut bytes = [0; 32768];
        loop {
            let size = stdout.read(&mut bytes).await.map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            envelope.push(&bytes[..size]);
            if envelope.data.len() as f64 > seconds * 100. + 1000. {
                return Err("音频时长与登记信息不一致".into());
            }
        }
        Ok::<_, String>(envelope)
    };
    let drain = async {
        let mut lines = BufReader::new(stderr).lines();
        let mut log = Log::default();
        let mut tail = std::collections::VecDeque::new();
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if let Some(time) = line.strip_prefix("out_time_us=").and_then(number) {
                progress((time / 1_000_000. / seconds).clamp(0., 0.99));
            } else {
                log.line(&line);
            }
            tail.push_back(line);
            if tail.len() > 30 {
                tail.pop_front();
            }
        }
        Ok::<_, String>((log, tail.into_iter().collect::<Vec<_>>().join("\n")))
    };
    let work = async {
        let (envelope, (log, tail)) = tokio::try_join!(scan, drain)?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((envelope, log, tail, status))
    };
    let result = tokio::select! {
        biased;
        _=cancel.cancelled()=>Err("分析已取消".into()),
        result=tokio::time::timeout(Duration::from_secs(30*60),work)=>result.map_err(|_|"音频分析超时".to_string()).and_then(|result|result),
    };
    let (envelope, log, tail, status) = match result {
        Ok(result) => result,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    if !status.success() {
        if tail.contains("matches no streams")
            || tail.contains("does not contain any stream")
            || tail.contains("Cannot find a matching stream")
        {
            return Ok(json!({"hasAudio":false}));
        }
        tracing::warn!(%tail,"音频分析失败");
        return Err("无法解码音频，请检查素材是否完整".into());
    }
    let seconds = envelope.samples as f64 / 16000.;
    Ok(log.finish(seconds, envelope.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_quantization_and_odd_chunks_match() {
        let samples: Vec<i16> = [vec![0; 160], vec![16384; 160], vec![i16::MIN; 1]].concat();
        let mut envelope = Envelope::default();
        for sample in samples {
            for byte in sample.to_le_bytes() {
                envelope.push(&[byte]);
            }
        }
        let value = envelope.finish();
        assert_eq!(
            STANDARD.decode(value["data"].as_str().unwrap()).unwrap(),
            vec![0, 188, 200]
        );
        let mut log = Log::default();
        for line in [
            "silence_start: -0.02",
            "silence_end: 0.5",
            "Summary:",
            "Integrated loudness:",
            "I: -23.1 LUFS",
            "Loudness range:",
            "LRA: 2.3 LU",
            "True peak:",
            "Peak: -2.0 dBFS",
            "Sample peak:",
            "Peak: -3.0 dBFS",
            "silence_start: 0.8",
        ] {
            log.line(line);
        }
        let result = log.finish(1., value);
        assert_eq!(result["integratedLufs"], -23.1);
        assert_eq!(result["truePeakDbtp"], -2.);
        assert_eq!(
            result["silence"]["ranges"],
            json!([{"start":0.,"end":0.5},{"start":0.8,"end":1.}])
        );
    }
}
