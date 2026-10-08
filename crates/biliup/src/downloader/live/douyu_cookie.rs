use super::{DOUYU_USER_AGENT, DOUYU_WEB_DOMAIN, LiveError, LiveResult};
use reqwest::{Client, header::HeaderValue};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;

#[derive(Deserialize)]
struct ExportedCookie {
    name: String,
    value: String,
    #[serde(default)]
    domain: Option<String>,
}

/// Accept a Cookie request header or a browser extension's JSON cookie array.
/// Keep credentials out of parser errors: serde and cookie errors can echo input.
pub(super) fn normalize_cookie_header(input: &str) -> LiveResult<String> {
    let input = input.trim();
    let mut pairs = BTreeMap::new();
    if input.starts_with('[') {
        let cookies: Vec<ExportedCookie> = serde_json::from_str(input)
            .map_err(|_| LiveError::custom("斗鱼 Cookie JSON 格式错误，需要 name/value 数组"))?;
        for cookie in cookies {
            if cookie.domain.as_deref().is_some_and(|domain| {
                let domain = domain.trim_start_matches('.');
                domain != DOUYU_WEB_DOMAIN && domain != "douyu.com"
            }) {
                continue;
            }
            insert_cookie(&mut pairs, &cookie.name, &cookie.value)?;
        }
    } else {
        let input = input
            .get(..7)
            .filter(|prefix| prefix.eq_ignore_ascii_case("cookie:"))
            .map_or(input, |_| input[7..].trim());
        for pair in input.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            let (name, value) = pair
                .split_once('=')
                .ok_or_else(|| LiveError::custom("斗鱼 Cookie 格式错误，需要 name=value"))?;
            insert_cookie(&mut pairs, name.trim(), value.trim())?;
        }
    }
    let header = pairs
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ");
    HeaderValue::from_str(&header)
        .map_err(|_| LiveError::custom("斗鱼 Cookie 含有无效的请求头字符"))?;
    Ok(header)
}

fn insert_cookie(pairs: &mut BTreeMap<String, String>, name: &str, value: &str) -> LiveResult<()> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        || value.bytes().any(|b| b.is_ascii_control() || b == b';')
    {
        return Err(LiveError::custom("斗鱼 Cookie 包含无效的名称或值"));
    }
    pairs.insert(name.to_owned(), value.to_owned());
    Ok(())
}

pub(super) fn cookie_value<'a>(cookie: &'a str, name: &str) -> Option<&'a str> {
    cookie.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name && !value.is_empty()).then_some(value)
    })
}

pub(super) async fn validate_cookie(input: &str, client: &Client) -> LiveResult<bool> {
    let cookie = normalize_cookie_header(input)?;
    if !["acf_uid", "acf_auth"]
        .iter()
        .all(|name| cookie_value(&cookie, name).is_some())
    {
        return Ok(false);
    }

    // betard is public and succeeds for arbitrary cookies. The account page
    // redirects unauthenticated requests to /member/login (then passport).
    let response = client
        .get(format!("https://{DOUYU_WEB_DOMAIN}/member/cp"))
        .header("referer", format!("https://{DOUYU_WEB_DOMAIN}/"))
        .header("user-agent", DOUYU_USER_AGENT)
        .header("cookie", cookie)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|err| LiveError::custom(format!("斗鱼登录状态检查失败: {}", err.without_url())))?;

    if response.status().is_redirection() {
        return Ok(false);
    }
    let response = response
        .error_for_status()
        .map_err(|err| LiveError::custom(format!("斗鱼登录状态检查失败: {}", err.without_url())))?;
    Ok(is_account_page(response.url()))
}

fn is_account_page(url: &Url) -> bool {
    url.host_str() == Some(DOUYU_WEB_DOMAIN) && url.path().trim_end_matches('/') == "/member/cp"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_preserves_encoded_auth_and_requires_exact_nonempty_fields() {
        let header = normalize_cookie_header(
            " Cookie: acf_auth=a%2Fb==; acf_uid=42; empty=; acf_did=device ",
        )
        .unwrap();
        assert_eq!(cookie_value(&header, "acf_auth"), Some("a%2Fb=="));
        assert_eq!(cookie_value(&header, "acf_uid"), Some("42"));
        assert_eq!(cookie_value(&header, "empty"), None);
        assert_eq!(cookie_value("not_acf_uid=42; acf_uid=", "acf_uid"), None);
    }

    #[test]
    fn browser_export_only_uses_cookies_for_the_douyu_web_host() {
        let header = normalize_cookie_header(
            r#"[
                {"name":"acf_uid","value":"42","domain":".douyu.com","httpOnly":true},
                {"name":"acf_auth","value":"valid%2F==","domain":"www.douyu.com"},
                {"name":"acf_auth","value":"wrong","domain":"passport.douyu.com"},
                {"name":"acf_auth","value":"wrong","domain":"notdouyu.com"}
            ]"#,
        )
        .unwrap();
        assert_eq!(cookie_value(&header, "acf_uid"), Some("42"));
        assert_eq!(cookie_value(&header, "acf_auth"), Some("valid%2F=="));
    }

    #[test]
    fn invalid_cookie_errors_never_echo_credentials() {
        for input in [
            "acf_auth=secret\r\nInjected: secret",
            r#"[{"name":"acf_auth","value":"secret; injected=secret"}]"#,
            r#"[{"name":"acf_auth","value":"secret"}"#,
            "secret",
        ] {
            let error = normalize_cookie_header(input).unwrap_err().to_string();
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    fn login_redirect_is_not_a_valid_session() {
        for url in [
            "https://www.douyu.com/member/login",
            "https://passport.douyu.com/member/login?state=/member/cp",
            "https://www.douyu.com/betard/288016",
            "https://www.douyu.com/",
        ] {
            assert!(!is_account_page(&Url::parse(url).unwrap()));
        }
        assert!(is_account_page(
            &Url::parse("https://www.douyu.com/member/cp").unwrap()
        ));
    }

    #[tokio::test]
    async fn missing_auth_is_rejected_without_a_network_request() {
        let client = Client::new();
        for input in [
            "",
            "acf_uid=42",
            "acf_uid=42; acf_auth=",
            "not_acf_uid=42; acf_auth=x",
        ] {
            assert!(!validate_cookie(input, &client).await.unwrap());
        }
    }
}
