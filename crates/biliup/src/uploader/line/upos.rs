use crate::error::{Kind, Result};
use futures::Stream;
use futures::StreamExt;

use reqwest::header::CONTENT_LENGTH;
use reqwest::{Body, header};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use crate::client::StatelessClient;
use crate::retry;
use crate::uploader::bilibili::Video;

pub struct Upos {
    client: StatelessClient,
    bucket: Bucket,
    url: String,
    upload_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bucket {
    pub chunk_size: usize,
    auth: String,
    endpoint: String,
    biz_id: usize,
    upos_uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Protocol<'a> {
    upload_id: &'a str,
    chunks: usize,
    total: u64,
    chunk: usize,
    size: usize,
    part_number: usize,
    start: u64,
    end: u64,
}

impl Upos {
    pub async fn from(client: StatelessClient, bucket: Bucket) -> Result<Self> {
        let url = format!(
            "https:{}/{}",
            bucket.endpoint,
            bucket.upos_uri.replace("upos://", "")
        ); // 视频上传路径
        let upload_id: serde_json::Value = client
            .client_with_middleware
            .post(format!("{url}?uploads&output=json"))
            .header("X-Upos-Auth", header::HeaderValue::from_str(&bucket.auth)?)
            .timeout(Duration::from_secs(60))
            .send()
            .await?
            .json()
            .await?;
        let upload_id = upload_id
            .get("upload_id")
            .and_then(|s| s.as_str())
            .ok_or_else(|| Kind::Custom(upload_id.to_string()))?
            .into();
        // = upload_id["upload_id"].as_str().unwrap().into();
        // let ret =  &upload.ret;
        // let chunk_size = ret["chunk_size"].as_u64().unwrap() as usize;
        // let auth = ret["auth"].as_str().unwrap();
        // let endpoint = ret["endpoint"].as_str().unwrap();
        // let biz_id = &ret["biz_id"];
        // let upos_uri = ret["upos_uri"].as_str().unwrap();
        Ok(Upos {
            client,
            bucket,
            url,
            upload_id,
        })
    }

    pub async fn upload_stream<'a, F, B>(
        &'a self,
        // file: std::fs::File,
        stream: F,
        total_size: u64,
        limit: usize,
    ) -> Result<impl Stream<Item = Result<(serde_json::Value, usize)>> + 'a>
    where
        F: Stream<Item = Result<(B, usize)>> + 'a,
        B: Into<Body> + Clone,
    {
        // let mut parts = Vec::new();

        // let total_size = file.metadata()?.len();
        // let parts = Vec::new();
        // let parts_cell = &RefCell::new(parts);
        let chunk_size = self.bucket.chunk_size;
        // 获取分块数量
        let chunks_num = (total_size as f64 / chunk_size as f64).ceil() as usize;
        // let file = tokio::io::BufReader::with_capacity(chunk_size, file);
        let client = &self.client.client;
        let url = &self.url;
        let upload_id = &*self.upload_id;
        let stream = stream
            // let mut chunks = read_chunk(file, chunk_size)
            .enumerate()
            .map(move |(i, chunk)| async move {
                let (chunk, len) = chunk?;
                // let len = chunk.len();
                // println!("{}", len);
                let params = Protocol {
                    upload_id,
                    chunks: chunks_num,
                    total: total_size,
                    chunk: i,
                    size: len,
                    part_number: i + 1,
                    start: i as u64 * chunk_size as u64,
                    end: i as u64 * chunk_size as u64 + len as u64,
                };
                retry(|| async {
                    let response = client
                        .put(url)
                        .header(
                            "X-Upos-Auth",
                            header::HeaderValue::from_str(&self.bucket.auth)?,
                        )
                        .query(&params)
                        .timeout(Duration::from_secs(240))
                        .header(CONTENT_LENGTH, len)
                        .body(chunk.clone())
                        .send()
                        .await?;
                    response.error_for_status()?;
                    Ok::<_, Kind>(())
                })
                .await?;

                Ok::<_, Kind>((json!({"partNumber": params.chunk + 1, "eTag": "etag"}), len))
            })
            .buffer_unordered(limit);
        Ok(stream)
    }

    /// 通知视频上传完成并获取视频信息
    pub(crate) async fn get_ret_video_info(
        &self,
        parts: &[serde_json::Value],
        path: &Path,
    ) -> Result<Video> {
        // println!("{:?}", parts_cell.borrow());
        let value = json!({
            "name": path.file_name().and_then(OsStr::to_str),
            "uploadId": self.upload_id,
            "biz_id": self.bucket.biz_id,
            "output": "json",
            "profile": "ugcupos/bup"
        });
        // let res: serde_json::Value = self.client.post(url).query(&value).json(&json!({"parts": *parts_cell.borrow()}))
        let res: serde_json::Value = self
            .client
            .client_with_middleware
            .post(&self.url)
            .header(
                "X-Upos-Auth",
                header::HeaderValue::from_str(&self.bucket.auth)?,
            )
            .query(&value)
            .json(&json!({ "parts": parts }))
            .timeout(Duration::from_secs(60))
            .send()
            .await?
            .json()
            .await?;
        if res["OK"] != 1 {
            return Err(Kind::Custom(res.to_string()));
        }
        Ok(Video {
            title: None,
            filename: Path::new(&self.bucket.upos_uri)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            desc: "".into(),
        })
    }
}
