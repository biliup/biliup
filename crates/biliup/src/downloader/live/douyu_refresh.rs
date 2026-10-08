//! Douyu passport safeAuth exchange. Protocol verified against bililive-go
//! c5a96e2818ca25d6046ea48812d5e193ecc394fc (refresh.go, auth.go) and
//! douyu-keep-just-works e1290ed111768d068f02c07ed616f42e4562c3a4
//! (src/core/douyu-passport.ts). Keep transport isolated from recording clients.

use super::{DOUYU_USER_AGENT, cookie};
use reqwest::{Client, Response, StatusCode, header};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

const SAFE_AUTH_URL: &str = "https://passport.douyu.com/lapi/passport/iframe/safeAuth";
const PROBE_URL: &str = "https://www.douyu.com/wgapi/livenc/liveweb/follow/top3";
const REQUIRED_KEYS: [&str; 6] = [
    "acf_uid",
    "acf_auth",
    "acf_stk",
    "acf_ltkid",
    "acf_biz",
    "acf_ct",
];
const MAX_REDIRECTS: usize = 8;
const MAX_BODY_BYTES: usize = 128 * 1024;
const DEVICE_KEYS: [&str; 3] = ["dy_did", "acf_did", "acf_devid"];

/// Safe to display or log: errors contain no response body, URL, or credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DouyuRefreshError {
    #[error("斗鱼 Cookie 格式错误或包含冲突字段")]
    InvalidCookie,
    #[error("自动续期需要同一次登录取得的 LTP0 和 dy_did")]
    MissingCredential,
    #[error("斗鱼登录凭据已失效，请重新登录并更新 Cookie")]
    LoginInvalid,
    #[error("斗鱼续期账号与当前账号不一致，请重新导出同一账号的凭据")]
    AccountMismatch,
    #[error("斗鱼续期网络请求失败，请稍后重试")]
    Network,
    #[error("斗鱼续期接口暂时不可用，请稍后重试")]
    Http,
    #[error("斗鱼续期接口返回了无法确认的响应，请稍后重试")]
    Protocol,
    #[error("斗鱼续期接口返回了不安全的跳转地址")]
    UnsafeRedirect,
}

impl DouyuRefreshError {
    pub fn is_login_invalid(self) -> bool {
        matches!(self, Self::LoginInvalid | Self::AccountMismatch)
    }
}

/// An import may contain both Web and passport cookies. These must be persisted
/// separately: LTP0 must not enter normal Web API request headers.
#[derive(Clone)]
pub struct DouyuCookieInput {
    pub cookie: String,
    pub ltp0: Option<String>,
    pub dy_did: Option<String>,
    pub account_id: Option<String>,
}

impl DouyuCookieInput {
    pub fn parse(input: &str) -> Result<Self, DouyuRefreshError> {
        let (mut web, passport) =
            cookie::parse_cookie_scopes(input).map_err(|_| DouyuRefreshError::InvalidCookie)?;
        let web_ticket = web.remove("LTP0").filter(|v| !v.is_empty());
        let passport_ticket = passport.get("LTP0").filter(|v| !v.is_empty()).cloned();
        if web_ticket
            .as_ref()
            .zip(passport_ticket.as_ref())
            .is_some_and(|(a, b)| a != b)
        {
            return Err(DouyuRefreshError::InvalidCookie);
        }
        let dy_did = if passport_ticket.is_some() {
            // Never pair a passport ticket with a different account's Web DID.
            passport
                .get("dy_did")
                .filter(|v| !v.is_empty())
                .cloned()
                .or_else(|| web.get("dy_did").filter(|v| !v.is_empty()).cloned())
        } else {
            web.get("dy_did").filter(|v| !v.is_empty()).cloned()
        };
        let account_id = web.get("acf_uid").filter(|v| !v.is_empty()).cloned();
        if account_id.as_deref().is_some_and(|uid| !is_account_id(uid)) {
            return Err(DouyuRefreshError::InvalidCookie);
        }
        Ok(Self {
            cookie: cookie::format_cookie_header(&web)
                .map_err(|_| DouyuRefreshError::InvalidCookie)?,
            ltp0: passport_ticket.or(web_ticket),
            dy_did,
            account_id,
        })
    }
}

