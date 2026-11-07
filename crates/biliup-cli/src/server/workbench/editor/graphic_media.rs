//! Adapted from Uncut export/graphicMedia.ts (MIT). Shared exact frames for preview and export.
//! Persistent software decoders, timestamp-aware VFR seeking, bounded recent frames and LRU.
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    process::Stdio,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, ChildStdout, Command},
    sync::{Mutex, Notify, OnceCell, RwLockReadGuard, Semaphore},
};
use tokio_util::sync::CancellationToken;
pub struct ProjectFrames {
    frames: Arc<Frames>,
    media: BTreeMap<String, (std::path::PathBuf, String)>,
}
impl ProjectFrames {
    pub fn new(project: &Value, paths: &BTreeMap<String, String>) -> Self {
        Self {
            frames: Arc::new(Frames::default()),
            media: project["media"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let id = m["id"].as_str()?;
                    let kind = m["kind"].as_str()?;
                    Some((id.to_string(), (paths.get(id)?.into(), kind.to_string())))
                })
                .collect(),
        }
    }
    pub async fn picture(
        &self,
        request: &Request,
        cancel: &CancellationToken,
    ) -> Result<Arc<Picture>, String> {
        request.validate()?;
        let (path, kind) = self
            .media
            .get(&request.id)
            .ok_or("图形引用的素材不属于当前工程")?;
        self.frames.picture(path, kind, request, cancel, None).await
    }
}
const CACHE_BYTES: usize = 192 * 1024 * 1024;
const RECENT_BYTES: usize = 32 * 1024 * 1024;
const DECODERS: usize = 4;
const IDLE: Duration = Duration::from_secs(20);
pub static PREVIEW: LazyLock<Arc<Frames>> = LazyLock::new(|| Arc::new(Frames::default()));
static PROBES: Semaphore = Semaphore::const_new(4);
#[derive(Clone, Deserialize)]
pub struct Request {
    pub id: String,
    #[serde(default)]
    pub t: f64,
    pub w: Option<u32>,
    pub h: Option<u32>,
}
impl Request {
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .id
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("素材标识无效".into());
        }
        if !self.t.is_finite()
            || !(0. ..=86400.).contains(&self.t)
            || [self.w, self.h]
                .into_iter()
                .flatten()
                .any(|n| n == 0 || n > 8192)
        {
            return Err("素材帧的时间或尺寸无效".into());
        }
        Ok(())
    }
}
pub struct Picture {
    pub bytes: Arc<Vec<u8>>,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
    pub frame: Option<usize>,
}
#[derive(Default)]
pub struct Frames {
    infos: Mutex<BTreeMap<String, Arc<OnceCell<Info>>>>,
    pool: Mutex<BTreeMap<String, Entry>>,
    cache: Mutex<VecDeque<(String, Arc<Picture>)>>,
    wake: Arc<Notify>,
    janitor: AtomicBool,
}
struct Entry {
    slot: Arc<Mutex<Option<Decoder>>>,
    used: Instant,
}
struct Lease {
    slot: Arc<Mutex<Option<Decoder>>>,
    wake: Arc<Notify>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.wake.notify_one()
    }
}
struct Decoder {
    _child: Child,
    stdout: ChildStdout,
    next: usize,
    recent: VecDeque<(usize, Arc<Vec<u8>>)>,
    _lifecycle: Option<RwLockReadGuard<'static, ()>>,
}
#[derive(Clone)]
struct Info {
    width: u32,
    height: u32,
    fps: f64,
    frames: Option<usize>,
    untagged: Option<u32>,
    times: Vec<f64>,
}
impl Info {
    fn frame(&self, t: f64) -> usize {
        if self.times.is_empty() {
            ((t * self.fps + 1e-6).floor() as usize)
                .min(self.frames.unwrap_or(usize::MAX).saturating_sub(1))
        } else {
            self.times
                .partition_point(|n| *n <= t + 0.0001)
                .saturating_sub(1)
        }
    }
    fn seek(&self, n: usize) -> f64 {
        if n == 0 {
            0.
        } else if self.times.len() > n {
            (self.times[n - 1] + self.times[n]) / 2.
        } else {
            (n as f64 - 0.5) / self.fps
        }
    }
    fn size(&self, ask: &Request) -> (u32, u32) {
        let k = 1f64
            .min(1920. / self.width.max(self.height) as f64)
            .min(ask.w.map(|w| w as f64 / self.width as f64).unwrap_or(1.))
            .min(ask.h.map(|h| h as f64 / self.height as f64).unwrap_or(1.));
        let even = |n: u32| ((n as f64 * k / 2.).round() as u32 * 2).max(2);
        (even(self.width), even(self.height))
    }
}
fn input(command: &mut Command, path: &Path) {
    command
        .args([
            "-protocol_whitelist",
            "file,pipe",
            "-format_whitelist",
            super::imports::FORMATS,
            "-i",
        ])
        .arg(path);
}
fn process() -> Command {
    let mut c = crate::tools::ffmpeg_command();
    c.args(["-nostdin", "-hide_banner"])
        .stdin(Stdio::null())
        .kill_on_drop(true);
    c
}
async fn run(
    mut command: Command,
    cancel: &CancellationToken,
    max: usize,
    seconds: u64,
) -> Result<(Vec<u8>, String), String> {
    let _permit = tokio::select! {_ = cancel.cancelled()=>return Err("素材解码已取消".into()),p=PROBES.acquire()=>p.map_err(|_|"素材解码服务已停止")?};
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("启动素材解码失败：{e}"))?;
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let read = async {
        let mut bytes = Vec::new();
        stdout
            .take(max as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        if bytes.len() > max {
            Err("素材解码结果超过大小限制".to_string())
        } else {
            Ok(bytes)
        }
    };
    let diagnostic = async {
        let mut tail = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stderr.read(&mut buf).await.map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buf[..n]);
            if tail.len() > 16384 {
                tail.drain(..tail.len() - 16384);
            }
        }
        Ok::<_, String>(String::from_utf8_lossy(&tail).into_owned())
    };
    let result = tokio::select! {_ = cancel.cancelled()=>Err("素材解码已取消".into()),r=tokio::time::timeout(Duration::from_secs(seconds),async{let (bytes,log)=tokio::try_join!(read,diagnostic)?;let status=child.wait().await.map_err(|e|e.to_string())?;if !status.success(){tracing::warn!(%log,"图形素材解码失败");return Err("无法读取图形使用的素材".into())}Ok((bytes,log))})=>r.unwrap_or_else(|_|Err("素材解码超时".into()))};
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
fn capture(text: &str, pattern: &str) -> Option<String> {
    regex::Regex::new(pattern)
        .ok()?
        .captures(text)
        .map(|c| c[1].to_string())
}
pub(super) async fn colour_hint(
    path: &Path,
    cancel: &CancellationToken,
) -> Result<Option<u32>, String> {
    Ok(probe(path, cancel).await?.untagged)
}
async fn probe(path: &Path, cancel: &CancellationToken) -> Result<Info, String> {
    let mut command = process();
    command.arg("-copyts");
    input(&mut command, path);
    command.args([
        "-map",
        "0:v:0",
        "-vf",
        "showinfo",
        "-frames:v",
        "1",
        "-fps_mode",
        "passthrough",
        "-an",
        "-f",
        "null",
        "-",
    ]);
    let (_, text) = run(command, cancel, 1024, 30).await?;
    let dimensions = regex::Regex::new(r"showinfo[^\n]*\bs:(\d+)x(\d+)").unwrap();
    let dims = dimensions.captures(&text).ok_or("无法读取图形素材分辨率")?;
    let width: u32 = dims[1].parse().map_err(|_| "素材宽度无效")?;
    let height: u32 = dims[2].parse().map_err(|_| "素材高度无效")?;
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err("素材尺寸超出范围".into());
    }
    let input = text.split("Output #").next().unwrap_or(&text);
    let rate = |p: &str| {
        capture(input, p)
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0. && *v < 1000.)
    };
    let nominal = rate(r"([0-9.]+) tbr");
    let average = rate(r"([0-9.]+) fps");
    let exact = regex::Regex::new(r"showinfo[^\n]*frame_rate:\s*(\d+)/(\d+)")
        .unwrap()
        .captures(&text)
        .and_then(|c| Some(c[1].parse::<f64>().ok()? / c[2].parse::<f64>().ok()?))
        .filter(|v| v.is_finite() && *v > 0. && *v < 1000.);
    let fps = exact.or(average).or(nominal).unwrap_or(30.);
    let duration = regex::Regex::new(r"Duration: (\d+):(\d+):([0-9.]+)")
        .unwrap()
        .captures(input)
        .and_then(|c| {
            Some(
                c[1].parse::<f64>().ok()? * 3600.
                    + c[2].parse::<f64>().ok()? * 60.
                    + c[3].parse::<f64>().ok()?,
            )
        });
    let start = capture(input, r"start:\s+([-+0-9.eE]+)")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.);
    let colour = capture(&text, r"showinfo[^\n]*color_space:(\w+)");
    let untagged = colour
        .as_deref()
        .is_none_or(|s| matches!(s, "unknown" | "unspecified" | "reserved"))
        .then_some(height);
    let mut info = Info {
        width,
        height,
        fps,
        frames: duration.map(|d| (d * fps).round().max(1.) as usize),
        untagged,
        times: Vec::new(),
    };
    if nominal
        .zip(average)
        .is_some_and(|(n, a)| (n - a).abs() > n * 0.001)
    {
        info.times = packet_times(path, start, cancel).await?;
        if !info.times.is_empty() {
            info.frames = Some(info.times.len())
        }
    }
    Ok(info)
}
// Stream-copy packet timestamps uses the configured FFmpeg, including bundles without ffprobe.
async fn packet_times(
    path: &Path,
    start: f64,
    cancel: &CancellationToken,
) -> Result<Vec<f64>, String> {
    let mut command = process();
    command.args(["-v", "error", "-copyts"]);
    input(&mut command, path);
    command.args([
        "-map", "0:v:0", "-an", "-sn", "-dn", "-c:v", "copy", "-f", "framecrc", "-",
    ]);
    let (bytes, _) = run(command, cancel, 128 * 1024 * 1024, 120).await?;
    let mut tb = 0.;
    let mut times = Vec::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        if let Some(value) = line.strip_prefix("#tb 0:") {
            if let Some((a, b)) = value.trim().split_once('/') {
                tb = a.trim().parse::<f64>().unwrap_or(0.) / b.trim().parse::<f64>().unwrap_or(1.)
            }
        } else if !line.starts_with('#') && tb > 0. {
            let fields: Vec<_> = line.split(',').map(str::trim).collect();
            let discarded = fields
                .iter()
                .filter_map(|field| field.strip_prefix("F=0x"))
                .any(|flags| u32::from_str_radix(flags, 16).is_ok_and(|flags| flags & 4 != 0));
            if fields.len() >= 6
                && !discarded
                && let Ok(pts) = fields[2].parse::<f64>()
            {
                let t = pts * tb - start;
                if t.is_finite() {
                    times.push(t)
                }
            }
        }
    }
    times.sort_by(f64::total_cmp);
    if times.is_empty() {
        return Err("无法读取变帧率素材的帧时间戳".into());
    }
    Ok(times)
}
impl Frames {
    /// Exact display size for a smaller preview-proxy reply. Reuses the original's probe cache.
    pub async fn display_size(
        &self,
        path: &Path,
        ask: &Request,
        cancel: &CancellationToken,
    ) -> Result<(u32, u32), String> {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|_| "素材文件不可读取")?;
        let identity = format!(
            "{}|{}|{}",
            path.display(),
            metadata.len(),
            metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let cell = self.info_cell(&identity).await;
        Ok(cell
            .get_or_try_init(|| probe(path, cancel))
            .await?
            .size(ask))
    }
    async fn info_cell(&self, identity: &str) -> Arc<OnceCell<Info>> {
        let mut infos = self.infos.lock().await;
        if infos.len() >= 16 && !infos.contains_key(identity) {
            let key = infos.keys().next().cloned().unwrap();
            infos.remove(&key);
        }
        infos.entry(identity.to_string()).or_default().clone()
    }
    async fn cached(&self, key: &str) -> Option<Arc<Picture>> {
        let mut cache = self.cache.lock().await;
        let index = cache.iter().position(|(k, _)| k == key)?;
        let entry = cache.remove(index)?;
        let result = entry.1.clone();
        cache.push_back(entry);
        Some(result)
    }
    async fn remember(&self, key: String, picture: Arc<Picture>) {
        let mut cache = self.cache.lock().await;
        cache.retain(|(k, _)| k != &key);
        cache.push_back((key, picture));
        let mut bytes = cache.iter().map(|(_, p)| p.bytes.len()).sum::<usize>();
        while bytes > CACHE_BYTES && cache.len() > 1 {
            bytes -= cache.pop_front().unwrap().1.bytes.len();
        }
    }
    async fn lease(
        self: &Arc<Self>,
        key: &str,
        cancel: &CancellationToken,
    ) -> Result<Lease, String> {
        loop {
            let notified = self.wake.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let mut pool = self.pool.lock().await;
            if !pool.contains_key(key) && pool.len() >= DECODERS {
                let idle = pool
                    .iter()
                    .filter(|(_, e)| Arc::strong_count(&e.slot) == 1)
                    .min_by_key(|(_, e)| e.used)
                    .map(|(k, _)| k.clone());
                if let Some(k) = idle {
                    pool.remove(&k);
                }
            }
            if pool.contains_key(key) || pool.len() < DECODERS {
                let entry = pool.entry(key.into()).or_insert_with(|| Entry {
                    slot: Arc::new(Mutex::new(None)),
                    used: Instant::now(),
                });
                entry.used = Instant::now();
                let lease = Lease {
                    slot: entry.slot.clone(),
                    wake: self.wake.clone(),
                };
                drop(pool);
                if !self.janitor.swap(true, Ordering::SeqCst) {
                    let weak = Arc::downgrade(self);
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(IDLE).await;
                            let Some(frames) = weak.upgrade() else { break };
                            let mut pool = frames.pool.lock().await;
                            pool.retain(|_, e| {
                                Arc::strong_count(&e.slot) > 1 || e.used.elapsed() < IDLE
                            });
                            frames.wake.notify_waiters();
                            if pool.is_empty() {
                                frames.janitor.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                    });
                }
                return Ok(lease);
            }
            drop(pool);
            tokio::select! {_ = cancel.cancelled()=>return Err("素材解码已取消".into()),_ = notified=>{}}
        }
    }
    pub async fn picture(
        self: &Arc<Self>,
        path: &Path,
        kind: &str,
        ask: &Request,
        cancel: &CancellationToken,
        lifecycle: Option<RwLockReadGuard<'static, ()>>,
    ) -> Result<Arc<Picture>, String> {
        ask.validate()?;
        if !matches!(kind, "image" | "video") {
            return Err("音频素材不能用于图形画面".into());
        }
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|_| "素材文件不可读取")?;
        let identity = format!(
            "{}|{}|{}",
            path.display(),
            metadata.len(),
            metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        if kind == "image"
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
        {
            if let Some(p) = self.cached(&identity).await {
                return Ok(p);
            }
            if metadata.len() > 10 * 1024 * 1024 {
                return Err("SVG 素材超过 10 MiB".into());
            }
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|_| "SVG 素材不可读取")?;
            let (width, height) =
                super::svg::size(std::str::from_utf8(&bytes).map_err(|_| "SVG 素材编码无效")?)?;
            let picture = Arc::new(Picture {
                bytes: Arc::new(bytes),
                mime: "image/svg+xml",
                width,
                height,
                frame: None,
            });
            self.remember(identity, picture.clone()).await;
            return Ok(picture);
        }
        let cell = self.info_cell(&identity).await;
        let info = cell.get_or_try_init(|| probe(path, cancel)).await?;
        let (width, height) = info.size(ask);
        let n = if kind == "image" {
            0
        } else {
            info.frame(ask.t)
        };
        let key = format!("{identity}|{width}x{height}|{n}");
        if let Some(picture) = self.cached(&key).await {
            return Ok(picture);
        }
        let picture = if kind == "image" {
            let mut command = process();
            command.args(["-v", "error"]);
            input(&mut command, path);
            command.args([
                "-frames:v",
                "1",
                "-vf",
                &format!("scale={width}:{height}:flags=bicubic,format=rgba"),
                "-c:v",
                "png",
                "-f",
                "image2pipe",
                "pipe:1",
            ]);
            let (bytes, _) = run(command, cancel, 32 * 1024 * 1024, 30).await?;
            Arc::new(Picture {
                bytes: Arc::new(bytes),
                mime: "image/png",
                width,
                height,
                frame: None,
            })
        } else {
            let lease = self
                .lease(&format!("{identity}|{width}x{height}"), cancel)
                .await?;
            let mut slot = tokio::select! {_ = cancel.cancelled()=>return Err("素材解码已取消".into()),s=lease.slot.lock()=>s};
            // Take ownership before awaiting stdout: cancellation cannot leave a half-read frame in the pool.
            let mut decoder = slot.take();
            let mut lifecycle = lifecycle;
            let mut target = n;
            let mut misses = 0;
            let raw = loop {
                if let Some(d) = &decoder
                    && let Some((index, raw)) = d.recent.iter().find(|(index, _)| *index == target)
                {
                    break (*index, raw.clone());
                }
                if decoder.as_ref().is_none_or(|d| {
                    target < d.next || target - d.next > (3. * info.fps).ceil().max(30.) as usize
                }) {
                    if let Some(mut previous) = decoder.take()
                        && lifecycle.is_none()
                    {
                        lifecycle = previous._lifecycle.take()
                    }
                    decoder = Some(start(path, info, width, height, target, lifecycle.take())?);
                }
                let d = decoder.as_mut().unwrap();
                let read = async {
                    while d.next <= target {
                        let mut bytes = vec![0; width as usize * height as usize * 3];
                        match d.stdout.read_exact(&mut bytes).await {
                            Ok(_) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                            Err(e) => return Err(e.to_string()),
                        };
                        d.recent.push_back((d.next, Arc::new(bytes)));
                        d.next += 1;
                        while d.recent.len() > 1
                            && d.recent.len() * width as usize * height as usize * 3 > RECENT_BYTES
                        {
                            d.recent.pop_front();
                        }
                    }
                    Ok::<_, String>(())
                };
                tokio::select! {_ = cancel.cancelled()=>return Err("素材解码已取消".into()),r=tokio::time::timeout(Duration::from_secs(30),read)=>r.map_err(|_|"素材逐帧解码超时")??};
                if let Some((index, raw)) = d.recent.back() {
                    break (*index, raw.clone());
                }
                misses += 1;
                if target == 0 || misses >= 4 {
                    return Err("素材没有可读取的视频帧".into());
                }
                target = target.saturating_sub(info.fps.ceil() as usize);
                if let Some(mut previous) = decoder.take() {
                    lifecycle = previous._lifecycle.take();
                }
            };
            *slot = decoder;
            Arc::new(Picture {
                bytes: Arc::new(bmp(&raw.1, width, height)),
                mime: "image/bmp",
                width,
                height,
                frame: Some(raw.0),
            })
        };
        self.remember(key, picture.clone()).await;
        Ok(picture)
    }
}
fn start(
    path: &Path,
    info: &Info,
    width: u32,
    height: u32,
    from: usize,
    lifecycle: Option<RwLockReadGuard<'static, ()>>,
) -> Result<Decoder, String> {
    let mut command = process();
    command.args(["-v", "error"]);
    if from > 0 {
        command.args(["-ss", &format!("{:.6}", info.seek(from))]);
    }
    input(&mut command, path);
    let colour = match info.untagged {
        None => "",
        Some(h) if h < 720 => {
            "setparams=colorspace=smpte170m:color_primaries=smpte170m:color_trc=smpte170m:range=tv,colorspace=all=bt709:format=yuv420p,"
        }
        Some(_) => "setparams=colorspace=bt709:color_primaries=bt709:color_trc=bt709:range=tv,",
    };
    command
        .args([
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-dn",
            "-fps_mode",
            "passthrough",
            "-vf",
            &format!("{colour}scale={width}:{height}:flags=bicubic,format=bgr24"),
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgr24",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|e| format!("启动素材解码失败：{e}"))?;
    let stdout = child.stdout.take().unwrap();
    Ok(Decoder {
        _child: child,
        stdout,
        next: from,
        recent: VecDeque::new(),
        _lifecycle: lifecycle,
    })
}
fn bmp(raw: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row = width as usize * 3;
    let stride = (row + 3) & !3;
    let mut bytes = vec![0; 54 + stride * height as usize];
    let len = bytes.len() as u32;
    bytes[0..2].copy_from_slice(b"BM");
    bytes[2..6].copy_from_slice(&len.to_le_bytes());
    bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&(-(height as i32)).to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes[34..38].copy_from_slice(&(stride as u32 * height).to_le_bytes());
    for y in 0..height as usize {
        bytes[54 + y * stride..54 + y * stride + row].copy_from_slice(&raw[y * row..y * row + row]);
    }
    bytes
}
