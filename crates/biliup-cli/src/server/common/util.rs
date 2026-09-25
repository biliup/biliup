use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::infrastructure::models::hook_step::HookStep;
use crate::server::workbench::retention::{self, Disposal, Retention};
use chrono::{Duration, Local};
use error_stack::{ResultExt, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};
use url::Url;
use regex::Regex;
use std::sync::OnceLock;

/// 录制器配置结构体
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Recorder {
    ///  filename_prefix   文件名前缀模板
    pub filename_prefix: Option<String>,
    /// 直播间信息
    pub streamer_info: StreamerInfo,
}

impl Recorder {
    pub fn new(filename_prefix: Option<String>, streamer_info: StreamerInfo) -> Self {
        Self {
            filename_prefix,
            streamer_info,
        }
    }

    /// 生成原始模板（包含时间格式占位符），不清洗非法字符
    fn raw_template(&self) -> String {
        if let Some(prefix) = &self.filename_prefix {
            self.template_with(prefix)
        } else {
            format!("{}%Y-%m-%dT%H_%M_%S", self.streamer_info.name)
        }
    }

    /// 生成文件名模板（包含时间格式占位符），并清洗非法字符
    pub fn filename_template(&self) -> String {
        sanitize_filename(&self.raw_template())
    }

    fn template_with(&self, template: &str) -> String {
        template
            .replace("{streamer}", &self.streamer_info.name)
            .replace("{title}", &self.streamer_info.title)
            .replace("{url}", &self.streamer_info.url)
    }

    /// 生成“基名”（不带扩展名），时间冲突时按秒+1继续尝试，直到唯一
    pub fn generate_filename(&self, suffix: &str) -> String {
        let template = self.filename_template();
        let mut t = Local::now();
        let mut last = String::new();
        // Path::with_extension 会截掉最后一个点之后的部分：`Mr.Beast` + flv → `Mr.flv`。
        // 冲突检测一旦永远命中已有文件，旧实现的 `loop` 会占死 tokio 线程且无法 stop。
        for attempt in 0..10_000u32 {
            let formatted = t.format(&template).to_string();
            let candidate = if attempt == 0 || formatted != last {
                formatted.clone()
            } else {
                format!("{formatted}_{attempt}")
            };
            if !path_with_suffix(&candidate, suffix).exists() {
                return candidate;
            }
            last = formatted;
            t += Duration::seconds(1);
        }
        format!("{}_{}", t.format(&template), std::process::id())
    }

    /// 生成“基名”（不带扩展名）
    pub fn format_filename(&self) -> String {
        let template = self.filename_template();
        self.streamer_info
            .date
            .with_timezone(&Local)
            .format(&template)
            .to_string()
    }

    /// 生成投稿标题，不清洗文件名非法字符
    pub fn format_title(&self) -> String {
        self.streamer_info
            .date
            .with_timezone(&Local)
            .format(&self.raw_template())
            .to_string()
    }

    pub fn format(&self, template: &str) -> String {
        self.streamer_info
            .date
            .with_timezone(&Local)
            .format(&self.template_with(template))
            .to_string()
    }

    /// 直接生成带扩展名的完整路径（当前目录下）
    pub fn generate_path(&self, suffix: &str) -> PathBuf {
        path_with_suffix(&self.generate_filename(suffix), suffix)
    }
}

/// 拼接基名与后缀。不能用 `Path::with_extension`：基名含 `.` 时会截断。
pub(crate) fn path_with_suffix(base: &str, suffix: &str) -> PathBuf {
    let suffix = suffix.trim_start_matches('.');
    PathBuf::from(format!("{base}.{suffix}"))
}

/// 非法字符清洗（最小可用实现）
/// - 替换常见非法字符为 '_'；去掉末尾空格与点（Windows 兼容）
/// - 保留 '%'，以便 strftime 能正常工作
fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if c.is_control() => out.push('_'),
            _ => out.push(ch),
        }
    }
    let out = out.trim_end_matches([' ', '.']).to_string();
    if out.is_empty() { "_".to_string() } else { out }
}

/// 从 `Command` 的 Debug 输出里抠掉 Cookie / OAuth / 密码，避免写进 ds_update.log。
pub fn redact_process_debug(cmd: &impl std::fmt::Debug) -> String {
    redact_secrets(&format!("{cmd:?}"))
}

