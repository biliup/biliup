use crate::error::Result;
use crate::uploader::{Uploader, VideoFile, VideoStream};
use futures::{Stream, TryStreamExt};
use reqwest::{Body, RequestBuilder};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::ffi::OsStr;

use crate::client::StatelessClient;
use crate::error::Kind::Custom;
use crate::uploader::bilibili::{BiliBili, Video};
// use crate::uploader::line::cos::Cos;
// use crate::uploader::line::kodo::Kodo;
use crate::uploader::line::upos::Upos;
use std::time::Instant;
use tracing::info;

// pub mod cos;
// pub mod kodo;
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
            // Bucket::Cos(bucket, enable_internal) => {
            //     // let bucket = self.pre_upload(client).await?;
            //     let cos_client = Cos::form_post(client, bucket).await?;
            //     let chunk_size = 10485760;
            //     let parts = cos_client
            //         .upload_stream(
            //             progress(self.video_file.get_stream(chunk_size)?),
            //             self.video_file.total_size,
            //             limit,
            //             enable_internal,
            //         )
            //         .await?;
            //     cos_client.merge_files(parts).await?
            // }
            // Bucket::Kodo(bucket) => {
            //     // let bucket = self.pre_upload(client).await?;
            //     let chunk_size = 4194304;
            //     Kodo::from(client, bucket)
            //         .await?
            //         .upload_stream(
            //             progress(self.video_file.get_stream(chunk_size)?),
            //             self.video_file.total_size,
            //             limit,
            //         )
            //         .await?
            // }
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
            video.title = self
                .video_file
                .filepath
                .file_stem()
                .and_then(OsStr::to_str)
                .map(|s| s.to_string())
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
                .body(vec![0; (1024. * 0.1 * 1024.) as usize])
        }
    }
}

enum Bucket {
    // Cos(cos::Bucket, bool),
    // Kodo(kodo::Bucket),
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
        // let profile = if let Uploader::Upos = self.os {
        //     "ugcupos/bup"
        // } else {
        //     "ugcupos/bupfetch"
        // };
        let profile = "ugcupos/bup";
        let params = json!({
            "r": self.os,
            "profile": profile,
            "ssl": 0,
            "version": "2.11.0",
            "build": 2110000,
            "name": file_name,
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
            return Err(Custom(format!(
                "Failed to pre_upload from {}",
                response.text().await?
            )));
        }
        match self.os {
            Uploader::Upos => Ok(Parcel {
                line: Bucket::Upos(response.json().await?),
                video_file,
            }),
            // Uploader::Kodo => Ok(Parcel {
            //     line: Bucket::Kodo(response.json().await?),
            //     video_file,
            // }),
            // Uploader::Bos | Uploader::Gcs => {
            //     panic!("unsupported")
            // }
            // Uploader::Cos => Ok(Parcel {
            //     line: Bucket::Cos(response.json().await?, self.probe_url == "internal"),
            //     video_file,
            // }),
            // _ => {
            //     panic!("unsupported")
            // }
        }
    }
}

impl Default for Line {
    fn default() -> Self {
        Line {
            os: Uploader::Upos,
            probe_url: "//upos-cs-upcdnbda2.bilivideo.com/OK".to_string(),
            query: "probe_version=20221109&upcdn=bda2&zone=cs".to_string(),
            cost: u128::MAX,
        }
    }
}

// pub fn kodo() -> Line {
//     Line {
//         os: Uploader::Kodo,
//         query: "bucket=bvcupcdnkodobm&probe_version=20211012".into(),
//         probe_url: "//up-na0.qbox.me/crossdomain.xml".into(),
//         cost: 0,
//     }
// }

pub fn bda2() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=bda2&zone=cs".into(),
        probe_url: "//upos-cs-upcdnbda2.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn ws() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=ws&zone=cs".into(),
        probe_url: "//upos-cs-upcdnws.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn qn() -> Line {
    Line {
        os: Uploader::Upos,
        query: "probe_version=20221109&upcdn=qn&zone=cs".into(),
        probe_url: "//upos-cs-upcdnqn.bilivideo.com/OK".into(),
        cost: 0,
    }
}

// pub fn cos() -> Line {
//     Line {
//         os: Uploader::Cos,
//         query: "&probe_version=20211012&r=cos&profile=ugcupos%2Fbupfetch&ssl=0&version=2.10.4.0&build=2100400&webVersion=2.0.0".into(),
//         probe_url: "".into(),
//         cost: 0,
//     }
// }

// pub fn cos_internal() -> Line {
//     Line {
//         os: Uploader::Cos,
//         query: "".into(),
//         probe_url: "internal".into(),
//         cost: 0,
//     }
// }

pub fn bldsa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=bldsa&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnbldsa.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn tx() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=tx&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdntx.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn txa() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=txa&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdntxa.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn bda() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=bda&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnbda.bilivideo.com/OK".into(),
        cost: 0,
    }
}

pub fn alia() -> Line {
    Line {
        os: Uploader::Upos,
        query: "zone=cs&upcdn=alia&probe_version=20221109".into(),
        probe_url: "//upos-cs-upcdnalia.bilivideo.com/OK".into(),
        cost: 0,
    }
}
