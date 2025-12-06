use crate::error::{Kind, Result};
use futures::{Stream, StreamExt, TryStreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_LENGTH};
use reqwest::{header, Body};
use reqwest_middleware::ClientWithMiddleware;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use std::path::Path;

use crate::client::StatelessClient;
use crate::retry;
use crate::uploader::bilibili::Video;

pub struct Cos {
    client: StatelessClient,
    bucket: Bucket,
    upload_id: String,
}

impl Cos {
    pub async fn form_post(client: StatelessClient, bucket: Bucket) -> Result<Cos> {
        let upload_id = get_uploadid(&client.client_with_middleware, &bucket).await?;
        Ok(Cos {
            client,
            bucket,
            upload_id,
        })
    }

    pub async fn upload_stream<F, B>(
        &self,
        stream: F,
        total_size: u64,
        limit: usize,
        enable_internal: bool,
    ) -> Result<Vec<(usize, String)>>
    where
        F: Stream<Item = Result<(B, usize)>>,
        B: Into<Body> + Clone, // Body: From<B>
    {
        let chunk_size = 10485760;
        let _chunks_num = (total_size as f64 / chunk_size as f64).ceil() as u32; // 获取分块数量

        let client = &self.client.client;
        let temp;
        let url = if enable_internal {
            temp = self
                .bucket
                .url
                .replace("cos.accelerate", "cos-internal.ap-shanghai");
            &temp
        } else {
            &self.bucket.url
        };
        let upload_id = &self.upload_id;
        let stream = stream
            // let mut chunks = read_chunk(file, chunk_size)
            .enumerate()
            .map(move |(i, chunk)| async move {
                let (chunk, len) = chunk?;
                // let len = chunk.len();
                // println!("{}", len);
                let params = Protocol {
                    upload_id,
                    part_number: (i + 1) as u32,
                };
                let response = retry(|| async {
                    let response = client
                        .put(url)
                        .header(AUTHORIZATION, &self.bucket.put_auth)
                        .header(CONTENT_LENGTH, len)
                        .query(&params)
                        .body(chunk.clone())
                        .send()
                        .await?;
                    response.error_for_status_ref()?;
                    Ok::<_, reqwest::Error>(response)
                })
                .await?;

                // json!({"partNumber": i + 1, "eTag": response.headers().get("Etag")})
                let headers = response.headers();
                let etag = match headers.get("Etag") {
                    None => {
                        return Err(Kind::Custom(format!(
                            "upload chunk {i} error: {}",
                            response.text().await?
                        )))
                    }
                    Some(etag) => etag
                        .to_str()
                        .map_err(|e| Kind::Custom(e.to_string()))?
                        .to_string(),
                };
                // etag.ok_or(anyhow!("{res}")).map(|s|s.to_str())??.to_string()
                // let res = response.text().await?;
                Ok::<_, Kind>((i + 1, etag))
            })
            .buffer_unordered(limit);
        let mut parts = Vec::new();
        tokio::pin!(stream);
        while let Some((part, etag)) = stream.try_next().await? {
            parts.push((part, etag));
        }
        Ok(parts)
    }

    pub async fn merge_files(&self, mut parts: Vec<(usize, String)>) -> Result<Video> {
        parts.sort_unstable_by_key(|annotate| annotate.0);
        // let complete_multipart_upload
        let complete_multipart_upload = parts
            .iter()
            .map(|(number, etag)| {
                format!(
                    r#"<Part>
                        <PartNumber>{number}</PartNumber>
                        <ETag>{etag}</ETag>
                       </Part>"#
                )
            })
            .reduce(|accum, item| accum + &item)
            .unwrap();
        let xml = format!(
            r#"<CompleteMultipartUpload>{complete_multipart_upload}</CompleteMultipartUpload>"#
        );
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&self.bucket.post_auth)?,
        );
        let response = self
            .client
            .client_with_middleware
            .post(&self.bucket.url)
            .query(&[("uploadId", &self.upload_id)])
            .body(xml)
            .headers(headers)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Kind::Custom(response.text().await?));
        }
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "X-Upos-Fetch-Source",
            header::HeaderValue::from_str(
                self.bucket
                    .fetch_headers
                    .get("X-Upos-Fetch-Source")
                    .unwrap(),
            )?,
        );
        headers.insert(
            "X-Upos-Auth",
            header::HeaderValue::from_str(self.bucket.fetch_headers.get("X-Upos-Auth").unwrap())?,
        );
        headers.insert(
            "Fetch-Header-Authorization",
            header::HeaderValue::from_str(
                self.bucket
                    .fetch_headers
                    .get("Fetch-Header-Authorization")
                    .unwrap(),
            )?,
        );
        let res = self
            .client
            .client_with_middleware
            .post(format!("https:{}", self.bucket.fetch_url))
            .headers(headers)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Kind::Custom(res.text().await?));
        }
        let filename = Path::new(&self.bucket.bili_filename)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();

        // B站限制分P视频标题不能超过80字符，需要截断filename字段
        let truncated_filename = if filename.chars().count() >= 80 {
            Video::truncate_title(filename, 80)
        } else {
            filename.to_string()
        };

        Ok(Video {
            title: None,
            filename: truncated_filename,
            desc: "".into(),
        })
    }
}

async fn get_uploadid(client: &ClientWithMiddleware, bucket: &Bucket) -> Result<String> {
    let res = client
        .post(format!("{}?uploads&output=json", bucket.url))
        .header(reqwest::header::AUTHORIZATION, &bucket.post_auth)
        .send()
        .await?
        .text()
        .await?;
    let start = res
        .find(r"<UploadId>")
        .ok_or_else(|| Kind::Custom(res.clone()))?
        + "<UploadId>".len();
    let end = res
        .rfind(r"</UploadId>")
        .ok_or_else(|| Kind::Custom(res.clone()))?;
    let uploadid = &res[start..end];
    Ok(uploadid.to_string())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bucket {
    #[serde(rename = "OK")]
    ok: u8,
    bili_filename: String,
    biz_id: usize,
    fetch_headers: HashMap<String, String>,
    fetch_url: String,
    fetch_urls: Vec<String>,
    post_auth: String,
    put_auth: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Protocol<'a> {
    upload_id: &'a str,
    part_number: u32,
}