impl fmt::Debug for DouyuCookieInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DouyuCookieInput")
            .field("has_cookie", &!self.cookie.is_empty())
            .field("has_ltp0", &self.ltp0.is_some())
            .field("has_dy_did", &self.dy_did.is_some())
            .field("account_id", &self.account_id)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DouyuLoginIdentity {
    pub account_id: String,
}

#[derive(Clone)]
pub struct DouyuCookieRefresh {
    pub cookie: String,
    pub ltp0: String,
    pub dy_did: String,
    pub account_id: String,
    pub updated_fields: usize,
}

impl fmt::Debug for DouyuCookieRefresh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DouyuCookieRefresh")
            .field("account_id", &self.account_id)
            .field("updated_fields", &self.updated_fields)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct DouyuRefreshClient {
    client: Client,
    safe_auth: Url,
    probe: Url,
    #[cfg(test)]
    mock_origin: Option<Url>,
}

impl DouyuRefreshClient {
    pub fn new() -> Result<Self, DouyuRefreshError> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| DouyuRefreshError::Network)?;
        Ok(Self {
            client,
            safe_auth: Url::parse(SAFE_AUTH_URL).expect("constant URL"),
            probe: Url::parse(PROBE_URL).expect("constant URL"),
            #[cfg(test)]
            mock_origin: None,
        })
    }

    #[cfg(test)]
    fn for_test_server(origin: &str) -> Self {
        let origin = Url::parse(origin).expect("test origin");
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("test client"),
            safe_auth: origin.join("/lapi/passport/iframe/safeAuth").unwrap(),
            probe: origin.join("/wgapi/livenc/liveweb/follow/top3").unwrap(),
            mock_origin: Some(origin),
        }
    }

    /// A new exchange has request-local state and cannot borrow another account's
    /// cookies from a shared jar. Callers must serialize refreshes and discard a
    /// result if credentials changed while the request was in flight.
    pub async fn refresh(
        &self,
        current_cookie: &str,
        ltp0: &str,
        dy_did: &str,
        expected_account_id: Option<&str>,
    ) -> Result<DouyuCookieRefresh, DouyuRefreshError> {
        let current = DouyuCookieInput::parse(current_cookie)?;
        if ltp0.is_empty() || dy_did.is_empty() {
            return Err(DouyuRefreshError::MissingCredential);
        }
        let mut passport = BTreeMap::from([
            ("LTP0".to_owned(), validated_credential("LTP0", ltp0)?),
            ("dy_did".to_owned(), validated_credential("dy_did", dy_did)?),
        ]);
        check_identity(current.account_id.as_deref(), expected_account_id)?;
        let expected = expected_account_id.or(current.account_id.as_deref());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DouyuRefreshError::Protocol)?
            .as_millis()
            .to_string();
        let mut url = self.safe_auth.clone();
        url.query_pairs_mut().extend_pairs([
            ("client_id", "1"),
            ("t", &timestamp),
            ("_", &timestamp),
            ("callback", "cb"),
        ]);
        let mut fresh: BTreeMap<String, String> = BTreeMap::new();
        let mut final_body = None;
        for _ in 0..MAX_REDIRECTS {
            self.check_exchange_url(&url)?;
            // The verified bridge expects LTP0 on its one-time login endpoint;
            // no other main-site path is allowed to receive it.
            let mut request_cookies = passport.clone();
            if self.is_web_host(&url) {
                for (name, value) in &fresh {
                    if name != "dy_did" {
                        request_cookies.insert(name.clone(), value.clone());
                    }
                }
            }
            let response = self
                .get(url.clone(), &format_pairs(&request_cookies)?)
                .await?;
            let status = response.status();
            self.collect_cookies(&response, &url, &mut passport, &mut fresh)?;
            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(DouyuRefreshError::Protocol)?;
                let next = url
                    .join(location)
                    .map_err(|_| DouyuRefreshError::UnsafeRedirect)?;
                self.check_exchange_url(&next)?;
                url = next;
                continue;
            }
            if status != StatusCode::OK {
                return Err(DouyuRefreshError::Http);
            }
            final_body = Some(read_body(response).await?);
            break;
        }
        let body = final_body.ok_or(DouyuRefreshError::Protocol)?;
        if REQUIRED_KEYS
            .iter()
            .any(|name| fresh.get(*name).is_none_or(|v| v.is_empty()))
        {
            return Err(DouyuRefreshError::LoginInvalid);
        }
        let uid = fresh
            .get("acf_uid")
            .cloned()
            .ok_or(DouyuRefreshError::LoginInvalid)?;
        if !is_account_id(&uid) {
            return Err(DouyuRefreshError::Protocol);
        }
        check_identity(Some(&uid), expected)?;
        // The bridge may return JSONP with identity as well as Set-Cookie.
        // Compare only the identity keys demonstrated in the reference source.
        if let Some(identity) = callback_identity(&body) {
            check_identity(Some(&identity), Some(&uid))?;
        }
        let mut web = validated_pairs(&current.cookie)?;
        let preserve_devices = DEVICE_KEYS
            .iter()
            .any(|key| web.get(*key).is_some_and(|v| !v.is_empty()));
        let updated_fields = fresh.len();
        for (name, value) in fresh {
            if preserve_devices && DEVICE_KEYS.contains(&name.as_str()) {
                continue;
            }
            web.insert(name, value);
        }
        let new_ltp0 = passport
            .remove("LTP0")
            .filter(|v| !v.is_empty())
            .ok_or(DouyuRefreshError::LoginInvalid)?;
        let new_did = passport
            .remove("dy_did")
            .filter(|v| !v.is_empty())
            .ok_or(DouyuRefreshError::LoginInvalid)?;
        if !preserve_devices && !web.contains_key("dy_did") {
            web.insert("dy_did".into(), new_did.clone());
        }
        web.remove("LTP0");
        let header = format_pairs(&web)?;
        if self.validate(&header, Some(&uid)).await?.is_none() {
            return Err(DouyuRefreshError::LoginInvalid);
        }
        Ok(DouyuCookieRefresh {
            cookie: header,
            ltp0: new_ltp0,
            dy_did: new_did,
            account_id: uid,
            updated_fields,
        })
    }

    /// The follow/top3 endpoint is authenticated. HTTP 200 alone, HTML, a
    /// missing error field, or a coerced JSON string must never imply login.
    pub async fn validate(
        &self,
        input: &str,
        expected_account_id: Option<&str>,
    ) -> Result<Option<DouyuLoginIdentity>, DouyuRefreshError> {
        let input = DouyuCookieInput::parse(input)?;
        let Some(uid) = input.account_id else {
            return Ok(None);
        };
        if cookie::cookie_value(&input.cookie, "acf_auth").is_none() {
            return Ok(None);
        }
        check_identity(Some(&uid), expected_account_id)?;
        let response = self.get(self.probe.clone(), &input.cookie).await?;
        if response.status() != StatusCode::OK {
            return Err(DouyuRefreshError::Http);
        }
        let body = read_body(response).await?;
        let json: Value = serde_json::from_slice(&body).map_err(|_| DouyuRefreshError::Protocol)?;
        let code = json
            .get("error")
            .and_then(Value::as_i64)
            .ok_or(DouyuRefreshError::Protocol)?;
        if code != 0 {
            return Ok(None);
        }
        Ok(Some(DouyuLoginIdentity { account_id: uid }))
    }

    async fn get(&self, url: Url, cookies: &str) -> Result<Response, DouyuRefreshError> {
        self.client
            .get(url)
            .header(header::USER_AGENT, DOUYU_USER_AGENT)
            .header(header::REFERER, "https://www.douyu.com/")
            .header(header::ORIGIN, "https://www.douyu.com")
            .header(header::COOKIE, cookies)
            .send()
            .await
            .map_err(|_| DouyuRefreshError::Network)
    }

    fn is_web_host(&self, url: &Url) -> bool {
        #[cfg(test)]
        if self
            .mock_origin
            .as_ref()
            .is_some_and(|origin| url.origin() == origin.origin())
        {
            return url.path() == "/api/passport/login";
        }
        matches!(url.host_str(), Some("www.douyu.com" | "douyu.com"))
    }

    fn check_exchange_url(&self, url: &Url) -> Result<(), DouyuRefreshError> {
        #[cfg(test)]
        if self
            .mock_origin
            .as_ref()
            .is_some_and(|origin| url.origin() == origin.origin())
            && matches!(
                url.path(),
                "/api/passport/login" | "/lapi/passport/iframe/safeAuth"
            )
            && url.username().is_empty()
            && url.password().is_none()
        {
            return Ok(());
        }
        check_exchange_url(url)
    }

    fn collect_cookies(
        &self,
        response: &Response,
        origin: &Url,
        passport: &mut BTreeMap<String, String>,
        web: &mut BTreeMap<String, String>,
    ) -> Result<(), DouyuRefreshError> {
        for header in response.headers().get_all(header::SET_COOKIE) {
            let raw = header.to_str().map_err(|_| DouyuRefreshError::Protocol)?;
            let parsed = ::cookie::Cookie::parse(raw).map_err(|_| DouyuRefreshError::Protocol)?;
            let name = parsed.name();
            let value = parsed.value();
            if let Some(domain) = parsed.domain() {
                let domain = domain.trim_start_matches('.');
                if domain != "douyu.com" && Some(domain) != origin.host_str() {
                    return Err(DouyuRefreshError::Protocol);
                }
            }
            if parsed.path().is_some_and(|path| path != "/") {
                // Only root-scoped cookies can be persisted as a Cookie header.
                continue;
            }
            let is_web = self.is_web_host(origin);
            let target = if !is_web && matches!(name, "LTP0" | "dy_did") {
                &mut *passport
            } else if is_web && (name.starts_with("acf_") || matches!(name, "dy_auth" | "dy_did")) {
                &mut *web
            } else {
                continue;
            };
            let pair = validated_pairs(&format!("{name}={value}"))?;
            let expired = parsed.max_age().is_some_and(|age| age.whole_seconds() <= 0)
                || parsed
                    .expires_datetime()
                    .is_some_and(|expires| expires <= ::time::OffsetDateTime::now_utc());
            if value.is_empty() || expired {
                target.remove(name);
            } else {
                target.extend(pair);
            }
        }
        Ok(())
    }
}

