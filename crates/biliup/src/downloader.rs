use crate::downloader::httpflv::Connection;
use flv_parser::header;
use nom::Err;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::downloader::util::{LifecycleFile, Segmentable};

use crate::client::StatelessClient;
use crate::downloader::extractor::CallbackFn;
use std::str::FromStr;

pub mod error;
pub mod extractor;
pub mod flv_parser;
pub mod flv_writer;
mod hls;
pub mod httpflv;
pub mod util;

#[tokio::main]
pub async fn download(
    url: &str,
    headers: HeaderMap,
    file_name: &str,
    segment: Segmentable,
    file_name_hook: Option<CallbackFn>,
    proxy: Option<&str>,
) -> anyhow::Result<()> {
    let client = StatelessClient::new(headers, proxy);
    let response = client.retryable(url).await?;
    let mut connection = Connection::new(response);
    // let buf = &mut [0u8; 9];
    let bytes = connection.read_frame(9).await?;
    // response.read_exact(buf)?;
    // let out = File::create(format!("{}.flv", file_name)).expect("Unable to create file.");
    // let mut writer = BufWriter::new(out);
    // let mut buf = [0u8; 8 * 1024];
    // response.copy_to(&mut writer)?;
    // io::copy(&mut resp, &mut out).expect("Unable to copy the content.");
    match header(&bytes) {
        Ok((_i, header)) => {
            debug!("header: {header:#?}");
            info!("Downloading {}...", url);
            let file = LifecycleFile::new(file_name, "flv", file_name_hook);
            httpflv::download(connection, file, segment).await;
        }
        Err(Err::Incomplete(needed)) => {
            error!("needed: {needed:?}")
        }
        Err(e) => {
            error!("{e}");
            let file = LifecycleFile::new(file_name, "ts", file_name_hook);
            hls::download(url, &client, file, segment).await?;
        }
    }
    Ok(())
}

pub fn construct_headers(hash_map: HashMap<String, String>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (key, value) in hash_map.iter() {
        headers.insert(
            HeaderName::from_str(key).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    headers
}

// fn retry<O, E: std::fmt::Display>(mut f: impl FnMut() -> Result<O, E>) -> Result<O, E> {
//     let mut retries = 0;
//     let mut wait = 1;
//     loop {
//         match f() {
//             Err(e) if retries < 3 => {
//                 retries += 1;
//                 println!(
//                     "Retry attempt #{}. Sleeping {wait}s before the next attempt. {e}",
//                     retries,
//                 );
//                 sleep(Duration::from_secs(wait));
//                 wait *= 2;
//             }
//             res => break res,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use crate::downloader::download;
    use crate::downloader::util::Segmentable;
    use anyhow::Result;
    use reqwest::header::{HeaderMap, HeaderValue, REFERER};

    #[test]
    #[ignore]
    fn it_works() -> Result<()> {
        tracing_subscriber::fmt::init();

        let mut headers = HeaderMap::new();
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://live.bilibili.com"),
        );
        download(
            "",
            headers,
            "testdouyu%Y-%m-%dT%H_%M_%S",
            // Segment::Size(20 * 1024 * 1024, 0),
            Segmentable::new(Some(std::time::Duration::from_secs(6000)), None),
            None,
            None,
        )?;
        Ok(())
    }
}
