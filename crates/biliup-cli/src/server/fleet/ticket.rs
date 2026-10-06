//! `bfleet` 加入票据。
//!
//! 字节布局（全部大端）：
//!
//! | 字段 | 长度 |
//! | --- | --- |
//! | 版本号（当前 1） | 1 |
//! | 控制面 EndpointId | 32 |
//! | token id（ASCII） | 6 |
//! | 一次性秘密 | 16 |
//! | 过期时间（Unix 毫秒，i64） | 8 |
//! | relay 个数 | 1 |
//! | 每个 relay：长度（u16）+ UTF-8 URL | 2 + n |
//! | 以上所有字节 SHA-256 的前 4 字节 | 4 |
//!
//! 票据不含 IP 字段；relay 地址里的主机是控制面自己的网卡地址或 `--relay-url`。
//! 末尾的校验和让抄错、截断的票据在本地就报错，不必连到控制面才发现。

use iroh::EndpointId;
use iroh_tickets::{ParseError, Ticket};
use sha2::{Digest, Sha256};
use std::fmt;

pub const TICKET_VERSION: u8 = 1;
pub const TOKEN_ID_LEN: usize = 6;
pub const SECRET_LEN: usize = 16;
const CHECKSUM_LEN: usize = 4;
const MAX_RELAYS: usize = 16;

#[derive(Clone, PartialEq, Eq)]
pub struct JoinTicket {
    pub controller: EndpointId,
    pub token: String,
    pub secret: [u8; SECRET_LEN],
    pub expires_at: i64,
    pub relays: Vec<String>,
}

impl fmt::Debug for JoinTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinTicket")
            .field("controller", &self.controller)
            .field("token", &self.token)
            .field("secret", &"[redacted]")
            .field("expires_at", &self.expires_at)
            .field("relays", &self.relays)
            .finish()
    }
}

fn checksum(bytes: &[u8]) -> [u8; CHECKSUM_LEN] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; CHECKSUM_LEN];
    out.copy_from_slice(&digest[..CHECKSUM_LEN]);
    out
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.0.len() < n {
            return Err(ParseError::verification_failed("ticket is truncated"));
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ParseError> {
        Ok(self.take(N)?.try_into().expect("length checked"))
    }
}

impl Ticket for JoinTicket {
    const KIND: &'static str = "bfleet";

    fn encode_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(96);
        out.push(TICKET_VERSION);
        out.extend_from_slice(self.controller.as_bytes());
        debug_assert_eq!(self.token.len(), TOKEN_ID_LEN);
        out.extend_from_slice(self.token.as_bytes());
        out.extend_from_slice(&self.secret);
        out.extend_from_slice(&self.expires_at.to_be_bytes());
        let relays = &self.relays[..self.relays.len().min(MAX_RELAYS)];
        out.push(relays.len() as u8);
        for relay in relays {
            out.extend_from_slice(&(relay.len() as u16).to_be_bytes());
            out.extend_from_slice(relay.as_bytes());
        }
        let sum = checksum(&out);
        out.extend_from_slice(&sum);
        out
    }

    fn decode_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        if bytes.len() < CHECKSUM_LEN {
            return Err(ParseError::verification_failed("ticket is truncated"));
        }
        let (body, sum) = bytes.split_at(bytes.len() - CHECKSUM_LEN);
        if checksum(body) != sum {
            return Err(ParseError::verification_failed("ticket checksum mismatch"));
        }
        let mut reader = Reader(body);
        if reader.array::<1>()?[0] != TICKET_VERSION {
            return Err(ParseError::verification_failed(
                "unsupported ticket version",
            ));
        }
        let controller = EndpointId::from_bytes(&reader.array::<32>()?)
            .map_err(|_| ParseError::verification_failed("invalid controller key"))?;
        let token = std::str::from_utf8(reader.take(TOKEN_ID_LEN)?)
            .ok()
            .filter(|token| is_token_id(token))
            .ok_or_else(|| ParseError::verification_failed("invalid token id"))?
            .to_string();
        let secret = reader.array::<SECRET_LEN>()?;
        let expires_at = i64::from_be_bytes(reader.array::<8>()?);
        let count = reader.array::<1>()?[0] as usize;
        if count == 0 || count > MAX_RELAYS {
            return Err(ParseError::verification_failed("invalid relay list"));
        }
        let mut relays = Vec::with_capacity(count);
        for _ in 0..count {
            let len = u16::from_be_bytes(reader.array::<2>()?) as usize;
            let relay = std::str::from_utf8(reader.take(len)?)
                .map_err(|_| ParseError::verification_failed("invalid relay url"))?;
            relays.push(relay.to_string());
        }
        if !reader.0.is_empty() {
            return Err(ParseError::verification_failed("trailing bytes in ticket"));
        }
        Ok(JoinTicket {
            controller,
            token,
            secret,
            expires_at,
            relays,
        })
    }
}

/// token id：6 位小写字母或数字
pub fn is_token_id(token: &str) -> bool {
    token.len() == TOKEN_ID_LEN
        && token
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn sample() -> JoinTicket {
        JoinTicket {
            controller: SecretKey::generate().public(),
            token: "k3x9q2".into(),
            secret: [7u8; SECRET_LEN],
            expires_at: 1_790_000_000_000,
            relays: vec![
                "http://203.0.113.5:19160/".into(),
                "http://192.168.1.10:19160/".into(),
            ],
        }
    }

    #[test]
    fn round_trips_through_the_string_form() {
        let ticket = sample();
        let encoded = ticket.encode_string();
        assert!(encoded.starts_with("bfleet"));
        assert_eq!(JoinTicket::decode_string(&encoded).unwrap(), ticket);
    }

    #[test]
    fn debug_output_hides_the_secret() {
        assert!(!format!("{:?}", sample()).contains("7, 7"));
    }

    #[test]
    fn any_flipped_bit_is_rejected() {
        let bytes = sample().encode_bytes();
        for index in 0..bytes.len() {
            let mut tampered = bytes.clone();
            tampered[index] ^= 0x01;
            assert!(
                JoinTicket::decode_bytes(&tampered).is_err(),
                "byte {index} flipped but still accepted"
            );
        }
    }

    #[test]
    fn tampered_strings_are_rejected() {
        let encoded = sample().encode_string();
        // 换掉秘密所在位置附近的一个字符
        let mut chars: Vec<char> = encoded.chars().collect();
        let index = "bfleet".len() + 70;
        chars[index] = if chars[index] == 'a' { 'b' } else { 'a' };
        let tampered: String = chars.into_iter().collect();
        assert!(JoinTicket::decode_string(&tampered).is_err());
        assert!(JoinTicket::decode_string(&encoded[..encoded.len() - 3]).is_err());
        assert!(JoinTicket::decode_string(&encoded.replacen("bfleet", "bflet", 1)).is_err());
    }

    #[test]
    fn token_ids_are_six_lowercase_alphanumerics() {
        assert!(is_token_id("abc123"));
        assert!(!is_token_id("ABC123"));
        assert!(!is_token_id("abc12"));
        assert!(!is_token_id("abc-12"));
    }
}
