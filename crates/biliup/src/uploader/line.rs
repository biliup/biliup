use crate::error::Result;
use crate::uploader::{Uploader, VideoFile, VideoStream};
use futures::{Stream, TryStreamExt};
use reqwest::{Body, RequestBuilder};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::ffi::OsStr;
use std::path::Path;

use crate::client::StatelessClient;
use crate::error::Kind::{Custom, RateLimit};
use crate::uploader::bilibili::{BiliBili, Video};
use crate::uploader::line::upos::{Upos, UposPart};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// 拉取线路列表（`preupload?r=probe`）的超时。
const PROBE_LIST_TIMEOUT: Duration = Duration::from_secs(15);
/// 单条线路测速的硬上限。
///
/// 共用的 HTTP 客户端只设了连接超时，没有请求超时；一条半开连接或极慢的线路会让
/// `send()` 永远不返回，AUTO 选线是顺序进行的，于是整个上传初始化（包括边录边传
/// 的启动）都会卡死在这里。探测体只有约 0.1 MB，正常几百毫秒就能返回，超过上限
/// 的线路本来也不值得选。
const PROBE_LINE_TIMEOUT: Duration = Duration::from_secs(15);
/// 服务端没有给出 POST 探测体大小时的默认值（MB），对齐 B 站网页端与 Python 版。
const DEFAULT_PROBE_POST_MB: f64 = 0.1;

pub mod upos;

pub struct Parcel {
    // line: &'a Line,
    line: Bucket,
    video_file: VideoFile,
}

