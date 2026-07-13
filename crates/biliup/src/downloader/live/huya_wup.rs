//! 虎牙 WUP(getCdnTokenInfoEx) 请求所需的最小 tars 编解码。
//!
//! 只实现该接口用到的子集：TarsV3 uni-packet（`map<string, bytes>` 载荷）、
//! `HUYA.GetCdnTokenExReq`/`HUYA.UserId` 结构编码与 `HUYA.GetCdnTokenExRsp` 解码。

use rand::Rng;

pub(crate) const WUP_MAIN_URL: &str = "https://wup.huya.com";
pub(crate) const WUP_YST_URL: &str = "https://snmhuya.yst.aisee.tv/liveui/getCdnTokenInfoEx";

const TYPE_INT8: u8 = 0;
const TYPE_INT16: u8 = 1;
const TYPE_INT32: u8 = 2;
const TYPE_INT64: u8 = 3;
const TYPE_STRING1: u8 = 6;
const TYPE_STRING4: u8 = 7;
const TYPE_MAP: u8 = 8;
const TYPE_LIST: u8 = 9;
const TYPE_STRUCT_BEGIN: u8 = 10;
const TYPE_STRUCT_END: u8 = 11;
const TYPE_ZERO: u8 = 12;
const TYPE_BYTES: u8 = 13;

struct TarsOutputStream {
    buffer: Vec<u8>,
}

impl TarsOutputStream {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn write_head(&mut self, tag: u8, ty: u8) {
        if tag < 15 {
            self.buffer.push((tag << 4) | ty);
        } else {
            self.buffer.push(0xF0 | ty);
            self.buffer.push(tag);
        }
    }

    fn write_int8(&mut self, tag: u8, value: i8) {
        if value == 0 {
            self.write_head(tag, TYPE_ZERO);
        } else {
            self.write_head(tag, TYPE_INT8);
            self.buffer.push(value as u8);
        }
    }

