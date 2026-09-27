//! 主副配对的模式与参数（ha-pair 方案 §5.7、§6 C）。
//!
//! 参数存在控制面的 `ha_pair` 里，随期望状态下发给备机，不进主库的空间配置：
//! 不配对的机器上 `/v1/configuration` 一个键都不多。

use serde::{Deserialize, Serialize};
use std::fmt;

/// 模式 1：两台都录，主机投、备机备投；模式 2：主机录，主机离线时备机接手
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum HaMode {
    DualRecord = 1,
    Takeover = 2,
}

impl TryFrom<u8> for HaMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(HaMode::DualRecord),
            2 => Ok(HaMode::Takeover),
            other => Err(format!("unknown HA mode {other}, expected 1 or 2")),
        }
    }
}

impl From<HaMode> for u8 {
    fn from(mode: HaMode) -> Self {
        mode as u8
    }
}

impl fmt::Display for HaMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

/// 时长一律是秒
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HaParams {
    /// 备机与控制面断开超过这么久没重连上，判主机离线（`ha.offline_grace`）
    pub offline_grace: u64,
    /// 模式 1：判主机离线后再等这么久备机才投（`ha.standby_upload_delay`）
    pub standby_upload_delay: u64,
    /// 模式 1：主机在线，下播后这么久还没开始投，备机投（`ha.upload_start_timeout`）
    pub upload_start_timeout: u64,
    /// 模式 1：主机开始投了，但上传进度停住这么久，备机投（§6 C）
    pub upload_stall_timeout: u64,
    /// 主机上传中每隔这么久报一次进度（§6 C）
    pub progress_interval: u64,
    /// 模式 2：备机接手的那一场等主机这么久，之后转人工（`ha.manual_timeout`）
    pub manual_timeout: u64,
    /// 模式 1：主机投成后备机立即删掉自己那份（`ha.delete_standby_copy_after_primary_upload`）
    pub delete_standby_copy: bool,
}

impl Default for HaParams {
    fn default() -> Self {
        HaParams {
            offline_grace: 60,
            standby_upload_delay: 10 * 60,
            upload_start_timeout: 30 * 60,
            upload_stall_timeout: 15 * 60,
            progress_interval: 60,
            manual_timeout: 24 * 60 * 60,
            delete_standby_copy: false,
        }
    }
}

/// 每个时长允许的上限：一周
const MAX_SECONDS: u64 = 7 * 24 * 60 * 60;

impl HaParams {
    /// 每个时长都在 1 秒到一周之间；进度间隔必须比停滞判定短，否则上传中也会被当成停住
    pub fn validate(&self) -> Result<(), String> {
        let durations = [
            ("offline_grace", self.offline_grace),
            ("standby_upload_delay", self.standby_upload_delay),
            ("upload_start_timeout", self.upload_start_timeout),
            ("upload_stall_timeout", self.upload_stall_timeout),
            ("progress_interval", self.progress_interval),
            ("manual_timeout", self.manual_timeout),
        ];
        for (name, seconds) in durations {
            if seconds == 0 || seconds > MAX_SECONDS {
                return Err(format!("{name} 必须在 1 秒到 {MAX_SECONDS} 秒之间"));
            }
        }
        if self.progress_interval >= self.upload_stall_timeout {
            return Err("progress_interval 必须小于 upload_stall_timeout".into());
        }
        Ok(())
    }

    pub fn ms(seconds: u64) -> i64 {
        i64::try_from(seconds.saturating_mul(1000)).unwrap_or(i64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_proposal() {
        let params = HaParams::default();
        assert_eq!(params.offline_grace, 60);
        assert_eq!(params.standby_upload_delay, 600);
        assert_eq!(params.upload_start_timeout, 1800);
        assert_eq!(params.upload_stall_timeout, 900);
        assert_eq!(params.progress_interval, 60);
        assert_eq!(params.manual_timeout, 86400);
        assert!(!params.delete_standby_copy);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn missing_fields_take_defaults_and_bad_values_are_rejected() {
        let params: HaParams =
            serde_json::from_value(serde_json::json!({ "offline_grace": 5 })).unwrap();
        assert_eq!(params.offline_grace, 5);
        assert_eq!(params.manual_timeout, 86400);
        let zero = HaParams {
            offline_grace: 0,
            ..HaParams::default()
        };
        assert!(zero.validate().unwrap_err().contains("offline_grace"));
        let slow = HaParams {
            progress_interval: 900,
            ..HaParams::default()
        };
        assert!(slow.validate().unwrap_err().contains("progress_interval"));
    }

    #[test]
    fn modes_are_numbers_on_the_wire() {
        assert_eq!(serde_json::to_value(HaMode::Takeover).unwrap(), 2);
        assert_eq!(
            serde_json::from_value::<HaMode>(serde_json::json!(1)).unwrap(),
            HaMode::DualRecord
        );
        assert!(serde_json::from_value::<HaMode>(serde_json::json!(3)).is_err());
    }
}
