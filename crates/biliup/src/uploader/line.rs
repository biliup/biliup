use crate::error::Result;
use crate::uploader::{Uploader, VideoFile, VideoStream};
use futures::{Stream, TryStreamExt};
use reqwest::{Body, RequestBuilder};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::ffi::OsStr;

use crate::client::StatelessClient;
use crate::error::Kind::{Custom, RateLimit};
use crate::uploader::bilibili::{BiliBili, Video};
use crate::uploader::line::upos::Upos;
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

        if video.title.is_none() {
            if let Some(filename) = self
                .video_file
                .filepath
                .file_stem()
                .and_then(OsStr::to_str)
            {
                // B站限制分P视频标题不能超过80字符，需要截断
                video.title = Some(if filename.chars().count() >= 80 {
                    Video::truncate_title(filename, 80)
                } else {
                    filename.to_string()
                });
            }
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
        // let client = res.ping(client);
        let mut choice_line: Line = Default::default();
        for mut line in res.lines {
            let instant = Instant::now();
            if Probe::ping(&res.probe, &format!("https:{}", line.probe_url), client)
                .send()
                .await?
                .status()
                .is_success()
            {
                line.cost = instant.elapsed().as_millis();
                info!("{}: {}", line.query, line.cost);
                if choice_line.cost > line.cost {
                    choice_line = line
                }
            };
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

impl Line {
    pub async fn pre_upload(&self, bili: &BiliBili, video_file: VideoFile) -> Result<Parcel> {
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let profile = "ugcupos/bup"; // ugcfx/bup 需上传视频metadata和frame.zip
        let params = json!({
            // "probe_version": "20221109",
            // "upcdn": "",
            // "zone": "",
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
            if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                if let Some(code) = error_json.get("code").and_then(|c| c.as_i64()) {
                    if code == 601 {
                        let message = error_json
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("上传过快")
                            .to_string();
                        // 直接返回限流错误，让调用方决定如何处理
                        return Err(RateLimit { code, message });
                    }
                }
            }

            return Err(Custom(format!(
                "Failed to pre_upload from {}",
                response_text
            )));
        }

        match self.os {
            Uploader::Upos => Ok(Parcel {
                line: Bucket::Upos(response.json().await?),
                video_file,
            }),
            // _ => {
            //     panic!("unsupported")
            // }
        }
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

/// 百度云海外（Cloudflare）
pub fn bda() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=bda&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnbda.bilivideo.com/OK".into(),
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
