use futures::Stream;
use rand::distributions::uniform::{UniformFloat, UniformSampler};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
pub mod client;
pub mod downloader;
pub mod error;
pub mod uploader;

pub use uploader::bilibili;
pub use uploader::credential;

pub async fn retry<F, Fut, O, E: std::fmt::Display>(mut f: F) -> Result<O, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<O, E>>,
{
    retry_with_config(f, 3, None::<fn(&E) -> bool>).await
}

pub async fn retry_with_config<F, Fut, O, E, P>(
    mut f: F,
    max_retries: usize,
    should_retry: Option<P>,
) -> Result<O, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<O, E>>,
    E: std::fmt::Display,
    P: Fn(&E) -> bool,
{
    let mut retries = max_retries;
    let mut wait = 1;
    let mut jittered_wait_for;
    loop {
        match f().await {
            Err(e) if retries > 0 => {
                // 如果提供了 should_retry 条件，检查是否应该重试
                if let Some(ref predicate) = should_retry {
                    if !predicate(&e) {
                        break Err(e);
                    }
                }

                retries -= 1;
                let jitter_factor =
                    UniformFloat::<f64>::sample_single(0., 1., &mut rand::thread_rng());
                wait *= 2;

                jittered_wait_for = f64::min(jitter_factor + (wait as f64), 64.);
                info!(
                    "Retry attempt #{}. Sleeping {:?} before the next attempt. {e}",
                    max_retries - retries,
                    jittered_wait_for
                );
                sleep(Duration::from_secs_f64(jittered_wait_for)).await;
            }
            res => break res,
        }
    }
}

trait ReqwestClientBuilderExt {
    fn proxy_builder<U: reqwest::IntoUrl>(proxy: Option<U>) -> reqwest::ClientBuilder;
}

impl ReqwestClientBuilderExt for reqwest::Client {
    fn proxy_builder<U: reqwest::IntoUrl>(proxy: Option<U>) -> reqwest::ClientBuilder {
        match proxy {
            Some(proxy) => {
                tracing::debug!("使用代理: {}", proxy.as_str());
                Self::builder().proxy(reqwest::Proxy::all(proxy).unwrap())
            }
            None => Self::builder(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::uploader::bilibili::Vid;
    use std::str::FromStr;
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;
    use url::Url;

    #[test]
    fn it_works() {
        assert_eq!(Ok(Vid::Aid(10)), Vid::from_str("10"));
        assert_eq!(Ok(Vid::Aid(971158452)), Vid::from_str("971158452"));
        assert_eq!(Ok(Vid::Aid(971158452)), Vid::from_str("av971158452"));
        assert_eq!(
            Ok(Vid::Bvid("BV1ip4y1x7Gi".into())),
            Vid::from_str("BV1ip4y1x7Gi")
        );
    }

    #[tokio::test]
    async fn try_clone_stream() {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
        Url::parse("https://bilibili.com/").unwrap();
        let chunks: Vec<Result<_, ::std::io::Error>> = vec![Ok("hello"), Ok(" "), Ok("world")];
        let _stream = futures::stream::iter(chunks);
    }
}
