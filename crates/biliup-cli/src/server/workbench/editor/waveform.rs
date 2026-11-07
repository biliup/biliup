//! Port of Uncut's streamed mono peak meter. The timeline consumes the same 0..255 peaks.
use super::MediaSource;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{Arc, LazyLock, Mutex, Weak},
    time::{Duration, UNIX_EPOCH},
};
use tokio::{io::AsyncReadExt, sync::Semaphore};

static DECODES: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));
static IN_FLIGHT: LazyLock<Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const SAMPLE_RATE: usize = 8000;
struct PeakMeter {
    peaks: Vec<u8>,
    peak: u16,
    count: usize,
    samples: usize,
    per_peak: usize,
    carry: Option<u8>,
}
impl PeakMeter {
    fn new(seconds: f64) -> Self {
        // Retain 100 peaks/s normally; very long recordings use at most two million peaks.
        let per_peak = (SAMPLE_RATE as f64 / 100.)
            .max((seconds * SAMPLE_RATE as f64 / 2_000_000.).ceil())
            as usize;
        Self {
            peaks: Vec::new(),
            peak: 0,
            count: 0,
            samples: 0,
            per_peak,
            carry: None,
        }
    }
    fn sample(&mut self, sample: i16) {
        self.peak = self.peak.max(sample.unsigned_abs());
        self.count += 1;
        self.samples += 1;
        if self.count == self.per_peak {
            self.flush();
        }
    }
    fn flush(&mut self) {
        self.peaks
            .push((self.peak as f64 / 32768. * 255.).round().min(255.) as u8);
        self.peak = 0;
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
        if self.count > 0 {
            self.flush();
        }
        json!({"version":1,"rate":SAMPLE_RATE as f64/self.per_peak as f64,"seconds":self.samples as f64/SAMPLE_RATE as f64,"peaks":STANDARD.encode(self.peaks)})
    }
}

pub async fn generate(
    root: &Path,
    path: &Path,
    source: &MediaSource,
) -> Result<Option<Value>, String> {
    let Some(ms) = source.duration_ms() else {
        return Ok(None);
    };
    let info = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
    let key = format!(
        "{:x}",
        Sha256::digest(format!(
            "waveform-v1|{}|{}|{}|{ms}",
            path.display(),
            info.len(),
            info.modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|t| t.as_nanos())
                .unwrap_or(0)
        ))
    );
    let folder = root.join("editor-cache");
    let cache = folder.join(format!("{key}.json"));
    if let Ok(bytes) = tokio::fs::read(&cache).await
        && let Ok(value) = serde_json::from_slice(&bytes)
    {
        return Ok(Some(value));
    }
    let shared = {
        let mut jobs = IN_FLIGHT.lock().unwrap_or_else(|e| e.into_inner());
        jobs.retain(|_, weak| weak.strong_count() > 0);
        match jobs.get(&key).and_then(Weak::upgrade) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                jobs.insert(key.clone(), Arc::downgrade(&lock));
                lock
            }
        }
    };
    let _same = shared.lock().await;
    if let Ok(bytes) = tokio::fs::read(&cache).await
        && let Ok(value) = serde_json::from_slice(&bytes)
    {
        return Ok(Some(value));
    }
    let _slot = DECODES.acquire().await.map_err(|e| e.to_string())?;
    let mut command = crate::tools::ffmpeg_command();
    command
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-t",
            &format!("{:.3}", ms as f64 / 1000.),
            "-map",
            "0:a:0",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "8000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "s16le",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let scan = async {
        let mut meter = PeakMeter::new(ms as f64 / 1000.);
        let mut bytes = [0u8; 32768];
        loop {
            let size = tokio::time::timeout(Duration::from_secs(60), stdout.read(&mut bytes))
                .await
                .map_err(|_| "音频解码长时间没有进展")?
                .map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            meter.push(&bytes[..size]);
            if meter.peaks.len() > 2_000_100 {
                return Err("音频时长与登记信息不一致".into());
            }
        }
        Ok::<_, String>(meter.finish())
    };
    let drain = async {
        let mut text = Vec::new();
        let mut bytes = [0u8; 4096];
        loop {
            let size = stderr.read(&mut bytes).await.map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            let keep = size.min(8192usize.saturating_sub(text.len()));
            text.extend_from_slice(&bytes[..keep]);
        }
        Ok::<_, String>(String::from_utf8_lossy(&text).into_owned())
    };
    let result = tokio::try_join!(scan, drain);
    let (waveform, errors) = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    };
    let status = child.wait().await.map_err(|e| e.to_string())?;
    if errors.contains("matches no streams")
        || waveform["seconds"].as_f64() == Some(0.) && status.success()
    {
        return Ok(None);
    }
    // A damaged tail can still yield useful waveform data for its readable prefix.
    if !status.success() && waveform["seconds"].as_f64().unwrap_or(0.) < 1. {
        return Err("无法解码音频波形".into());
    }
    if tokio::fs::create_dir_all(&folder).await.is_ok()
        && let Ok(file) = tempfile::Builder::new()
            .prefix("peaks-")
            .tempfile_in(&folder)
    {
        let temp = file.into_temp_path();
        if tokio::fs::write(&temp, waveform.to_string()).await.is_ok() {
            let _ = temp.persist_noclobber(cache);
        }
    }
    Ok(Some(waveform))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_boundaries_and_negative_full_scale_keep_correct_peaks() {
        let mut meter = PeakMeter::new(1.);
        let mut samples = vec![0i16; 161];
        samples[0] = i16::MIN;
        samples[85] = 16384;
        let bytes: Vec<_> = samples.into_iter().flat_map(i16::to_le_bytes).collect();
        for byte in bytes {
            meter.push(&[byte]);
        }
        let value = meter.finish();
        assert_eq!(value["rate"], 100.);
        assert_eq!(value["seconds"], 161. / 8000.);
        assert_eq!(
            STANDARD.decode(value["peaks"].as_str().unwrap()).unwrap(),
            vec![255, 128, 0]
        );
    }
}
