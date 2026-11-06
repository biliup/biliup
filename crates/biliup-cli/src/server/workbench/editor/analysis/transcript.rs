//! Portable transcription; adapted from Uncut plugins/whisper-transcribe/whisper.ts (MIT).
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;
fn rounded(v: f64) -> f64 {
    (v * 1000.).round() / 1000.
}
fn decode(text: &str) -> String {
    String::from_utf8_lossy(&text.chars().map(|c| c as u8).collect::<Vec<_>>())
        .trim()
        .to_owned()
}
fn punctuation_char(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
        || "，。！？、；：％（）【】《》〈〉「」『』〔〕〖〗〘〙〚〛“”‘’…—·・〜～".contains(c)
}
fn punctuation(text: &str) -> bool {
    !text.is_empty() && text.chars().all(punctuation_char)
}
fn unspaced(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff | 0x3400..=0x4dbf |
        0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af)
        || "々〆〇".contains(c)
}
fn annotation(text: &str) -> bool {
    let text = text.trim();
    [('[', ']'), ('(', ')'), ('*', '*')]
        .iter()
        .any(|(a, b)| text.starts_with(*a) && text.ends_with(*b))
        || (!text.is_empty() && text.chars().all(|c| c == '♪'))
}
struct Word {
    text: String,
    start: f64,
    end: Option<f64>,
    p: f64,
}
fn append_token(words: &mut Vec<Word>, text: String, start: f64, end: f64, p: f64) {
    let letters = !punctuation(&text);
    let trimmed = text.trim();
    if trimmed.is_empty() { return; }
    let opens = trimmed.chars().all(|c| "([{（【《〈「『〔〖〘〚“‘".contains(c));
    let separate = words.last().is_none_or(|previous| {
        if punctuation(&previous.text) { return false; }
        if !letters { return opens; }
        text.starts_with(char::is_whitespace)
            || trimmed.chars().find(|c| !punctuation_char(*c)).is_some_and(unspaced)
            || previous.text.chars().rev().find(|c| !punctuation_char(*c)).is_some_and(unspaced)
    });
    if separate {
        words.push(Word { text: trimmed.into(), start, end: letters.then_some(end), p: if letters { p } else { 1. } });
    } else if let Some(previous) = words.last_mut() {
        previous.text.push_str(trimmed);
        if letters {
            if previous.end.is_none() { previous.start = start; }
            previous.end = Some(end);
            previous.p = previous.p.min(p);
        }
    }
}
pub fn parse(bytes: &[u8], seconds: f64) -> Result<Value, String> {
    // whisper.cpp can split UTF-8 byte sequences across tokens. Preserve bytes until words join.
    // Normal JSON also accepts escaped Unicode. Only byte-split token output needs
    // the byte-preserving representation; never decode normal Unicode a second time.
    let (doc, byte_encoded): (Value, bool) = match serde_json::from_slice(bytes) {
        Ok(doc) => (doc, false),
        Err(_) => {
            let latin1: String = bytes.iter().copied()
                .filter(|b| !matches!(b,0..=8|11|12|14..=31)).map(char::from).collect();
            (serde_json::from_str(&latin1).map_err(|_| "无法读取 whisper-cli 的转写结果".to_string())?, true)
        }
    };
    let segments = doc["transcription"].as_array().ok_or("转写结果缺少段落")?;
    let mut words = vec![];
    let mut out = vec![];
    let mut last_start: f64 = 0.;
    let clamp = |v: f64| v.clamp(0., seconds);
    for segment in segments {
        let source_text = segment["text"].as_str().unwrap_or("");
        let text = if byte_encoded { decode(source_text) } else { source_text.trim().to_owned() };
        if text.is_empty() || annotation(&text) {
            continue;
        }
        let first = words.len();
        let mut raw: Vec<Word> = vec![];
        // A CJK token may be only part of a UTF-8 character. Join only those byte
        // fragments first; complete CJK tokens retain their original timing span.
        let mut pending = Vec::new();
        let mut token_start = 0.;
        let mut token_p: f64 = 1.;
        let mut boundary = false;
        for token in segment["tokens"].as_array().into_iter().flatten() {
            let Some(text) = token["text"].as_str() else {
                continue;
            };
            if text.is_empty() { continue; }
            if text.starts_with("[_")
                && text.ends_with(']')
                && text[2..text.len() - 1]
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            {
                continue;
            }
            let (Some(from), Some(to)) = (
                token["offsets"]["from"].as_f64(),
                token["offsets"]["to"].as_f64(),
            ) else {
                continue;
            };
            let p = token["p"].as_f64().unwrap_or(1.).clamp(0., 1.);
            if pending.is_empty() { token_start = from / 1000.; token_p = 1.; }
            if byte_encoded { pending.extend(text.chars().map(|c| c as u8)); }
            else { pending.extend_from_slice(text.as_bytes()); }
            token_p = token_p.min(p);
            match std::str::from_utf8(&pending) {
                Ok(decoded) => {
                    let mut text = decoded.to_owned();
                    if text.trim().is_empty() { boundary = true; }
                    else {
                        if boundary && !text.starts_with(char::is_whitespace) { text.insert(0, ' '); }
                        boundary = text.ends_with(char::is_whitespace);
                        append_token(&mut raw, text, token_start, to / 1000., token_p);
                    }
                    pending.clear();
                }
                Err(error) if error.error_len().is_none() => continue,
                Err(_) => return Err("转写 token 包含无效的 UTF-8 字节".into()),
            }
        }
        if !pending.is_empty() { return Err("转写 token 的 UTF-8 字符不完整".into()); }
        let mut skipping = false;
        for word in raw {
            let text = word.text.trim().to_owned();
            if text.is_empty() {
                continue;
            }
            let opens = text.starts_with(['[', '(', '*']);
            let numbered = text
                .strip_prefix('(')
                .unwrap_or(&text)
                .starts_with(|c: char| c.is_ascii_digit());
            if skipping || (opens && !numbered) {
                skipping = !text.ends_with([']', ')', '*']);
                continue;
            }
            if punctuation(&word.text) || text == "♪" {
                continue;
            }
            let start = last_start.max(clamp(word.start));
            let end = start.max(clamp(word.end.unwrap_or(word.start)));
            last_start = start;
            words.push(
                json!({"text":text,"start":rounded(start),"end":rounded(end),"p":rounded(word.p)}),
            );
        }
        if words.len() > first {
            out.push(json!({"start":rounded(clamp(segment["offsets"]["from"].as_f64().unwrap_or(0.)/1000.)),"end":rounded(clamp(segment["offsets"]["to"].as_f64().unwrap_or(seconds*1000.)/1000.)),"text":text,"words":[first,words.len()]}));
        }
    }
    Ok(json!({"language":doc["result"]["language"],"segments":out,"words":words}))
}
pub fn valid(result: &Value) -> bool {
    if result["hasAudio"] == false {
        return true;
    }
    let Some(words) = result["words"].as_array() else {
        return false;
    };
    let Some(segments) = result["segments"].as_array() else {
        return false;
    };
    result["hasAudio"] == true
        && words.iter().all(|w| {
            w["text"].is_string()
                && w["start"].is_number()
                && w["end"].is_number()
                && w["p"].is_number()
        })
        && segments.iter().all(|s| {
            s["text"].is_string()
                && s["start"].is_number()
                && s["end"].is_number()
                && s["words"].as_array().is_some_and(|range| {
                    range.len() == 2
                        && range[0].as_u64().is_some_and(|a| {
                            range[1]
                                .as_u64()
                                .is_some_and(|b| a <= b && b <= words.len() as u64)
                        })
                })
        })
}
async fn run(
    command: &mut tokio::process::Command,
    cancel: &CancellationToken,
    timeout: Duration,
    line: impl Fn(&str) + Sync,
) -> Result<(bool, String), String> {
    if cancel.is_cancelled() {
        return Err("转写已取消".into());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动转写工具：{e}"))?;
    let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();
    let work = async {
        let mut tail = std::collections::VecDeque::new();
        while let Some(text) = stderr.next_line().await.map_err(|e| e.to_string())? {
            line(&text);
            tail.push_back(text);
            if tail.len() > 20 {
                tail.pop_front();
            }
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((
            status.success(),
            tail.into_iter().collect::<Vec<_>>().join("\n"),
        ))
    };
    let result = tokio::select! {biased;_=cancel.cancelled()=>Err("转写已取消".into()),value=tokio::time::timeout(timeout,work)=>value.map_err(|_|"转写超时".to_string()).and_then(|v|v)};
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
pub async fn analyze(
    path: &Path,
    seconds: f64,
    model_id: &str,
    language: &str,
    models: &super::speech_models::Models,
    cancel: &CancellationToken,
    progress: impl Fn(f64) + Sync,
) -> Result<Value, String> {
    let (bin, model) = models.ready(model_id).await?;
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let wav = temp.path().join("audio.wav");
    let timeout = Duration::from_secs_f64((seconds * 4.).max(3600.));
    let mut extract = crate::tools::ffmpeg_command();
    extract
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
            "16000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
            "-y",
        ])
        .arg(&wav);
    let (ok, error) = run(&mut extract, cancel, timeout, |line| {
        if let Some(time) = line
            .strip_prefix("out_time_us=")
            .and_then(|v| v.parse::<f64>().ok())
        {
            progress((time / 1e6 / seconds * 0.05).clamp(0., 0.05));
        }
    })
    .await?;
    if !ok {
        if error.contains("matches no streams") || error.contains("does not contain any stream") {
            return Ok(json!({"hasAudio":false}));
        }
        tracing::warn!(stderr=%error,"转写音频解码失败");
        return Err("无法读取素材音频".into());
    }
    let output = temp.path().join("transcript");
    let mut whisper = tokio::process::Command::new(bin);
    #[cfg(windows)]
    whisper.creation_flags(0x08000000);
    whisper
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&wav)
        .args(["-ojf", "-of"])
        .arg(&output)
        .args([
            "-l",
            language,
            "-pp",
            "-np",
            "-t",
            &std::thread::available_parallelism()
                .map(|v| v.get())
                .unwrap_or(1)
                .clamp(1, 8)
                .to_string(),
        ]);
    let (ok, error) = run(&mut whisper, cancel, timeout, |line| {
        if let Some(value) = line
            .split("progress =")
            .nth(1)
            .and_then(|v| v.trim().strip_suffix('%'))
            .and_then(|v| v.trim().parse::<f64>().ok())
        {
            progress((0.05 + 0.94 * value / 100.).clamp(0.05, 0.99));
        }
    })
    .await?;
    if !ok {
        tracing::warn!(stderr=%error,"whisper-cli 转写失败");
        return Err("语音转写失败，请检查所选模型及 whisper-cli".into());
    }
    let file = output.with_extension("json");
    let size = tokio::fs::metadata(&file)
        .await
        .map_err(|e| e.to_string())?
        .len();
    if size > 128 * 1024 * 1024 {
        return Err("转写结果过大".into());
    }
    let mut result = parse(
        &tokio::fs::read(file).await.map_err(|e| e.to_string())?,
        seconds,
    )?;
    result["hasAudio"] = json!(true);
    result["seconds"] = json!(rounded(seconds));
    result["requestedLanguage"] = json!(language);
    result["model"] = json!(model_id);
    Ok(result)
}
