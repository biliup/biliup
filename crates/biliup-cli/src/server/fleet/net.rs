//! 票据里的 relay 地址、节点端的 DNS 解析器。

use iroh::dns::{DnsResolver, NameserverConfig};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use url::Url;

/// Termux 等环境读不到系统 DNS 时的回落，国内可用
const FALLBACK_NAMESERVERS: [IpAddr; 2] = [
    IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
    IpAddr::V4(Ipv4Addr::new(119, 29, 29, 29)),
];

/// 公网地址：不是回环、内网、链路本地、CGNAT、文档或保留地址。
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                // 100.64.0.0/10：运营商 NAT、Tailscale 等
                || (a == 100 && (64..128).contains(&b))
                || a == 0
                || a >= 240)
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // fc00::/7 唯一本地地址、fe80::/10 链路本地
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                // 2001:db8::/32 文档地址
                || (first == 0x2001 && v6.segments()[1] == 0x0db8))
        }
    }
}

/// 节点能不能拿它当 relay 地址：排除回环、链路本地、未指定地址。
fn usable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !(v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()),
        IpAddr::V6(v6) => {
            !(v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

pub fn relay_url(ip: IpAddr, port: u16) -> Url {
    Url::parse(&format!("http://{}/", SocketAddr::new(ip, port))).expect("valid relay url")
}

/// 本机网卡地址拼成的 relay 地址：公网在前，内网在后；同类里 IPv4 在前。
pub fn interface_relay_urls(port: u16) -> Vec<Url> {
    let networks = sysinfo::Networks::new_with_refreshed_list();
    let ips = networks
        .values()
        .flat_map(|data| data.ip_networks().iter().map(|network| network.addr));
    order_relay_ips(ips)
        .into_iter()
        .map(|ip| relay_url(ip, port))
        .collect()
}

fn order_relay_ips(ips: impl IntoIterator<Item = IpAddr>) -> Vec<IpAddr> {
    let mut ips: Vec<IpAddr> = ips.into_iter().filter(|ip| usable(*ip)).collect();
    ips.sort_by_key(|ip| (!is_public(*ip), ip.is_ipv6(), *ip));
    ips.dedup();
    ips
}

/// 票据里的 relay 是否全是内网地址。主机名（域名）一律当作外部可达。
pub fn only_private(relays: &[Url]) -> bool {
    !relays.iter().any(|url| match url.host() {
        Some(url::Host::Ipv4(ip)) => is_public(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_public(IpAddr::V6(ip)),
        Some(url::Host::Domain(domain)) => domain != "localhost",
        None => false,
    })
}

/// 控制面自己连内嵌 relay 用的地址：监听在通配地址上时走回环。
pub fn local_relay_url(listen: SocketAddr) -> Url {
    let ip = match listen.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    relay_url(ip, listen.port())
}

fn nameservers_from_resolv_conf(text: &str) -> Vec<IpAddr> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next()? == "nameserver").then_some(())?;
            // 带 scope 的 IPv6（fe80::1%eth0）解析器不支持，跳过
            parts.next()?.parse().ok()
        })
        .collect()
}

/// 节点端的 DNS 解析器：先读 resolv.conf（Termux 是 `$PREFIX/etc/resolv.conf`），
/// 读不到再用系统配置；都不行时回落到 223.5.5.5 / 119.29.29.29。
pub fn dns_resolver() -> DnsResolver {
    let mut candidates = Vec::new();
    if let Some(prefix) = std::env::var_os("PREFIX") {
        candidates.push(std::path::PathBuf::from(prefix).join("etc/resolv.conf"));
    }
    candidates.push("/etc/resolv.conf".into());
    let nameservers = candidates
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .map(|text| nameservers_from_resolv_conf(&text))
        .find(|servers| !servers.is_empty())
        .unwrap_or_default();

    let builder = DnsResolver::builder();
    let builder = if nameservers.is_empty() {
        builder.with_system_defaults()
    } else {
        builder.add_nameserver_configs(nameservers.into_iter().map(NameserverConfig::udp))
    };
    builder
        .fallback_nameserver_configs(FALLBACK_NAMESERVERS.map(NameserverConfig::udp))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn classifies_public_and_private_addresses() {
        for public in ["203.0.114.5", "8.8.8.8", "2408:8207::1"] {
            assert!(is_public(ip(public)), "{public}");
        }
        for private in [
            "192.168.1.2",
            "10.0.0.1",
            "172.16.5.5",
            "100.100.1.1",
            "127.0.0.1",
            "169.254.1.1",
            "fd00::1",
            "fe80::1",
            "::1",
            "2001:db8::1",
        ] {
            assert!(!is_public(ip(private)), "{private}");
        }
    }

    #[test]
    fn public_addresses_come_first_and_unusable_ones_are_dropped() {
        let ordered = order_relay_ips([
            ip("192.168.1.2"),
            ip("127.0.0.1"),
            ip("fe80::1"),
            ip("2408:8207::1"),
            ip("8.8.8.8"),
            ip("fd00::1"),
            ip("192.168.1.2"),
        ]);
        assert_eq!(
            ordered,
            [
                ip("8.8.8.8"),
                ip("2408:8207::1"),
                ip("192.168.1.2"),
                ip("fd00::1")
            ]
        );
    }

    #[test]
    fn relay_urls_bracket_ipv6() {
        assert_eq!(
            relay_url(ip("fd00::1"), 19160).as_str(),
            "http://[fd00::1]:19160/"
        );
        assert_eq!(
            local_relay_url("0.0.0.0:19160".parse().unwrap()).as_str(),
            "http://127.0.0.1:19160/"
        );
        assert_eq!(
            local_relay_url("192.168.1.2:1".parse().unwrap()).as_str(),
            "http://192.168.1.2:1/"
        );
    }

    #[test]
    fn private_only_detection() {
        let urls =
            |list: &[&str]| -> Vec<Url> { list.iter().map(|u| u.parse().unwrap()).collect() };
        assert!(only_private(&urls(&["http://192.168.1.2:19160"])));
        assert!(!only_private(&urls(&[
            "http://192.168.1.2:19160",
            "http://8.8.8.8:19160"
        ])));
        assert!(!only_private(&urls(&["http://relay.example.com:19160"])));
        assert!(only_private(&[]));
    }

    #[test]
    fn parses_resolv_conf() {
        let text = "# comment\nnameserver 192.168.1.1\nsearch lan\nnameserver fe80::1%wlan0\nnameserver 2001:4860::8888\n";
        assert_eq!(
            nameservers_from_resolv_conf(text),
            [ip("192.168.1.1"), ip("2001:4860::8888")]
        );
    }
}