/// 脱敏命令行或日志文本中的登录态。
///
/// `Command` 的 Debug 输出把每个 argv 用双引号包起来，所以头部值一律吃到下一个引号：
/// `"Cookie: SESSDATA=..; bili_jct=..\r\n"`、`"Authorization=OAuth <token>"` 整段变成 `[redacted]`。
pub fn redact_secrets(text: &str) -> String {
    let out = header_secret_re().replace_all(text, "$1=[redacted]");
    let out = flag_secret_re().replace_all(&out, "$1\" \"[redacted]\"");
    let out = assign_secret_re().replace_all(&out, "$1=[redacted]");
    oauth_secret_re()
        .replace_all(&out, "$1 [redacted]")
        .into_owned()
}

/// `Cookie: ...` / `Authorization=...` 头部，值吃到引号或行尾。
fn header_secret_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)\b(cookie|authorization)\s*[:=]\s*[^"\n]*"#).expect("header secret regex")
    })
}

/// `"--niconico-password" "hunter2"` 这类把密钥放在下一个 argv 的开关。
fn flag_secret_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)(--?[\w-]*(?:password|passwd|token)[\w-]*)"\s+"[^"]*""#)
            .expect("flag secret regex")
    })
}

/// 裸露在文本里的 `key=value` 登录态。不含 `sid`：斗鱼直链的 `&sid=` 不是密钥。
fn assign_secret_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(SESSDATA|bili_jct|DedeUserID(?:__ckMd5)?|sessionid|auth-token|ttwid|__ac_nonce|passwd|password)\s*=\s*[^;\s,"]+"#,
        )
        .expect("assign secret regex")
    })
}

fn oauth_secret_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)\b(oauth)\s+[^\s"]+"#).expect("oauth secret regex"))
}

/// 生成弹幕文件名模板（包含时间格式占位符），并清洗非法字符
pub fn danmaku_filename_template(filename_prefix: Option<&str>, name: &str) -> String {
    let template = filename_prefix
        .map(|prefix| prefix.replace("{streamer}", name))
        .unwrap_or_else(|| format!("{}%Y-%m-%dT%H_%M_%S", name));
    sanitize_filename(&template)
}

/// 从 URL 中提取媒体扩展名（小写），例如 "flv", "mp4" 等。
/// 先尝试解析 URL 的 path 的扩展名；如果没有，再查 query 中常见的参数（format/type/ext）。
/// 返回 None 表示无法判断。
pub fn media_ext_from_url(input: &str) -> Option<String> {
    // 统一的扩展名清洗：去空白/前导点，切掉 MIME/分隔符，并转小写
    fn clean_ext(val: &str) -> Option<String> {
        let token = val
            .trim()
            .trim_start_matches('.') // .mp4 -> mp4
            .split(['/', ';', ',', '?', '&', '#']) // video/mp4;codecs=...
            .next()
            .map(str::trim)
            .unwrap_or("");
        if token.is_empty() {
            None
        } else {
            Some(token.to_ascii_lowercase())
        }
    }

    // 1) 先尝试按 URL 解析
    if let Ok(url) = Url::parse(input) {
        // a) 从最后一个 path segment 取扩展名
        if let Some(seg) = url.path_segments().and_then(|mut s| s.next_back())
            && let Some((_, ext)) = seg.rsplit_once('.')
            && let Some(ext) = clean_ext(ext)
        {
            return Some(ext);
        }

        // b) 常见 query 参数中找一次（不重复多轮扫描），忽略大小写
        let keys = ["format", "type", "ext", "filetype", "fmt"];
        if let Some(ext) = url.query_pairs().find_map(|(k, v)| {
            if keys.iter().any(|t| k.as_ref().eq_ignore_ascii_case(t)) {
                clean_ext(&v)
            } else {
                None
            }
        }) {
            return Some(ext);
        }

        return None;
    }

    // 2) 不是完整 URL 的兜底：纯字符串/相对地址
    let before_q = input.split('?').next().unwrap_or(input);
    if let Some((_, ext)) = before_q.rsplit_once('.') {
        return clean_ext(ext);
    }

    None
}