impl Parcel {
    pub async fn upload<F, S, B>(
        self,
        client: StatelessClient,
        limit: usize,
        progress: F,
    ) -> Result<Video>
    where
        F: FnOnce(VideoStream) -> S,
        S: Stream<Item = Result<(B, usize)>>,
        B: Into<Body> + Clone,
    {
        let mut video = match self.line {
            Bucket::Upos(bucket) => {
                // let bucket: crate::uploader::upos::Bucket = self.pre_upload(client).await?;
                let chunk_size = bucket.chunk_size;
                let upos = Upos::from(client, bucket).await?;
                let mut parts = Vec::new();
                let stream = upos
                    .upload_stream(
                        progress(self.video_file.get_stream(chunk_size)?),
                        self.video_file.total_size,
                        limit,
                    )
                    .await?;
                tokio::pin!(stream);
                while let Some((part, _size)) = stream.try_next().await? {
                    parts.push(part);
                }
                upos.get_ret_video_info(&parts, &self.video_file.filepath)
                    .await?
            }
        };

        if video.title.is_none()
            && let Some(filename) = self.video_file.filepath.file_stem().and_then(OsStr::to_str)
        {
            // B站限制分P视频标题不能超过80字符，需要截断
            video.title = Some(if filename.chars().count() >= 80 {
                Video::truncate_title(filename, 80)
            } else {
                filename.to_string()
            });
        };
        Ok(video)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Probe {
    #[serde(rename = "OK")]
    ok: u8,
    lines: Vec<Line>,
    probe: serde_json::Value,
}

impl Probe {
    pub async fn probe(client: &reqwest::Client) -> Result<Line> {
        let res: Self = client
            .get("https://member.bilibili.com/preupload?r=probe")
            .timeout(PROBE_LIST_TIMEOUT)
            .send()
            .await?
            .json()
            .await?;
        Self::select_line(client, &res.probe, res.lines, PROBE_LINE_TIMEOUT).await
    }

    /// 逐条线路测速并选出耗时最短者。
    /// 单条线路网络错误、异常状态码或超过 `per_line_timeout` 都只跳过该线路，
    /// 全部失败才返回错误；任何一条线路都不可能让整个选线过程无限等待。
    async fn select_line(
        client: &reqwest::Client,
        probe: &serde_json::Value,
        lines: Vec<Line>,
        per_line_timeout: Duration,
    ) -> Result<Line> {
        let mut choice_line: Line = Default::default();
        for mut line in lines {
            let instant = Instant::now();
            let request = Probe::ping(probe, &probe_url(&line.probe_url), client)
                .timeout(per_line_timeout)
                .send();
            // 请求级超时之外再套一层，确保连 DNS/连接阶段也受同一上限约束
            match tokio::time::timeout(per_line_timeout, request).await {
                Ok(Ok(response)) if response.status().is_success() => {
                    line.cost = instant.elapsed().as_millis();
                    info!("{}: {}", line.query, line.cost);
                    if choice_line.cost > line.cost {
                        choice_line = line
                    }
                }
                Ok(Ok(response)) => {
                    warn!(
                        "{} 测速返回异常状态码 {}，跳过该线路",
                        line.query,
                        response.status()
                    );
                }
                Ok(Err(e)) => {
                    warn!("{} 测速失败，跳过该线路: {e}", line.query);
                }
                Err(_) => {
                    warn!(
                        "{} 测速超过 {:?} 未响应，跳过该线路",
                        line.query, per_line_timeout
                    );
                }
            }
        }
        if choice_line.cost == u128::MAX {
            return Err(Custom("所有上传线路测速均失败".to_string()));
        }
        Ok(choice_line)
    }

    fn ping(probe: &serde_json::Value, url: &str, client: &reqwest::Client) -> RequestBuilder {
        if !probe["get"].is_null() {
            client.get(url)
        } else {
            client.post(url).body(vec![0; probe_post_bytes(probe)])
        }
    }
}

/// B 站返回的 `probe_url` 形如 `//host/OK`，需要补上 https；已带协议的地址原样使用。
fn probe_url(probe_url: &str) -> String {
    if probe_url.starts_with("http://") || probe_url.starts_with("https://") {
        probe_url.to_string()
    } else {
        format!("https:{probe_url}")
    }
}

/// POST 探测体大小：以服务端 `probe.post`（单位 MB）为准，缺省 0.1 MB。
/// 之前固定发 10 MB，是 B 站建议值的 100 倍，慢线路上单条测速就要几十秒。
fn probe_post_bytes(probe: &serde_json::Value) -> usize {
    let mb = probe["post"]
        .as_f64()
        .filter(|mb| mb.is_finite() && *mb > 0.0)
        .unwrap_or(DEFAULT_PROBE_POST_MB);
    (mb * 1024.0 * 1024.0) as usize
}

#[derive(Clone)]
enum Bucket {
    Upos(upos::Bucket),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Line {
    os: Uploader,
    probe_url: String,
    query: String,
    #[serde(skip)]
    cost: u128,
}

pub struct StreamParcel {
    line: Bucket,
    file_name: String,
    total_size: u64,
}

pub struct UploadedStream {
    upos: Upos,
    file_name: String,
    declared_size: u64,
    parts: Vec<UposPart>,
    uploaded_size: u64,
}

impl UploadedStream {
    pub fn declared_size(&self) -> u64 {
        self.declared_size
    }

    pub fn uploaded_size(&self) -> u64 {
        self.uploaded_size
    }

    pub fn parts_len(&self) -> usize {
        self.parts.len()
    }

    pub async fn complete(self) -> Result<Video> {
        let video = self
            .upos
            .get_ret_video_info(&self.parts, Path::new(&self.file_name))
            .await?;
        Ok(with_stream_video_title(video, &self.file_name))
    }
}

impl StreamParcel {
    pub fn chunk_size(&self) -> usize {
        match &self.line {
            Bucket::Upos(bucket) => bucket.chunk_size,
        }
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub async fn upload_stream<S, B>(
        self,
        client: StatelessClient,
        limit: usize,
        stream: S,
    ) -> Result<Video>
    where
        S: Stream<Item = Result<(B, usize)>>,
        B: Into<Body> + Clone,
    {
        self.upload_parts(client, limit, stream)
            .await?
            .complete()
            .await
    }

    pub async fn upload_parts<S, B>(
        self,
        client: StatelessClient,
        limit: usize,
        stream: S,
    ) -> Result<UploadedStream>
    where
        S: Stream<Item = Result<(B, usize)>>,
        B: Into<Body> + Clone,
    {
        match self.line {
            Bucket::Upos(bucket) => {
                let upos = Upos::from(client, bucket).await?;
                let mut parts = Vec::new();
                let mut uploaded_size = 0u64;
                {
                    let uploaded = upos.upload_stream(stream, self.total_size, limit).await?;
                    tokio::pin!(uploaded);
                    while let Some((part, size)) = uploaded.try_next().await? {
                        parts.push(part);
                        uploaded_size += size as u64;
                    }
                }
                Ok(UploadedStream {
                    upos,
                    file_name: self.file_name,
                    declared_size: self.total_size,
                    parts,
                    uploaded_size,
                })
            }
        }
    }
}

fn with_stream_video_title(mut video: Video, file_name: &str) -> Video {
    if video.title.is_none()
        && let Some(stem) = Path::new(file_name).file_stem().and_then(OsStr::to_str)
    {
        video.title = Some(if stem.chars().count() >= 80 {
            Video::truncate_title(stem, 80)
        } else {
            stem.to_string()
        });
    }
    video
}

impl Line {
    async fn request_bucket(
        &self,
        bili: &BiliBili,
        file_name: &str,
        total_size: u64,
    ) -> Result<Bucket> {
        let profile = "ugcupos/bup"; // ugcfx/bup 需上传视频metadata和frame.zip
        let params = json!({
            "name": file_name,
            "r": self.os, // upos
            "profile": profile,
            "ssl": 0,
            "version": "2.14.0",
            "build": 2140000,
            "size": total_size,
        });
        info!("pre_upload: {}", params);

        let response = bili
            .client
            .get(format!(
                "https://member.bilibili.com/preupload?{}",
                self.query
            ))
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let response_text = response.text().await?;

            // 尝试解析JSON错误响应，检测限流错误（code: 601）
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&response_text)
                && let Some(code) = error_json.get("code").and_then(|c| c.as_i64())
                && code == 601
            {
                let message = error_json
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("上传过快")
                    .to_string();
                return Err(RateLimit { code, message });
            }

            return Err(Custom(format!(
                "Failed to pre_upload from {}",
                response_text
            )));
        }

        match self.os {
            Uploader::Upos => Ok(Bucket::Upos(response.json().await?)),
        }
    }

    pub async fn pre_upload(&self, bili: &BiliBili, video_file: VideoFile) -> Result<Parcel> {
        let bucket = self
            .request_bucket(bili, &video_file.file_name, video_file.total_size)
            .await?;
        Ok(Parcel {
            line: bucket,
            video_file,
        })
    }

    /// 边录边传：在还没有完整文件时，按预声明大小申请 UPOS 上传。
    pub async fn pre_upload_stream(
        &self,
        bili: &BiliBili,
        file_name: impl Into<String>,
        total_size: u64,
    ) -> Result<StreamParcel> {
        let file_name = file_name.into();
        let bucket = self.request_bucket(bili, &file_name, total_size).await?;
        Ok(StreamParcel {
            line: bucket,
            file_name,
            total_size,
        })
    }
}

impl Default for Line {
    fn default() -> Self {
        Line {
            cost: u128::MAX,
            ..bldsa()
        }
    }
}

/// B站自建DSA
pub fn bldsa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=bldsa&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnbldsa.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// B站自建DSA
pub fn cnbldsa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=cnbldsa&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnbldsa.bilivideo.cn/OK".into(),
        cost: 0,
    }
}