fn check_exchange_url(url: &Url) -> Result<(), DouyuRefreshError> {
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(DouyuRefreshError::UnsafeRedirect);
    }
    match (url.host_str(), url.path()) {
        (Some("passport.douyu.com"), "/lapi/passport/iframe/safeAuth")
        | (Some("www.douyu.com" | "douyu.com"), "/api/passport/login") => Ok(()),
        _ => Err(DouyuRefreshError::UnsafeRedirect),
    }
}

fn validated_pairs(input: &str) -> Result<BTreeMap<String, String>, DouyuRefreshError> {
    cookie::parse_cookie_scopes(input)
        .map(|(web, _)| web)
        .map_err(|_| DouyuRefreshError::InvalidCookie)
}

fn validated_credential(name: &str, value: &str) -> Result<String, DouyuRefreshError> {
    let pair = validated_pairs(&format!("{name}={value}"))?;
    if pair.len() != 1 || pair.get(name).is_none_or(|parsed| parsed != value) {
        return Err(DouyuRefreshError::InvalidCookie);
    }
    Ok(value.to_owned())
}

fn format_pairs(pairs: &BTreeMap<String, String>) -> Result<String, DouyuRefreshError> {
    cookie::format_cookie_header(pairs).map_err(|_| DouyuRefreshError::InvalidCookie)
}