    fn write_int16(&mut self, tag: u8, value: i16) {
        if (-128..=127).contains(&value) {
            self.write_int8(tag, value as i8);
        } else {
            self.write_head(tag, TYPE_INT16);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    fn write_int32(&mut self, tag: u8, value: i32) {
        if (-32768..=32767).contains(&value) {
            self.write_int16(tag, value as i16);
        } else {
            self.write_head(tag, TYPE_INT32);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    fn write_int64(&mut self, tag: u8, value: i64) {
        if (i32::MIN as i64..=i32::MAX as i64).contains(&value) {
            self.write_int32(tag, value as i32);
        } else {
            self.write_head(tag, TYPE_INT64);
            self.buffer.extend_from_slice(&value.to_be_bytes());
        }
    }

    fn write_string(&mut self, tag: u8, value: &str) {
        let bytes = value.as_bytes();
        if bytes.len() <= 255 {
            self.write_head(tag, TYPE_STRING1);
            self.buffer.push(bytes.len() as u8);
        } else {
            self.write_head(tag, TYPE_STRING4);
            self.buffer
                .extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        }
        self.buffer.extend_from_slice(bytes);
    }

    fn write_bytes(&mut self, tag: u8, value: &[u8]) {
        self.write_head(tag, TYPE_BYTES);
        self.write_head(0, TYPE_INT8);
        self.write_int32(0, value.len() as i32);
        self.buffer.extend_from_slice(value);
    }

    fn write_map_str_bytes(&mut self, tag: u8, entries: &[(&str, &[u8])]) {
        self.write_head(tag, TYPE_MAP);
        self.write_int32(0, entries.len() as i32);
        for (key, value) in entries {
            self.write_string(0, key);
            self.write_bytes(1, value);
        }
    }

    fn write_empty_str_map(&mut self, tag: u8) {
        self.write_head(tag, TYPE_MAP);
        self.write_int32(0, 0);
    }

    fn write_struct_begin(&mut self, tag: u8) {
        self.write_head(tag, TYPE_STRUCT_BEGIN);
    }

    fn write_struct_end(&mut self) {
        self.write_head(0, TYPE_STRUCT_END);
    }
}

struct TarsInputStream<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> TarsInputStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek_head(&self) -> Option<(u8, u8)> {
        let byte = *self.data.get(self.pos)?;
        let tag = (byte >> 4) & 0x0F;
        let ty = byte & 0x0F;
        if tag >= 15 {
            Some((*self.data.get(self.pos + 1)?, ty))
        } else {
            Some((tag, ty))
        }
    }

    fn read_head(&mut self) -> Option<(u8, u8)> {
        let byte = *self.data.get(self.pos)?;
        let tag = (byte >> 4) & 0x0F;
        let ty = byte & 0x0F;
        self.pos += 1;
        if tag >= 15 {
            let tag = *self.data.get(self.pos)?;
            self.pos += 1;
            Some((tag, ty))
        } else {
            Some((tag, ty))
        }
    }

    fn skip_to_tag(&mut self, target: u8) -> bool {
        while let Some((tag, ty)) = self.peek_head() {
            if ty == TYPE_STRUCT_END || tag > target {
                return false;
            }
            if tag == target {
                return true;
            }
            self.read_head();
            self.skip_field(ty);
        }
        false
    }

    fn skip_field(&mut self, ty: u8) {
        match ty {
            TYPE_INT8 => self.pos += 1,
            TYPE_INT16 => self.pos += 2,
            TYPE_INT32 => self.pos += 4,
            TYPE_INT64 => self.pos += 8,
            TYPE_STRING1 => {
                if let Some(len) = self.data.get(self.pos).copied() {
                    self.pos += 1 + len as usize;
                }
            }
            TYPE_STRING4 => {
                if let Some(len) = self.read_u32_be() {
                    self.pos += len as usize;
                }
            }
            TYPE_MAP => {
                let size = self.read_int_value().unwrap_or(0);
                for _ in 0..size.saturating_mul(2) {
                    if let Some((_, ty)) = self.read_head() {
                        self.skip_field(ty);
                    }
                }
            }
            TYPE_LIST => {
                let size = self.read_int_value().unwrap_or(0);
                for _ in 0..size {
                    if let Some((_, ty)) = self.read_head() {
                        self.skip_field(ty);
                    }
                }
            }
            TYPE_BYTES => {
                self.read_head();
                let size = self.read_int_value().unwrap_or(0);
                self.pos += size as usize;
            }
            TYPE_STRUCT_BEGIN => {
                while let Some((_, ty)) = self.read_head() {
                    if ty == TYPE_STRUCT_END {
                        break;
                    }
                    self.skip_field(ty);
                }
            }
            _ => {}
        }
    }

    fn read_u32_be(&mut self) -> Option<u32> {
        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_be_bytes(bytes.try_into().ok()?))
    }

    /// 读取一个已消费掉 head 的整数值（带 head 的话先 read_head）。
    fn read_int_value(&mut self) -> Option<i64> {
        let (_, ty) = self.read_head()?;
        self.read_int_body(ty)
    }

    fn read_int_body(&mut self, ty: u8) -> Option<i64> {
        match ty {
            TYPE_ZERO => Some(0),
            TYPE_INT8 => {
                let v = *self.data.get(self.pos)? as i8;
                self.pos += 1;
                Some(v as i64)
            }
            TYPE_INT16 => {
                let bytes = self.data.get(self.pos..self.pos + 2)?;
                self.pos += 2;
                Some(i16::from_be_bytes(bytes.try_into().ok()?) as i64)
            }
            TYPE_INT32 => {
                let bytes = self.data.get(self.pos..self.pos + 4)?;
                self.pos += 4;
                Some(i32::from_be_bytes(bytes.try_into().ok()?) as i64)
            }
            TYPE_INT64 => {
                let bytes = self.data.get(self.pos..self.pos + 8)?;
                self.pos += 8;
                Some(i64::from_be_bytes(bytes.try_into().ok()?))
            }
            _ => None,
        }
    }

    fn read_string_body(&mut self, ty: u8) -> Option<String> {
        let len = match ty {
            TYPE_STRING1 => {
                let len = *self.data.get(self.pos)? as usize;
                self.pos += 1;
                len
            }
            TYPE_STRING4 => self.read_u32_be()? as usize,
            _ => return None,
        };
        let bytes = self.data.get(self.pos..self.pos + len)?;
        self.pos += len;
        Some(String::from_utf8_lossy(bytes).to_string())
    }

    fn read_bytes_body(&mut self) -> Option<Vec<u8>> {
        self.read_head()?;
        let size = self.read_int_value()? as usize;
        let bytes = self.data.get(self.pos..self.pos + size)?;
        self.pos += size;
        Some(bytes.to_vec())
    }

    fn read_string(&mut self, tag: u8) -> Option<String> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        let (_, ty) = self.read_head()?;
        self.read_string_body(ty)
    }

    fn read_bytes(&mut self, tag: u8) -> Option<Vec<u8>> {
        if !self.skip_to_tag(tag) {
            return None;
        }
        let (_, ty) = self.read_head()?;
        if ty != TYPE_BYTES {
            return None;
        }
        self.read_bytes_body()
    }
}

/// 生成随机的虎牙客户端 sHuYaUA（对应 Python UAGenerator.get_random_hyapp_ua）。
pub(crate) fn random_hyapp_ua() -> String {
    let mut rng = rand::thread_rng();
    // (平台短名, 版本, 是否安卓系——安卓系追加随机 build 号与 api level)
    let configs = [
        ("adr", "13.1.0", true),
        ("ios", "13.1.0", false),
        ("huya_nftv", "2.6.10", true),
        ("pc_exe", "7000000", false),
    ];
    let (platform, version, android) = configs[rng.gen_range(0..configs.len())];
    if android {
        format!(
            "{platform}&{version}.{}&official&{}",
            rng.gen_range(3000..=5000),
            rng.gen_range(28..=36)
        )
    } else {
        format!("{platform}&{version}&official")
    }
}

/// 编码 getCdnTokenInfoEx 的 TarsV3 uni-packet 请求体。
pub(crate) fn encode_get_cdn_token_ex(stream_name: &str, huya_ua: &str) -> Vec<u8> {
    // HUYA.GetCdnTokenExReq，写在 tag 0
    let mut req = TarsOutputStream::new();
    req.write_struct_begin(0);
    req.write_string(0, ""); // sFlvUrl
    req.write_string(1, stream_name); // sStreamName
    req.write_int32(2, 0); // iLoopTime
    req.write_struct_begin(3); // tId: HUYA.UserId
    req.write_int64(0, 0); // lUid
    req.write_string(1, ""); // sGuid
    req.write_string(2, ""); // sToken
    req.write_string(3, huya_ua); // sHuYaUA
    req.write_string(4, ""); // sCookie
    req.write_int32(5, 0); // iTokenType
    req.write_string(6, ""); // sDeviceId
    req.write_string(7, ""); // sQIMEI
    req.write_struct_end();
    req.write_int32(4, 66); // iAppId
    req.write_struct_end();

    // TarsV3 载荷：map<string, bytes> { "tReq": <encoded req> }
    let mut payload = TarsOutputStream::new();
    payload.write_map_str_bytes(0, &[("tReq", &req.buffer)]);

    // RequestPacket
    let mut packet = TarsOutputStream::new();
    packet.write_int16(1, 3); // iVersion = 3
    packet.write_int8(2, 0); // cPacketType
    packet.write_int32(3, 0); // iMessageType
    packet.write_int32(4, 1); // iRequestId
    packet.write_string(5, "liveui"); // sServantName
    packet.write_string(6, "getCdnTokenInfoEx"); // sFuncName
    packet.write_bytes(7, &payload.buffer); // sBuffer
    packet.write_int32(8, 0); // iTimeout
    packet.write_empty_str_map(9); // context
    packet.write_empty_str_map(10); // status

    let mut out = Vec::with_capacity(4 + packet.buffer.len());
    out.extend_from_slice(&((4 + packet.buffer.len()) as u32).to_be_bytes());
    out.extend_from_slice(&packet.buffer);
    out
}

/// 从 getCdnTokenInfoEx 响应中解出 sFlvToken。
pub(crate) fn decode_get_cdn_token_ex(body: &[u8]) -> Option<String> {
    let data = body.get(4..)?;
    let mut packet = TarsInputStream::new(data);
    let payload = packet.read_bytes(7)?;

    // map<string, bytes>，找 "tRsp"
    let mut map = TarsInputStream::new(&payload);
    let (tag, ty) = map.read_head()?;
    if tag != 0 || ty != TYPE_MAP {
        return None;
    }
    let size = map.read_int_value()?;
    for _ in 0..size {
        let (_, key_ty) = map.read_head()?;
        let key = map.read_string_body(key_ty)?;
        let (_, value_ty) = map.read_head()?;
        if value_ty != TYPE_BYTES {
            return None;
        }
        let value = map.read_bytes_body()?;
        if key == "tRsp" {
            // HUYA.GetCdnTokenExRsp 结构体在 tag 0，sFlvToken 在结构体内 tag 0
            let mut rsp = TarsInputStream::new(&value);
            let (_, ty) = rsp.read_head()?;
            if ty != TYPE_STRUCT_BEGIN {
                return None;
            }
            return rsp.read_string(0);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // 黄金数据由移植前的 Python tars 实现生成：
    // stream_name = "1234567-1234567-5299160-2469038-10057-A-0-1-imgplus"
    // sHuYaUA = "adr&13.1.0.4321&official&30"
    const GOLDEN_REQ: &str = "000000a210032c3c400156066c6976657569661167657443646e546f6b656e496e666f45787d0000740800010604745265711d0000670a06001633313233343536372d313233343536372d353239393136302d323436393033382d31303035372d412d302d312d696d67706c75732c3a0c16002600361b6164722631332e312e302e34333231266f6666696369616c26333046005c660076000b40420b8c980ca80c";
    // 同一实现编码的响应包，sFlvToken 见下方断言，iExpireTime = 1699999999
    const GOLDEN_RSP: &str = "0000009210032c3c400156066c6976657569661167657443646e546f6b656e496e666f45787d0000640800010604745273701d0000570a064e77735365637265743d61626331323326777354696d653d36366626666d3d6447567a6446397a64484a6c5957302533442663747970653d687579615f6c69766526743d3130302666733d62676374126553f0ff0b8c980ca80c";

    fn hex_decode(input: &str) -> Vec<u8> {
        (0..input.len())
            .step_by(2)
            .map(|idx| u8::from_str_radix(&input[idx..idx + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn encode_matches_python_reference_bytes() {
        let encoded = encode_get_cdn_token_ex(
            "1234567-1234567-5299160-2469038-10057-A-0-1-imgplus",
            "adr&13.1.0.4321&official&30",
        );
        assert_eq!(encoded, hex_decode(GOLDEN_REQ));
    }

    #[test]
    fn decode_extracts_flv_token_from_python_reference_bytes() {
        let token = decode_get_cdn_token_ex(&hex_decode(GOLDEN_RSP)).unwrap();
        assert_eq!(
            token,
            "wsSecret=abc123&wsTime=66f&fm=dGVzdF9zdHJlYW0%3D&ctype=huya_live&t=100&fs=bgct"
        );
    }

    #[test]
    fn decode_rejects_truncated_body() {
        let mut body = hex_decode(GOLDEN_RSP);
        body.truncate(20);
        assert!(decode_get_cdn_token_ex(&body).is_none());
    }

    #[test]
    fn random_hyapp_ua_has_expected_shape() {
        for _ in 0..50 {
            let ua = random_hyapp_ua();
            let parts: Vec<&str> = ua.split('&').collect();
            assert!(parts.len() == 3 || parts.len() == 4, "unexpected ua: {ua}");
            assert_eq!(parts[2], "official");
        }
    }
}
