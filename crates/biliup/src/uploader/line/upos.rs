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
use crate::uploader::bilibili::Video;
use crate::{retry, retry_with_config};

pub struct Upos {
    client: StatelessClient,
    bucket: Bucket,
    url: String,
    upload_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UposPart {
    pub(crate) part_number: usize,
    e_tag: String,
}

impl Upos {
    pub async fn from(client: StatelessClient, bucket: Bucket) -> Result<Self> {
        if bucket.chunk_size == 0 {
            return Err(Kind::Custom(
                "UPOS chunk_size must be greater than zero".into(),
            ));
        }
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

    pub(crate) async fn upload_stream<'a, F, B>(
        &'a self,
        // file: std::fs::File,
        stream: F,
        total_size: u64,
        limit: usize,
    ) -> Result<impl Stream<Item = Result<(UposPart, usize)>> + 'a>
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
        let chunks_num = total_size.div_ceil(chunk_size as u64) as usize;
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

                Ok::<_, Kind>((
                    UposPart {
                        part_number: params.chunk + 1,
                        e_tag: "etag".to_string(),
                    },
                    len,
                ))
            })
            // `buffer_unordered(0)` 永远不拉取分块、也不唤醒，上传会无声挂死；
            // `--limit 0` / `limit=0` / `threads: 0` 都按 1 处理。
            .buffer_unordered(limit.max(1));
        Ok(stream)
    }

    /// 通知视频上传完成并获取视频信息
    pub(crate) async fn get_ret_video_info(
        &self,
        parts: &[UposPart],
        path: &Path,
    ) -> Result<Video> {
        let parts = sorted_parts(parts)?;

        // println!("{:?}", parts_cell.borrow());
        let url = reqwest::Url::parse_with_params(
            &self.url,
            [
                (
                    "name",
                    path.file_name().and_then(OsStr::to_str).unwrap_or_default(),
                ),
                ("uploadId", &self.upload_id),
                ("biz_id", &self.bucket.biz_id.to_string()),
                ("output", "json"),
                ("profile", "ugcupos/bup"),
            ],
        )
        .map_err(|e| Kind::Custom(e.to_string()))?;
        let res = retry_with_config(
            || async {
                let response = self
                    .client
                    .client_with_middleware
                    .post(url.clone())
                    .header(
                        "X-Upos-Auth",
                        header::HeaderValue::from_str(&self.bucket.auth)?,
                    )
                    .json(&json!({ "parts": parts }))
                    .timeout(Duration::from_secs(60))
                    .send()
                    .await?
                    .error_for_status()?;
                let value: serde_json::Value = response.json().await?;
                if value["OK"] != 1 {
                    return Err(Kind::Custom(value.to_string()));
                }
                Ok(value)
            },
            2,
            None::<fn(&Kind) -> bool>,
        )
        .await?;
        debug_assert_eq!(res["OK"], 1);
        uploaded_video(&self.bucket.upos_uri)
    }
}

fn uploaded_video(upos_uri: &str) -> Result<Video> {
    let filename = Path::new(upos_uri)
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|filename| !filename.is_empty())
        .ok_or_else(|| Kind::Custom("UPOS response contains no video filename".into()))?;
    // This is the uploaded object's key. Only the human-readable P title can be truncated.
    Ok(Video {
        title: None,
        filename: filename.to_string(),
        desc: "".into(),
    })
}