/// 解析 `segment_time`，语法与 ffmpeg 的时长参数一致（ffmpeg 下载器把原字符串直接交给 `-to`）：
/// `[HH:]MM:SS[.小数]` 或纯秒数 `S[.小数]`，例如 `"01:00:00"`、`"30:00"`（30 分钟）、`"3600"`。
/// 带冒号时分、秒必须小于 60。无法解析时返回 `None`。
pub fn parse_segment_time(raw: &str) -> Option<std::time::Duration> {
    let parts: Vec<&str> = raw.trim().split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [seconds] => ("0", "0", *seconds),
        [minutes, seconds] => ("0", *minutes, *seconds),
        [hours, minutes, seconds] => (*hours, *minutes, *seconds),
        _ => return None,
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let seconds_ok = seconds
        .split_once('.')
        .map_or(digits(seconds), |(whole, frac)| {
            digits(whole) && (frac.is_empty() || digits(frac))
        });
    if !digits(hours) || !digits(minutes) || !seconds_ok {
        return None;
    }
    let hours: u64 = hours.parse().ok()?;
    let minutes: u64 = minutes.parse().ok()?;
    let seconds: f64 = seconds.parse().ok()?;
    if parts.len() > 1 && (minutes >= 60 || seconds >= 60.0) {
        return None;
    }
    let whole = hours.checked_mul(3600)?.checked_add(minutes * 60)?;
    std::time::Duration::try_from_secs_f64(whole as f64 + seconds).ok()
}

#[cfg(test)]
mod tests {
    use crate::server::common::util::{
        Recorder, media_ext_from_url, parse_segment_time, path_with_suffix, redact_secrets,
    };
    use crate::server::infrastructure::models::StreamerInfo;
    use chrono::Utc;
    use std::path::PathBuf;

