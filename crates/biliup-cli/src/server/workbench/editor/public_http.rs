//! HTTPS media requests pin validated public DNS addresses and revalidate every redirect.
use reqwest::{Response, Url};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
pub(super) fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, c, _] = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_documentation()
                || v.is_multicast()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 198 && (18..20).contains(&b)))
        }
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return public(IpAddr::V4(v4));
            }
            let s = v.segments();
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] == 0 || s[1] == 0x0db8))
        }
    }
}
pub fn https(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "音源地址无效")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
        || url.port_or_known_default() != Some(443)
    {
        return Err("音源必须使用公开的 HTTPS 地址".into());
    }
    Ok(url)
}
pub async fn get(
    raw: &str,
    authorization: Option<&str>,
    range: Option<&str>,
) -> Result<Response, String> {
    let mut url = https(raw)?;
    let mut credential = authorization;
    for _ in 0..6 {
        let host = url
            .host_str()
            .ok_or("音源地址无效")?
            .trim_matches(['[', ']'])
            .to_owned();
        let addresses: Vec<SocketAddr> = tokio::time::timeout(
            Duration::from_secs(20),
            tokio::net::lookup_host((host.as_str(), 443)),
        )
        .await
        .map_err(|_| "音源 DNS 解析超时")?
        .map_err(|_| "无法解析音源地址")?
        .collect();
        if addresses.is_empty() || addresses.iter().any(|address| !public(address.ip())) {
            return Err("音源地址不能指向本机、内网或保留网络".into());
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&host, &addresses)
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(300))
            .user_agent("biliup-editor/1")
            .build()
            .map_err(|_| "无法初始化音源连接")?;
        let mut request = client.get(url.clone());
        if let Some(value) = credential {
            request = request.header(reqwest::header::AUTHORIZATION, value)
        }
        if let Some(value) = range {
            request = request.header(reqwest::header::RANGE, value)
        }
        let response = tokio::time::timeout(Duration::from_secs(30), request.send())
            .await
            .map_err(|_| "音源响应超时")?
            .map_err(|_| "无法连接音源服务")?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or("音源重定向无效")?;
            let next = https(url.join(location).map_err(|_| "音源重定向无效")?.as_str())?;
            if next.host_str() != url.host_str() {
                credential = None
            }
            url = next;
            continue;
        }
        return Ok(response);
    }
    Err("音源重定向次数过多".into())
}
pub async fn json(mut response: Response) -> Result<serde_json::Value, String> {
    if !response.status().is_success() {
        return Err(format!(
            "音源服务请求失败（{}）",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "音源响应中断")? {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("音源响应过大".into());
        }
        bytes.extend_from_slice(&chunk)
    }
    serde_json::from_slice(&bytes).map_err(|_| "音源响应格式无效".into())
}
pub fn stream(
    response: Response,
    max: usize,
) -> impl futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send {
    futures::stream::try_unfold(
        (response, 0usize),
        move |(mut response, total)| async move {
            match response
                .chunk()
                .await
                .map_err(|_| std::io::Error::other("音源下载中断"))?
            {
                Some(chunk) => {
                    let size = total.saturating_add(chunk.len());
                    if size > max {
                        return Err(std::io::Error::other("音频文件超过大小限制"));
                    }
                    Ok(Some((chunk, (response, size))))
                }
                None => Ok(None),
            }
        },
    )
}