/// B站自建DSA
pub fn andsa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=andsa&probe_version=20221109".into(),
        probe_url: "//c3350892csdsa.anitama.cn/OK".into(),
        cost: 0,
    }
}

/// B站自建DSA
pub fn atdsa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=atdsa&probe_version=20221109".into(),
        probe_url: "//c3350892csdsa.anitama.net/OK".into(),
        cost: 0,
    }
}

/// 百度云
pub fn bda2() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=bda2&zone=cs".into(),
        probe_url: "//upos-cs-upcdnbda2.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// 百度云
pub fn cnbd() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=cnbd&zone=cs".into(),
        probe_url: "//upos-cs-upcdnbd.bilivideo.cn/OK".into(),
        cost: 0,
    }
}

/// 百度云
pub fn anbd() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=anbd&zone=cs".into(),
        probe_url: "//c3350892csbd.anitama.cn/OK".into(),
        cost: 0,
    }
}

/// 百度云
pub fn atbd() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=atbd&zone=cs".into(),
        probe_url: "//c3350892csbd.anitama.net/OK".into(),
        cost: 0,
    }
}

/// 腾讯云EO
pub fn tx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=tx&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdntx.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// 腾讯云EO
pub fn cntx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=cntx&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdntx.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// 腾讯云EO
pub fn antx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=antx&probe_version=20221109".into(),
        probe_url: "//c3350892cstx.anitama.cn/OK".into(),
        cost: 0,
    }
}

