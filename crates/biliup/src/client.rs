use crate::{ReqwestClientBuilderExt, retry};
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::{Response, header};
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StatelessClient {
    pub client: reqwest::Client,
    pub client_with_middleware: ClientWithMiddleware,
    pub headers: HeaderMap,
}

impl StatelessClient {
    pub fn new(headers: HeaderMap, proxy: Option<&str>) -> Self {
        let client = reqwest::Client::proxy_builder(proxy)
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:60.1) Gecko/20100101 Firefox/60.1")
            .default_headers(headers)
            // .timeout(Duration::new(60, 0))
            .connect_timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
        let client_with_middleware = ClientBuilder::new(client.clone())
            // Retry failed requests.
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            client,
            client_with_middleware,
            headers: HeaderMap::new(),
        }
    }

    pub async fn retryable(&self, url: &str) -> reqwest::Result<Response> {
        let resp = retry(|| {
            self.client
                .get(url)
                .headers(self.headers.clone())
                // .timeout(Duration::MAX)
                // .header(ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                // .header(ACCEPT_ENCODING, "gzip, deflate")
                // .header(ACCEPT_LANGUAGE, "zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3")
                // .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1")
                // .headers(headers.clone())
                .send()
        })
        .await?;
        resp.error_for_status_ref()?;
        Ok(resp)
    }
}

#[derive(Debug)]
pub struct StatefulClient {
    pub client: reqwest::Client,
    pub cookie_store: Arc<CookieStoreMutex>,
    pub buvid: String,
}

impl StatefulClient {
    pub fn new(headers: HeaderMap, proxy: Option<&str>) -> Self {
        let cookie_store = reqwest_cookie_store::CookieStore::default();
        let cookie_store = CookieStoreMutex::new(cookie_store);
        let cookie_store = Arc::new(cookie_store);
        StatefulClient {
            client: reqwest::Client::proxy_builder(proxy)
                .cookie_provider(std::sync::Arc::clone(&cookie_store))
                .user_agent(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/63.0.3239.108",
                )
                .default_headers(headers)
                .connect_timeout(Duration::from_secs(60))
                // .timeout(Duration::new(60, 0))
                .build()
                .unwrap(),
            cookie_store,
            buvid: generate_buvid(),
        }
    }
}

impl Default for StatelessClient {
    fn default() -> Self {
        Self::new(header::HeaderMap::new(), None)
    }
}

// ref: https://github.com/SocialSisterYi/bilibili-API-collect
fn generate_buvid() -> String {
    let mut rng = rand::thread_rng();

    let dummy_md5 = (0..32).map(|_| rng.gen_range(0..0xf)).collect::<Vec<u8>>();
    let prefix = [2, 12, 22].map(|i| dummy_md5[i]);

    let hash_string = [prefix.as_slice(), dummy_md5.as_slice()]
        .map(|array| {
            array
                .iter()
                .map(|n| format!("{n:X}"))
                .collect::<Vec<_>>()
                .join("")
        })
        .join("");

    format!("Y{hash_string}")
}