    /// 与 ffmpeg `-to` 接受的时长写法一致（ffmpeg 下载器把原字符串直接交给它）
    #[test]
    fn segment_time_accepts_what_ffmpeg_accepts() {
        use std::time::Duration;
        let secs = |s: u64| Some(Duration::from_secs(s));
        assert_eq!(parse_segment_time("01:00:00"), secs(3600));
        assert_eq!(parse_segment_time("1:2:3"), secs(3723));
        assert_eq!(parse_segment_time("100:00:00"), secs(360_000));
        assert_eq!(parse_segment_time("30:00"), secs(1800));
        assert_eq!(parse_segment_time("3600"), secs(3600));
        assert_eq!(parse_segment_time(" 01:00:00 "), secs(3600));
        assert_eq!(parse_segment_time("1.5"), Some(Duration::from_millis(1500)));
        assert_eq!(parse_segment_time("00:00:00"), secs(0));
        for invalid in [
            "",
            "abc",
            "90:00",
            "1:70:00",
            "01:00:60",
            "01:00:00:00",
            "-1:00:00",
            "1h",
            "inf",
            "NaN",
            "+5",
            "1e3",
        ] {
            assert_eq!(parse_segment_time(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn path_with_suffix_keeps_dots_in_basename() {
        assert_eq!(
            path_with_suffix("Mr.Beast", "flv"),
            PathBuf::from("Mr.Beast.flv")
        );
        assert_eq!(
            path_with_suffix("show.1.5x", ".mp4"),
            PathBuf::from("show.1.5x.mp4")
        );
    }

    #[test]
    fn path_with_suffix_does_not_use_with_extension_truncation() {
        let truncated = PathBuf::from("Mr.Beast").with_extension("flv");
        assert_eq!(truncated, PathBuf::from("Mr.flv"));
        assert_ne!(path_with_suffix("Mr.Beast", "flv"), truncated);
    }

    #[test]
    fn redact_secrets_strips_ffmpeg_cookie_header() {
        // tokio Command 的 Debug 形态：每个 argv 一对双引号
        let raw = r#""ffmpeg" "-headers" "Cookie: SESSDATA=abc123; bili_jct=def456\r\n" "-i" "https://cdn.example.com/live.flv?sid=1""#;
        let redacted = redact_secrets(raw);
        assert!(!redacted.contains("abc123"), "{redacted}");
        assert!(!redacted.contains("def456"), "{redacted}");
        assert!(redacted.contains(r#""Cookie=[redacted]""#), "{redacted}");
        // 直链和非密钥参数原样保留，便于排障
        assert!(
            redacted.contains(r#""https://cdn.example.com/live.flv?sid=1""#),
            "{redacted}"
        );
    }

    #[test]
    fn redact_secrets_strips_streamlink_oauth_and_password_flags() {
        let raw = r#""streamlink" "--twitch-api-header" "Authorization=OAuth tok3n" "--niconico-password" "hunter2" "https://twitch.tv/x" "best""#;
        let redacted = redact_secrets(raw);
        assert!(!redacted.contains("tok3n"), "{redacted}");
        assert!(!redacted.contains("hunter2"), "{redacted}");
        assert!(redacted.contains(r#""Authorization=[redacted]""#), "{redacted}");
        assert!(
            redacted.contains(r#""--niconico-password" "[redacted]""#),
            "{redacted}"
        );
        assert!(redacted.contains(r#""https://twitch.tv/x""#), "{redacted}");
    }

    #[test]
    fn redact_secrets_keeps_unrelated_args() {
        let raw = r#""streamlink" "--hls-duration" "01:00:00" "https://example.com/live?sid=42" "best""#;
        assert_eq!(redact_secrets(raw), raw);
    }

    #[test]
    fn format_title_preserves_filename_invalid_characters() {
        let recorder = Recorder::new(
            Some("{streamer}/{title}:archive".to_string()),
            StreamerInfo::new("streamer", "https://example.com", "live", Utc::now(), ""),
        );

        assert_eq!(recorder.format_title(), "streamer/live:archive");
        assert_eq!(recorder.format_filename(), "streamer_live_archive");
    }

    #[test]
    fn it_works() {
        assert_eq!(
            media_ext_from_url(
                "https://hwa.douyucdn2.cn/live/6512r9pAbb5Ercd1.flv?wsAuth=c77de01c8fcbc7b04b3d6daf66e523f5&token=web-h5-0-6512-f52253ea808109b3e2b66f385345c5e4ebdd692a847af73b&logo=0&expire=0&did=b6b79db91ca484562dcd6a1d5cdd9639&ver=219032101&pt=2&st=0&sid=420338944&mcid2=0&origin=dy&fcdn=hw&fo=0&mix=0&isp="
            ),
            Some("flv".to_string())
        );
    }
}

/// 文件验证配置
#[derive(Clone)]
pub struct FileValidator {
    min_size: u64,
    check_format: bool,
    /// 给了就按切片工作台的引用与场次保留期决定过滤删除要不要推迟
    retention: Option<Retention>,
}

impl FileValidator {
    pub fn new(min_size: u64, check_format: bool) -> Self {
        Self {
            min_size,
            check_format,
            retention: None,
        }
    }

    pub fn with_retention(mut self, retention: Retention) -> Self {
        self.retention = Some(retention);
        self
    }
}

impl Default for FileValidator {
    fn default() -> Self {
        Self {
            min_size: 1024 * 1024 * 100, // 100MB minimum
            check_format: true,
            retention: None,
        }
    }
}

impl FileValidator {
    /// [`Self::validate`] 会不会因为文件太小把它过滤删除。
    pub fn will_delete(&self, path: &Path) -> bool {
        fs::metadata(path).is_ok_and(|m| m.len() < self.min_size)
    }

    /// 验证文件有效性。太小的文件在后台删掉，删之前先等 `settled` 完成（见
    /// [`RecorderHandle::settled`](crate::server::workbench::recorder::RecorderHandle::settled)）。
    pub fn validate(
        &self,
        path: &Path,
        settled: impl Future<Output = ()> + Send + 'static,
    ) -> AppResult<()> {
        let metadata = fs::metadata(path).change_context(AppError::Unknown)?;

        let size = metadata.len();

        if size < self.min_size {
            let display = path.display();
            let path = path.to_owned();
            let retention = self.retention.clone();
            tokio::spawn(async move {
                settled.await;
                let removed = match &retention {
                    Some(retention) => retention::remove(retention, &[&path])
                        .await
                        .map(|outcome| outcome.first() != Some(&Disposal::Deferred))
                        .inspect_err(|e| error!(e=?e))
                        .ok(),
                    None => HookStep::remove_file(&[&path])
                        .await
                        .inspect_err(|e| error!(e=?e))
                        .ok()
                        .map(|()| true),
                };
                if removed == Some(true) {
                    info!("过滤删除 - {}", path.display());
                }
            });
            bail!(AppError::Custom(format!(
                "File {display} too small: {size} bytes, minimum: {} bytes",
                self.min_size
            )));
        }

        // 可选：检查文件格式
        if self.check_format {
            self.validate_format(path)?;
        }

        Ok(())
    }

    fn validate_format(&self, path: &Path) -> AppResult<()> {
        // 简单的格式验证 - 检查扩展名
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "mp4" | "flv" | "ts" | "m3u8" | "mkv" => Ok(()),
                _ => bail!(AppError::Custom(format!("Unsupported format: {}", ext))),
            }
        } else {
            bail!(AppError::Custom("No file extension found".to_string()))
        }
    }
}