fn is_account_id(uid: &str) -> bool {
    !uid.is_empty() && uid != "0" && uid.bytes().all(|byte| byte.is_ascii_digit())
}

fn check_identity(actual: Option<&str>, expected: Option<&str>) -> Result<(), DouyuRefreshError> {
    if expected.is_some_and(|uid| !is_account_id(uid)) {
        return Err(DouyuRefreshError::InvalidCookie);
    }
    if actual.zip(expected).is_some_and(|(a, b)| a != b) {
        return Err(DouyuRefreshError::AccountMismatch);
    }
    Ok(())
}

fn callback_identity(body: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(body).ok()?.trim();
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let json: Value = serde_json::from_str(&text[start..=end]).ok()?;
    ["/uid", "/user_id", "/id", "/data/uid"]
        .into_iter()
        .find_map(|path| {
            let value = json.pointer(path)?;
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|uid| uid.to_string()))
        })
}

async fn read_body(mut response: Response) -> Result<Vec<u8>, DouyuRefreshError> {
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| DouyuRefreshError::Network)?
    {
        if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
            return Err(DouyuRefreshError::Protocol);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, Uri},
        response::IntoResponse,
        routing::get,
    };
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;

    #[test]
    fn parses_passport_ticket_without_leaking_it_to_web_cookie() {
        let input = r#"[
            {"name":"acf_uid","value":"42","domain":"www.douyu.com"},
            {"name":"acf_auth","value":"old","domain":"www.douyu.com"},
            {"name":"LTP0","value":"ticket","domain":"passport.douyu.com"},
            {"name":"dy_did","value":"device","domain":"passport.douyu.com"}
        ]"#;
        let parsed = DouyuCookieInput::parse(input).unwrap();
        assert_eq!(parsed.ltp0.as_deref(), Some("ticket"));
        assert_eq!(parsed.dy_did.as_deref(), Some("device"));
        assert!(!parsed.cookie.contains("LTP0"));
    }

    #[test]
    fn redirect_allowlist_rejects_cross_origin_and_http() {
        for input in [
            "http://www.douyu.com/api/passport/login",
            "https://www.douyu.com.evil.test/api/passport/login",
            "https://www.douyu.com:8443/api/passport/login",
            "https://user@www.douyu.com/api/passport/login",
            "https://www.douyu.com/api/passport/login#fragment",
            "https://www.douyu.com/member/login",
            "https://other.douyu.com/api/passport/login",
            "https://passport.douyu.com/api/passport/login",
        ] {
            assert_eq!(
                check_exchange_url(&Url::parse(input).unwrap()),
                Err(DouyuRefreshError::UnsafeRedirect)
            );
        }
        assert!(
            check_exchange_url(&Url::parse("https://www.douyu.com/api/passport/login").unwrap())
                .is_ok()
        );
    }

    #[derive(Clone)]
    struct Fixture {
        seen: Arc<Mutex<Vec<(String, String, String)>>>,
        start_location: String,
        start_cookies: Vec<String>,
        exchange_status: StatusCode,
        exchange_body: String,
        exchange_cookies: Vec<String>,
        probe_status: StatusCode,
        probe_body: String,
    }

    impl Default for Fixture {
        fn default() -> Self {
            Self {
                seen: Arc::default(),
                start_location: "/api/passport/login".into(),
                start_cookies: Vec::new(),
                exchange_status: StatusCode::OK,
                exchange_body: "cb({\"data\":{\"uid\":42}});".into(),
                exchange_cookies: [
                    ("acf_uid", "42"),
                    ("acf_auth", "fresh"),
                    ("acf_stk", "stk"),
                    ("acf_ltkid", "ltk"),
                    ("acf_biz", "biz"),
                    ("acf_ct", "ct"),
                ]
                .into_iter()
                .map(|(name, value)| format!("{name}={value}; Path=/"))
                .collect(),
                probe_status: StatusCode::OK,
                probe_body: r#"{"error":0,"room_list":[]}"#.into(),
            }
        }
    }

    fn record(fixture: &Fixture, uri: &Uri, headers: &HeaderMap) {
        fixture.seen.lock().unwrap().push((
            uri.path().into(),
            uri.query().unwrap_or_default().into(),
            headers
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .into(),
        ));
    }

    fn response_cookies(cookies: &[String]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for cookie in cookies {
            headers.append("set-cookie", cookie.parse().unwrap());
        }
        headers
    }

    async fn start(
        State(fixture): State<Fixture>,
        uri: Uri,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        record(&fixture, &uri, &headers);
        let mut out = response_cookies(&fixture.start_cookies);
        out.insert("location", fixture.start_location.parse().unwrap());
        (StatusCode::FOUND, out, "")
    }

    async fn exchange(
        State(fixture): State<Fixture>,
        uri: Uri,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        record(&fixture, &uri, &headers);
        (
            fixture.exchange_status,
            response_cookies(&fixture.exchange_cookies),
            fixture.exchange_body,
        )
    }

    async fn probe(
        State(fixture): State<Fixture>,
        uri: Uri,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        record(&fixture, &uri, &headers);
        (
            fixture.probe_status,
            [("content-type", "application/json")],
            fixture.probe_body,
        )
    }

    async fn start_fixture(fixture: Fixture) -> (String, Fixture, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/lapi/passport/iframe/safeAuth", get(start))
            .route("/api/passport/login", get(exchange))
            .route("/wgapi/livenc/liveweb/follow/top3", get(probe))
            .with_state(fixture.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{address}"), fixture, handle)
    }

    #[tokio::test]
    async fn safe_auth_exchange_is_manual_and_probe_is_authenticated() {
        let (origin, fixture, server) = start_fixture(Fixture::default()).await;
        let client = DouyuRefreshClient::for_test_server(&origin);
        let result = client
            .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
            .await
            .unwrap();
        assert_eq!(result.account_id, "42");
        assert_eq!(
            cookie::cookie_value(&result.cookie, "acf_auth"),
            Some("fresh")
        );
        let requests = fixture.seen.lock().unwrap();
        assert!(requests.iter().any(
            |(path, _, cookie)| path == "/api/passport/login" && cookie.contains("LTP0=ticket")
        ));
        assert!(
            requests.iter().any(
                |(path, _, cookie)| path.ends_with("top3") && cookie.contains("acf_auth=fresh")
            )
        );
        assert!(
            requests
                .iter()
                .filter(|(path, _, _)| path.ends_with("top3"))
                .all(|(_, _, cookie)| !cookie.contains("LTP0"))
        );
        let (_, query, passport) = &requests[0];
        let parameters = url::form_urlencoded::parse(query.as_bytes()).collect::<BTreeMap<_, _>>();
        assert_eq!(parameters.get("client_id").map(|v| v.as_ref()), Some("1"));
        assert_eq!(parameters.get("t"), parameters.get("_"));
        assert!(passport.contains("dy_did=device"));
        assert!(!passport.contains("acf_auth=old"));
        server.abort();
    }

    #[test]
    fn credential_debug_and_errors_are_redacted() {
        let input = DouyuCookieInput::parse(
            "LTP0=secret_ticket; dy_did=secret_device; acf_auth=secret_auth; acf_uid=42",
        )
        .unwrap();
        assert!(!format!("{input:?}").contains("secret"));
        for input in [
            "acf_auth=secret\r\nInjected: secret",
            "acf_auth=secret; acf_auth=other",
            "acf_uid=secret",
        ] {
            let error = DouyuCookieInput::parse(input).unwrap_err();
            assert!(!format!("{error:?} {error}").contains("secret"));
        }
        for value in ["secret; acf_uid=1", "secret\r\n", " secret"] {
            assert_eq!(
                validated_credential("LTP0", value),
                Err(DouyuRefreshError::InvalidCookie)
            );
        }
    }

    #[test]
    fn conflicting_cookie_imports_fail_closed() {
        for input in [
            "acf_uid=42; acf_uid=99",
            "LTP0=one; LTP0=two",
            r#"[{"name":"acf_auth","value":"one","domain":".douyu.com"},{"name":"acf_auth","value":"two","domain":"www.douyu.com"}]"#,
            r#"[{"name":"LTP0","value":"one","domain":".douyu.com"},{"name":"LTP0","value":"two","domain":"passport.douyu.com"}]"#,
        ] {
            assert_eq!(
                DouyuCookieInput::parse(input).unwrap_err(),
                DouyuRefreshError::InvalidCookie
            );
        }
    }

    #[tokio::test]
    async fn probe_requires_explicit_integer_success() {
        for (body, expected) in [
            (r#"{"error":-3,"msg":"secret"}"#, Ok(false)),
            ("<html>secret</html>", Err(DouyuRefreshError::Protocol)),
            (r#"{"room_list":[]}"#, Err(DouyuRefreshError::Protocol)),
            (r#"{"error":"0"}"#, Err(DouyuRefreshError::Protocol)),
            (r#"{"error":null}"#, Err(DouyuRefreshError::Protocol)),
        ] {
            let (origin, _, server) = start_fixture(Fixture {
                probe_body: body.into(),
                ..Default::default()
            })
            .await;
            let actual = DouyuRefreshClient::for_test_server(&origin)
                .validate("acf_uid=42; acf_auth=fixture", Some("42"))
                .await
                .map(|id| id.is_some());
            assert_eq!(actual, expected);
            server.abort();
        }
    }

    #[tokio::test]
    async fn probe_http_failure_is_temporary() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::FOUND,
        ] {
            let (origin, _, server) = start_fixture(Fixture {
                probe_status: status,
                ..Default::default()
            })
            .await;
            assert_eq!(
                DouyuRefreshClient::for_test_server(&origin)
                    .validate("acf_uid=42; acf_auth=fixture", None)
                    .await
                    .unwrap_err(),
                DouyuRefreshError::Http
            );
            server.abort();
        }
    }

    #[tokio::test]
    async fn exchange_rejects_account_mismatch_before_probe() {
        for callback in [false, true] {
            let mut fixture = Fixture::default();
            if callback {
                fixture.exchange_body = r#"cb({"uid":99});"#.into();
            } else {
                fixture.exchange_cookies[0] = "acf_uid=99; Path=/".into();
            }
            let (origin, fixture, server) = start_fixture(fixture).await;
            assert_eq!(
                DouyuRefreshClient::for_test_server(&origin)
                    .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                    .await
                    .unwrap_err(),
                DouyuRefreshError::AccountMismatch
            );
            assert!(
                !fixture
                    .seen
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|(path, _, _)| path.ends_with("top3"))
            );
            server.abort();
        }
    }

    #[tokio::test]
    async fn all_required_cookie_fields_must_be_fresh() {
        for removed in REQUIRED_KEYS {
            let mut fixture = Fixture::default();
            fixture
                .exchange_cookies
                .retain(|header| !header.starts_with(&format!("{removed}=")));
            let (origin, _, server) = start_fixture(fixture).await;
            let old =
                "acf_uid=42; acf_auth=old; acf_stk=old; acf_ltkid=old; acf_biz=old; acf_ct=old";
            assert_eq!(
                DouyuRefreshClient::for_test_server(&origin)
                    .refresh(old, "ticket", "device", Some("42"))
                    .await
                    .unwrap_err(),
                DouyuRefreshError::LoginInvalid
            );
            server.abort();
        }
    }

    #[tokio::test]
    async fn exchange_http_failure_is_temporary_even_with_missing_fields() {
        let (origin, _, server) = start_fixture(Fixture {
            exchange_status: StatusCode::INTERNAL_SERVER_ERROR,
            exchange_cookies: Vec::new(),
            ..Default::default()
        })
        .await;
        assert_eq!(
            DouyuRefreshClient::for_test_server(&origin)
                .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::Http
        );
        server.abort();
    }

    #[tokio::test]
    async fn unsafe_redirects_are_never_requested() {
        for location in [
            "http://evil.test/?secret",
            "https://www.douyu.com.evil.test/api/passport/login",
            "http://www.douyu.com/api/passport/login",
            "https://www.douyu.com/member/cp",
        ] {
            let (origin, fixture, server) = start_fixture(Fixture {
                start_location: location.into(),
                ..Default::default()
            })
            .await;
            assert_eq!(
                DouyuRefreshClient::for_test_server(&origin)
                    .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                    .await
                    .unwrap_err(),
                DouyuRefreshError::UnsafeRedirect
            );
            assert_eq!(fixture.seen.lock().unwrap().len(), 1);
            server.abort();
        }
    }

    #[tokio::test]
    async fn looping_redirects_are_bounded() {
        let (origin, fixture, server) = start_fixture(Fixture {
            start_location: "/lapi/passport/iframe/safeAuth".into(),
            ..Default::default()
        })
        .await;
        assert_eq!(
            DouyuRefreshClient::for_test_server(&origin)
                .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::Protocol
        );
        assert_eq!(fixture.seen.lock().unwrap().len(), MAX_REDIRECTS);
        server.abort();
    }

    #[tokio::test]
    async fn rotating_passport_credentials_are_preserved_with_stable_web_device() {
        let mut fixture = Fixture {
            start_cookies: vec![
                "LTP0=rotated_ticket; Path=/".into(),
                "dy_did=rotated_device; Path=/".into(),
            ],
            ..Default::default()
        };
        fixture
            .exchange_cookies
            .push("dy_did=random_main_device; Path=/".into());
        fixture
            .exchange_cookies
            .push("tracking=unwanted; Path=/".into());
        let (origin, fixture, server) = start_fixture(fixture).await;
        let result = DouyuRefreshClient::for_test_server(&origin)
            .refresh(
                "acf_uid=42; acf_auth=old; dy_did=stable_device; preference=keep",
                "ticket",
                "device",
                Some("42"),
            )
            .await
            .unwrap();
        assert_eq!(result.ltp0, "rotated_ticket");
        assert_eq!(result.dy_did, "rotated_device");
        assert_eq!(
            cookie::cookie_value(&result.cookie, "dy_did"),
            Some("stable_device")
        );
        assert_eq!(
            cookie::cookie_value(&result.cookie, "preference"),
            Some("keep")
        );
        assert!(cookie::cookie_value(&result.cookie, "tracking").is_none());
        let requests = fixture.seen.lock().unwrap();
        assert!(requests[1].2.contains("LTP0=rotated_ticket"));
        assert!(!format!("{result:?}").contains("rotated"));
        server.abort();
    }

    #[tokio::test]
    async fn deleted_fresh_auth_is_not_resurrected_from_old_cookie() {
        let mut fixture = Fixture::default();
        fixture
            .exchange_cookies
            .push("acf_auth=; Path=/; Max-Age=0".into());
        let (origin, _, server) = start_fixture(fixture).await;
        assert_eq!(
            DouyuRefreshClient::for_test_server(&origin)
                .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::LoginInvalid
        );
        server.abort();
    }

    #[tokio::test]
    async fn body_limit_rejects_unbounded_response() {
        let (origin, _, server) = start_fixture(Fixture {
            probe_body: "x".repeat(MAX_BODY_BYTES + 1),
            ..Default::default()
        })
        .await;
        assert_eq!(
            DouyuRefreshClient::for_test_server(&origin)
                .validate("acf_uid=42; acf_auth=fixture", None)
                .await
                .unwrap_err(),
            DouyuRefreshError::Protocol
        );
        server.abort();
    }

    #[tokio::test]
    async fn missing_credentials_and_known_account_conflict_do_not_send_requests() {
        let (origin, fixture, server) = start_fixture(Fixture::default()).await;
        let client = DouyuRefreshClient::for_test_server(&origin);
        assert_eq!(
            client
                .refresh("acf_uid=42; acf_auth=old", "", "device", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::MissingCredential
        );
        assert_eq!(
            client
                .refresh("acf_uid=42; acf_auth=old", "ticket", "", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::MissingCredential
        );
        assert_eq!(
            client
                .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("99"))
                .await
                .unwrap_err(),
            DouyuRefreshError::AccountMismatch
        );
        assert!(client.validate("acf_uid=42", None).await.unwrap().is_none());
        assert!(fixture.seen.lock().unwrap().is_empty());
        server.abort();
    }

    #[tokio::test]
    async fn concurrent_exchanges_do_not_share_passport_cookies() {
        let (origin, fixture, server) = start_fixture(Fixture::default()).await;
        let client = DouyuRefreshClient::for_test_server(&origin);
        let (first, second) = tokio::join!(
            client.refresh(
                "acf_uid=42; acf_auth=old",
                "first",
                "first_device",
                Some("42")
            ),
            client.refresh(
                "acf_uid=42; acf_auth=old",
                "second",
                "second_device",
                Some("42")
            ),
        );
        assert_eq!(first.unwrap().ltp0, "first");
        assert_eq!(second.unwrap().ltp0, "second");
        let seen = fixture.seen.lock().unwrap();
        for (_, _, cookie) in seen.iter().filter(|(path, _, _)| !path.ends_with("top3")) {
            assert_ne!(
                cookie.contains("LTP0=first"),
                cookie.contains("LTP0=second")
            );
            assert_eq!(
                cookie.contains("LTP0=first"),
                cookie.contains("dy_did=first_device")
            );
        }
        server.abort();
    }

    #[tokio::test]
    async fn unrelated_set_cookie_domains_cannot_enter_login_state() {
        let mut fixture = Fixture::default();
        fixture
            .exchange_cookies
            .push("acf_auth=foreign; Domain=evil.test; Path=/".into());
        let (origin, _, server) = start_fixture(fixture).await;
        assert_eq!(
            DouyuRefreshClient::for_test_server(&origin)
                .refresh("acf_uid=42; acf_auth=old", "ticket", "device", Some("42"))
                .await
                .unwrap_err(),
            DouyuRefreshError::Protocol
        );
        server.abort();
    }
}