/// 腾讯云EO
pub fn attx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=attx&probe_version=20221109".into(),
        probe_url: "//c3350892cstx.anitama.net/OK".into(),
        cost: 0,
    }
}

/// 腾讯云EO海外
pub fn txa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=txa&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdntxa.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// 阿里云海外
pub fn alia() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=alia&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnalia.bilivideo.com/OK".into(),
        cost: 0,
    }
}

/// B站自建
pub fn estx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20250923&upcdn=estx&zone=cs".into(),
        probe_url: "//e17962d5cstx.esheep.com/OK".into(),
        cost: 0,
    }
}

/// B站自建
pub fn akbd() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20250923&upcdn=akbd&zone=cs".into(),
        probe_url: "//bb27c891csbd.aikobo.cn/OK".into(),
        cost: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broken_line(query: &str) -> Line {
        Line {
            os: Uploader::Upos,
            // "https:" 拼接后不是合法 URL，send() 直接报错且不产生网络请求
            probe_url: String::new(),
            query: query.into(),
            cost: 0,
        }
    }

    /// 单条线路 send 失败不应中断测速，全部失败时返回统一错误而不是首条线路的网络错误。
    #[tokio::test]
    async fn select_line_skips_failed_lines_and_reports_all_failed() {
        let client = reqwest::Client::new();
        let result = Probe::select_line(
            &client,
            &serde_json::json!({"get": {}}),
            vec![broken_line("a"), broken_line("b")],
            PROBE_LINE_TIMEOUT,
        )
        .await;
        match result {
            Err(Custom(message)) => assert_eq!(message, "所有上传线路测速均失败"),
            other => panic!("期望所有线路失败的统一错误，实际为 {other:?}"),
        }
    }

    /// 无候选线路时同样返回错误，而不是把未测速的默认线路当作结果。
    #[tokio::test]
    async fn select_line_rejects_empty_lines() {
        let client = reqwest::Client::new();
        let result = Probe::select_line(
            &client,
            &serde_json::json!({"get": {}}),
            Vec::new(),
            PROBE_LINE_TIMEOUT,
        )
        .await;
        assert!(result.is_err());
    }

    /// 只接受连接、永不回包的“半开”线路：模拟卡死的上传 CDN。
    async fn hanging_line(query: &str) -> (Line, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                held.push(socket);
            }
        });
        let line = Line {
            os: Uploader::Upos,
            // 与 B 站返回的形式一致：无协议前缀，走 https 握手，对端永不应答
            probe_url: format!("//127.0.0.1:{port}/OK"),
            query: query.into(),
            cost: 0,
        };
        (line, server)
    }

    /// 立刻返回 200 的明文 HTTP 线路：模拟正常可用的上传 CDN。
    async fn healthy_line(query: &str) -> (Line, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 8192];
                    let mut received = Vec::new();
                    while let Ok(n) = socket.read(&mut buf).await {
                        if n == 0 {
                            return;
                        }
                        received.extend_from_slice(&buf[..n]);
                        if received.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let _ = socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                        )
                        .await;
                });
            }
        });
        let line = Line {
            os: Uploader::Upos,
            probe_url: format!("http://127.0.0.1:{port}/OK"),
            query: query.into(),
            cost: 0,
        };
        (line, server)
    }

    /// 卡死的线路必须在单条超时内被跳过，最终仍选出可用线路，而不是无限挂起。
    #[tokio::test]
    async fn select_line_skips_hanging_line_within_timeout_and_picks_healthy_one() {
        let client = reqwest::Client::new();
        let (hanging, hang_server) = hanging_line("hang").await;
        let (healthy, ok_server) = healthy_line("ok").await;
        let per_line_timeout = Duration::from_millis(500);

        let started = Instant::now();
        let chosen = Probe::select_line(
            &client,
            &serde_json::json!({"get": {}}),
            vec![hanging, healthy],
            per_line_timeout,
        )
        .await
        .expect("有可用线路时必须选出线路");
        let elapsed = started.elapsed();

        assert_eq!(chosen.query, "ok");
        assert_ne!(chosen.cost, u128::MAX);
        assert!(
            elapsed < per_line_timeout * 6,
            "卡死线路只能拖慢一个超时周期，实际耗时 {elapsed:?}"
        );
        hang_server.abort();
        ok_server.abort();
    }

    /// 全部线路都卡死时要在有限时间内报错，交给调用方回退默认线路。
    #[tokio::test]
    async fn select_line_fails_in_bounded_time_when_every_line_hangs() {
        let client = reqwest::Client::new();
        let (first, server_a) = hanging_line("a").await;
        let (second, server_b) = hanging_line("b").await;
        let per_line_timeout = Duration::from_millis(300);

        let started = Instant::now();
        let result = Probe::select_line(
            &client,
            &serde_json::json!({"get": {}}),
            vec![first, second],
            per_line_timeout,
        )
        .await;
        let elapsed = started.elapsed();

        match result {
            Err(Custom(message)) => assert_eq!(message, "所有上传线路测速均失败"),
            other => panic!("期望统一失败错误，实际为 {other:?}"),
        }
        assert!(
            elapsed < per_line_timeout * 8,
            "两条卡死线路应在约两个超时周期内放弃，实际耗时 {elapsed:?}"
        );
        server_a.abort();
        server_b.abort();
    }

    #[test]
    fn probe_url_prefixes_scheme_only_when_missing() {
        assert_eq!(
            probe_url("//upos-cs-upcdntxa.bilivideo.com/OK"),
            "https://upos-cs-upcdntxa.bilivideo.com/OK"
        );
        assert_eq!(probe_url("http://127.0.0.1:1/OK"), "http://127.0.0.1:1/OK");
        assert_eq!(probe_url("https://a/OK"), "https://a/OK");
    }

    #[test]
    fn probe_post_bytes_follows_server_hint_with_sane_default() {
        assert_eq!(
            probe_post_bytes(&serde_json::json!({"post": 0.1})),
            (0.1 * 1024.0 * 1024.0) as usize
        );
        assert_eq!(
            probe_post_bytes(&serde_json::json!({"post": 2})),
            2 * 1024 * 1024
        );
        let fallback = (DEFAULT_PROBE_POST_MB * 1024.0 * 1024.0) as usize;
        assert_eq!(probe_post_bytes(&serde_json::json!({})), fallback);
        assert_eq!(probe_post_bytes(&serde_json::json!({"post": 0})), fallback);
        assert_eq!(probe_post_bytes(&serde_json::json!({"post": -1})), fallback);
    }

    /// 真实网络测速仍能选出可用线路。默认忽略，本地验证：
    /// `cargo test -p biliup probe_selects_line_over_network -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "requires network access to member.bilibili.com"]
    async fn probe_selects_line_over_network() {
        let client = reqwest::Client::new();
        let line = Probe::probe(&client).await.expect("测速应选出可用线路");
        assert_ne!(line.cost, u128::MAX);
        println!("selected line: {} cost={}ms", line.query, line.cost);
    }
}