fn sorted_parts(parts: &[UposPart]) -> Result<Vec<UposPart>> {
    let mut parts = parts.to_vec();
    parts.sort_by_key(|part| part.part_number);
    for (index, part) in parts.iter().enumerate() {
        if part.part_number != index + 1 {
            return Err(Kind::Custom(format!(
                "UPOS parts 不连续：期望 partNumber={}，实际={}",
                index + 1,
                part.part_number
            )));
        }
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::{Bucket, Upos, UposPart, sorted_parts, uploaded_video};
    use crate::client::StatelessClient;
    use crate::error::Kind;
    use bytes::Bytes;
    use futures::TryStreamExt;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// 接受任意请求并返回 200 的假 UPOS，记录每个 PUT 的查询参数。
    async fn fake_upos() -> (Upos, Arc<Mutex<Vec<HashMap<String, String>>>>) {
        let puts = Arc::new(Mutex::new(Vec::new()));
        let recorded = puts.clone();
        let app =
            axum::Router::new().fallback(move |uri: axum::http::Uri, _body: axum::body::Bytes| {
                let recorded = recorded.clone();
                async move {
                    let query: HashMap<String, String> =
                        serde_urlencoded::from_str(uri.query().unwrap_or_default()).unwrap();
                    recorded.lock().unwrap().push(query);
                    ""
                }
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let upos = Upos {
            client: StatelessClient::default(),
            bucket: Bucket {
                chunk_size: 4,
                auth: "auth".into(),
                endpoint: format!("//{addr}"),
                biz_id: 1,
                upos_uri: "upos://bucket/file.mp4".into(),
            },
            url: format!("http://{addr}/bucket/file.mp4"),
            upload_id: "upload-id".into(),
        };
        (upos, puts)
    }

    /// 并发上限为 0（CLI `--limit 0`、stream-gears `limit=0`、服务端 `threads: 0`）时
    /// 也必须把每个分块传完，而不是永远挂起。
    #[tokio::test]
    async fn zero_concurrency_limit_still_uploads_every_chunk() {
        let (upos, puts) = fake_upos().await;
        let chunks = vec![
            Ok::<_, Kind>((Bytes::from_static(b"abcd"), 4)),
            Ok((Bytes::from_static(b"ef"), 2)),
        ];

        let uploaded = tokio::time::timeout(Duration::from_secs(20), async {
            upos.upload_stream(futures::stream::iter(chunks), 6, 0)
                .await?
                .try_collect::<Vec<_>>()
                .await
        })
        .await
        .expect("upload with limit 0 hung")
        .unwrap();

        let mut numbers: Vec<_> = uploaded
            .iter()
            .map(|(part, len)| (part.part_number, *len))
            .collect();
        numbers.sort();
        assert_eq!(numbers, [(1, 4), (2, 2)]);

        let mut puts = puts.lock().unwrap().clone();
        puts.sort_by_key(|query| query["partNumber"].clone());
        let ranges: Vec<_> = puts
            .iter()
            .map(|q| {
                (
                    q["partNumber"].as_str(),
                    q["start"].as_str(),
                    q["end"].as_str(),
                    q["chunks"].as_str(),
                    q["total"].as_str(),
                )
            })
            .collect();
        assert_eq!(
            ranges,
            [("1", "0", "4", "2", "6"), ("2", "4", "6", "2", "6")]
        );
    }

    fn part(part_number: usize) -> UposPart {
        UposPart {
            part_number,
            e_tag: "etag".to_string(),
        }
    }

    #[test]
    fn complete_parts_are_sorted_by_part_number() {
        let parts = sorted_parts(&[part(3), part(1), part(2)]).unwrap();
        assert_eq!(
            parts
                .into_iter()
                .map(|part| part.part_number)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn complete_parts_must_be_contiguous() {
        assert!(sorted_parts(&[part(1), part(3)]).is_err());
        assert!(sorted_parts(&[part(1), part(1)]).is_err());
    }

    #[test]
    fn uploaded_object_key_is_preserved_even_when_longer_than_a_title() {
        let key = "a".repeat(120);
        let video = uploaded_video(&format!("upos://bucket/{key}.mp4")).unwrap();
        assert_eq!(video.filename, key);
        assert!(video.title.is_none());
        assert!(uploaded_video("").is_err());
    }

    #[tokio::test]
    async fn zero_chunk_size_is_rejected_before_requesting_an_upload_id() {
        let bucket = Bucket {
            chunk_size: 0,
            auth: "auth".into(),
            endpoint: "//127.0.0.1:1".into(),
            biz_id: 1,
            upos_uri: "upos://bucket/file.mp4".into(),
        };
        let error = Upos::from(StatelessClient::default(), bucket)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("chunk_size"));
    }
}
