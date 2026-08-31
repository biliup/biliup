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
use std::time::Instant;
use tracing::{info, warn};

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
            .send()
            .await?
            .json()
            .await?;
        Self::select_line(client, &res.probe, res.lines).await
    }

    /// 逐条线路测速并选出耗时最短者。
    /// 单条线路网络错误或异常状态码只跳过该线路，全部失败才返回错误。
    async fn select_line(
        client: &reqwest::Client,
        probe: &serde_json::Value,
        lines: Vec<Line>,
    ) -> Result<Line> {
        let mut choice_line: Line = Default::default();
        for mut line in lines {
            let instant = Instant::now();
            match Probe::ping(probe, &format!("https:{}", line.probe_url), client)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    line.cost = instant.elapsed().as_millis();
                    info!("{}: {}", line.query, line.cost);
                    if choice_line.cost > line.cost {
                        choice_line = line
                    }
                }
                Ok(response) => {
                    warn!(
                        "{} 测速返回异常状态码 {}，跳过该线路",
                        line.query,
                        response.status()
                    );
                }
                Err(e) => {
                    warn!("{} 测速失败，跳过该线路: {e}", line.query);
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
            client
                .post(url)
                .body(vec![0; (1024. * 1024. * 10.) as usize]) // 10MB chunk
        }
    }
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
        let result = Probe::select_line(&client, &serde_json::json!({"get": {}}), Vec::new()).await;
        assert!(result.is_err());
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
