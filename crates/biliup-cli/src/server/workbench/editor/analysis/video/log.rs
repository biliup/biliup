//! Uncut video/videoLog.ts (MIT), including rotation/DAR, VFR and restarted frame counters.
use regex::Regex;
use std::sync::LazyLock;
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\bframe:\s*\d+\s+pts:\s*\S+\s+pts_time:\s*(-?[\d.]+(?:e[-+]?\d+)?)",
        r", (\d{2,5})x(\d{2,5})\b",
        r"DAR (\d+):(\d+)",
        r"(?i)rotation of (-?[\d.]+) degrees",
        r"(?i)\blavfi\.blur=\s*(-?[\d.]+(?:e[-+]?\d+)?|nan|-?inf)",
        r"^out_time_us=(\d+)",
        r"lavfi\.scd\.score:\s*([\d.]+),\s*lavfi\.scd\.time:\s*(-?[\d.]+)",
        r"lavfi\.freezedetect\.freeze_start:\s*(-?[\d.]+)",
        r"lavfi\.freezedetect\.freeze_end:\s*(-?[\d.]+)",
        r"^\s*Duration:\s*(\d+):(\d+):([\d.]+)",
    ]
    .into_iter()
    .map(|s| Regex::new(s).expect("video metadata regex"))
    .collect()
});
#[derive(Default)]
pub struct VideoLog {
    pub times: Vec<Option<f64>>,
    pub blurs: Vec<Option<f64>>,
    pub cuts: Vec<(f64, f64)>,
    pub frozen: Vec<(f64, f64)>,
    pub duration: Option<f64>,
    pub out_time: f64,
    pub display: Option<(f64, f64)>,
    freeze_start: Option<f64>,
    input: u8,
}
fn number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|v| v.is_finite())
}
impl VideoLog {
    pub fn line(&mut self, text: &str) -> Result<(), String> {
        let p = &*PATTERNS;
        if let Some(m) = p[0].captures(text) {
            if self.times.len() >= super::MAX_FRAMES {
                return Err("视频分析超过四百万帧，请拆分素材".into());
            }
            self.times.push(number(&m[1]));
            self.blurs.push(None);
            return Ok(());
        }
        if self.input != 3 {
            if text.trim().starts_with("Output #") || text.trim().starts_with("Stream mapping:") {
                self.input = 3;
            } else if text.trim_start().starts_with("Stream #0:") {
                if self.display.is_none()
                    && text.contains(": Video: ")
                    && !text.contains("(attached pic)")
                    && let Some(m) = p[1].captures(text)
                {
                    let w = number(&m[1]).unwrap_or(128.);
                    let h = number(&m[2]).unwrap_or(72.);
                    let width = p[2]
                        .captures(text)
                        .and_then(|dar| Some((h * number(&dar[1])? / number(&dar[2])?).round()))
                        .filter(|v| v.is_finite() && *v > 0.)
                        .unwrap_or(w);
                    self.display = Some((width, h));
                    self.input = 1;
                } else {
                    self.input = 2;
                }
            } else if self.input == 1
                && let Some(m) = p[3].captures(text)
                && let Some(angle) = number(&m[1])
                && ((angle / 90.).round() as i64) % 2 != 0
            {
                self.display = self.display.map(|(w, h)| (h, w));
            }
        }
        if let Some(m) = p[4].captures(text) {
            if let Some(blur) = self.blurs.last_mut() {
                *blur = number(&m[1]);
            }
            return Ok(());
        }
        if let Some(m) = p[5].captures(text) {
            self.out_time = number(&m[1]).unwrap_or(0.) / 1e6;
            return Ok(());
        }
        if let Some(m) = p[6].captures(text) {
            if self.cuts.len() >= super::MAX_FRAMES {
                return Err("视频切点过多".into());
            }
            if let (Some(time), Some(score)) = (number(&m[2]), number(&m[1])) {
                self.cuts.push((time, score));
            }
            return Ok(());
        }
        if let Some(m) = p[7].captures(text) {
            self.freeze_start = number(&m[1]).map(|v| v.max(0.));
            return Ok(());
        }
        if let Some(m) = p[8].captures(text) {
            if self.frozen.len() >= super::MAX_FRAMES {
                return Err("视频冻结区间过多".into());
            }
            if let Some(end) = number(&m[1]) {
                self.frozen.push((self.freeze_start.unwrap_or(0.), end));
            }
            self.freeze_start = None;
            return Ok(());
        }
        if self.duration.is_none()
            && let Some(m) = p[9].captures(text)
        {
            self.duration = Some(
                number(&m[1]).unwrap_or(0.) * 3600.
                    + number(&m[2]).unwrap_or(0.) * 60.
                    + number(&m[3]).unwrap_or(0.),
            );
        }
        Ok(())
    }
    pub fn close_freeze(&mut self, end: f64) {
        if let Some(start) = self.freeze_start.take()
            && end > start
        {
            self.frozen.push((start, end));
        }
    }
}
