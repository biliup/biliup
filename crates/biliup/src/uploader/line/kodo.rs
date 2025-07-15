use crate::error::{Kind, Result};
use base64::{engine::general_purpose, Engine as _};
use futures::{Stream, StreamExt, TryStreamExt};
use reqwest::header::{HeaderMap, HeaderName, CONTENT_LENGTH};
use reqwest::{header, Body};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::str::FromStr;

use crate::client::StatelessClient;
use crate::retry;
use crate::uploader::bilibili::Video;

pub struct Kodo {
    client: StatelessClient,
    bucket: Bucket,
    url: String,
}

impl Kodo {
    pub async fn from(client: StatelessClient, bucket: Bucket) -> Result<Self> {
        let url = format!("https:{}/mkblk", bucket.endpoint); // 视频上传路径
        Ok(Kodo {
            client,
            bucket,
            url,
        })
    }

    pub async fn upload_stream<F, B>(
        self,
        // file: std::fs::File,
        stream: F,
        total_size: u64,
        limit: usize,
        // mut process: impl FnMut(usize) -> bool,
    ) -> Result<Video>
    where
        F: Stream<Item = Result<(B, usize)>>,
        B: Into<Body> + Clone,
    {
        // let total_size = file.metadata()?.len();
        let _chunk_size = 4194304;
        let mut parts = Vec::new();
        // let parts_cell = &RefCell::new(parts);
        let client = &self.client.client;
        let url = &self.url;
        let uptoken = &format!("UpToken {}", &self.bucket.uptoken);
        // let stream = read_chunk(file, chunk_size, process)
        let stream = stream
            // let mut chunks = read_chunk(file, chunk_size)
            .enumerate()
            .map(|(i, chunk)| async move {
                let (chunk, len) = chunk?;
                // let len = chunk.len();
                // println!("{}", len);
                let ctx: serde_json::Value = retry(|| async {
                    let url = format!("{url}/{len}");
                    let response = client
                        .post(url)
                        .header(CONTENT_LENGTH, len)
                        .header("Authorization", header::HeaderValue::try_from(uptoken)?)
                        .body(chunk.clone())
                        .send()
                        .await?;
                    response.error_for_status_ref()?;
                    let res = response.json().await?;
                    Ok::<_, Kind>(res)
                })
                .await?;

                Ok::<_, Kind>((
                    Ctx {
                        index: i,
                        ctx: ctx["ctx"].as_str().unwrap_or_default().into(),
                    },
                    len,
                ))
            })
            .buffer_unordered(limit);
        tokio::pin!(stream);
        while let Some((part, _size)) = stream.try_next().await? {
            parts.push(part);
        }
        parts.sort_by_key(|x| x.index);
        let key = general_purpose::URL_SAFE_NO_PAD.encode(self.bucket.key);
        self.client
            .client_with_middleware
            .post(format!(
                "https:{}/mkfile/{total_size}/key/{key}",
                self.bucket.endpoint,
            ))
            .header("Authorization", header::HeaderValue::try_from(uptoken)?)
            .body(
                parts
                    .iter()
                    .map(|x| &x.ctx[..])
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .send()
            .await?
            .error_for_status_ref()?;
        let mut headers = HeaderMap::new();
        for (key, value) in self.bucket.fetch_headers {
            headers.insert(HeaderName::from_str(&key)?, value.parse()?);
        }
        // reqwest::header::HeaderName::
        let result: serde_json::Value = self
            .client
            .client_with_middleware
            .post(format!("https:{}", self.bucket.fetch_url))
            .headers(headers)
            .send()
            .await?
            .json()
            .await?;
        Ok(match result.get("OK") {
            Some(x) if x.as_i64().ok_or("kodo fetch err")? != 1 => {
                return Err(Kind::Custom(result.to_string()));
            }
            _ => Video {
                title: None,
                filename: self.bucket.bili_filename,
                desc: "".into(),
            },
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Ctx {
    index: usize,
    ctx: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bucket {
    bili_filename: String,
    fetch_url: String,
    endpoint: String,
    uptoken: String,
    key: String,
    fetch_headers: HashMap<String, String>,
}
