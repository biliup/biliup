// 马赛克配置模块
//
// 定义马赛克区域和配置的数据结构

use serde::{Deserialize, Serialize};
use std::fmt;

/// 遮挡效果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectType {
    /// 马赛克（像素化）
    Mosaic,
    /// 高斯模糊
    Blur,
    /// 纯色填充
    Solid,
}

impl fmt::Display for EffectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mosaic => write!(f, "mosaic"),
            Self::Blur => write!(f, "blur"),
            Self::Solid => write!(f, "solid"),
        }
    }
}

/// 马赛克区域
///
/// 使用归一化坐标（0-1），可自动适配不同分辨率
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicRegion {
    /// 区域 ID（前端生成）
    pub id: String,

    /// X 坐标（归一化，0-1）
    pub x: f32,

    /// Y 坐标（归一化，0-1）
    pub y: f32,

    /// 宽度（归一化，0-1）
    pub width: f32,

    /// 高度（归一化，0-1）
    pub height: f32,

    /// 效果类型
    #[serde(rename = "effectType")]
    pub effect_type: EffectType,

    /// 强度
    /// - 马赛克：块大小（4-64 像素）
    /// - 模糊：半径（1-100）
    /// - 纯色：不使用
    pub strength: u32,

    /// 纯色填充的颜色（可选，如 "#000000"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl MosaicRegion {
    /// 验证区域配置
    pub fn validate(&self) -> Result<(), String> {
        // 检查坐标范围
        if !(0.0..=1.0).contains(&self.x) {
            return Err(format!("x 坐标超出范围 [0, 1]: {}", self.x));
        }
        if !(0.0..=1.0).contains(&self.y) {
            return Err(format!("y 坐标超出范围 [0, 1]: {}", self.y));
        }
        if !(0.0..=1.0).contains(&self.width) {
            return Err(format!("width 超出范围 [0, 1]: {}", self.width));
        }
        if !(0.0..=1.0).contains(&self.height) {
            return Err(format!("height 超出范围 [0, 1]: {}", self.height));
        }

        // 检查区域不超出边界
        if self.x + self.width > 1.0 {
            return Err(format!(
                "区域右边界超出画面: x({}) + width({}) > 1.0",
                self.x, self.width
            ));
        }
        if self.y + self.height > 1.0 {
            return Err(format!(
                "区域下边界超出画面: y({}) + height({}) > 1.0",
                self.y, self.height
            ));
        }

        // 检查区域大小（至少 1% 的画面）
        if self.width < 0.01 {
            return Err(format!("width 太小: {}", self.width));
        }
        if self.height < 0.01 {
            return Err(format!("height 太小: {}", self.height));
        }

        // 检查强度范围
        match self.effect_type {
            EffectType::Mosaic => {
                if !(4..=64).contains(&self.strength) {
                    return Err(format!("马赛克强度应在 4-64 之间: {}", self.strength));
                }
            }
            EffectType::Blur => {
                if !(1..=100).contains(&self.strength) {
                    return Err(format!("模糊强度应在 1-100 之间: {}", self.strength));
                }
            }
            EffectType::Solid => {
                // 纯色不检查强度
            }
        }

        if let Some(color) = &self.color
            && (color.len() != 7
                || !color.starts_with('#')
                || !color[1..].chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("颜色必须是 #RRGGBB 格式".into());
        }

        Ok(())
    }

    /// 转换为像素坐标
    pub fn to_pixel(&self, video_width: u32, video_height: u32) -> PixelRegion {
        // Round outward so subpixel edges are covered rather than silently exposing
        // the right/bottom edge. Use f64 to avoid rounding errors at frame boundaries.
        let x = (f64::from(self.x) * f64::from(video_width)).floor() as u32;
        let y = (f64::from(self.y) * f64::from(video_height)).floor() as u32;
        let right = ((f64::from(self.x) + f64::from(self.width)) * f64::from(video_width))
            .ceil()
            .min(f64::from(video_width)) as u32;
        let bottom = ((f64::from(self.y) + f64::from(self.height)) * f64::from(video_height))
            .ceil()
            .min(f64::from(video_height)) as u32;
        PixelRegion {
            x,
            y,
            width: right.saturating_sub(x).max(1),
            height: bottom.saturating_sub(y).max(1),
            effect_type: self.effect_type,
            strength: self.strength,
            color: self.color.clone(),
        }
    }
}

/// 像素坐标的区域（用于 FFmpeg 滤镜生成）
#[derive(Debug, Clone)]
pub struct PixelRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub effect_type: EffectType,
    pub strength: u32,
    pub color: Option<String>,
}

/// 马赛克完整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicConfig {
    /// 是否启用
    pub enabled: bool,

    /// 区域列表
    pub regions: Vec<MosaicRegion>,

    /// 处理模式（未来扩展）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl MosaicConfig {
    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.regions.is_empty() {
            return Err("启用了马赛克但未配置任何区域".to_string());
        }
        if self.regions.len() > 32 {
            return Err("最多支持 32 个遮挡区域".into());
        }

        for (i, region) in self.regions.iter().enumerate() {
            region
                .validate()
                .map_err(|e| format!("区域 {} 配置错误: {}", i + 1, e))?;
        }

        Ok(())
    }

    /// 从 JSON 字符串解析
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("解析 JSON 失败: {}", e))
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("序列化 JSON 失败: {}", e))
    }
}

impl Default for MosaicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            regions: Vec::new(),
            mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_validation() {
        let valid = MosaicRegion {
            id: "test".to_string(),
            x: 0.1,
            y: 0.2,
            width: 0.3,
            height: 0.4,
            effect_type: EffectType::Mosaic,
            strength: 16,
            color: None,
        };
        assert!(valid.validate().is_ok());

        let invalid_x = MosaicRegion {
            x: 1.5,
            ..valid.clone()
        };
        assert!(invalid_x.validate().is_err());

        let invalid_boundary = MosaicRegion {
            x: 0.8,
            width: 0.3,
            ..valid.clone()
        };
        assert!(invalid_boundary.validate().is_err());

        let invalid_strength = MosaicRegion {
            strength: 100,
            ..valid.clone()
        };
        assert!(invalid_strength.validate().is_err());
    }

    #[test]
    fn test_to_pixel() {
        let region = MosaicRegion {
            id: "test".to_string(),
            x: 0.5,
            y: 0.5,
            width: 0.2,
            height: 0.1,
            effect_type: EffectType::Blur,
            strength: 20,
            color: None,
        };

        let pixel = region.to_pixel(1920, 1080);
        assert_eq!(pixel.x, 960);
        assert_eq!(pixel.y, 540);
        assert!(pixel.width >= 384 && pixel.width <= 385);
        assert!(pixel.height >= 108 && pixel.height <= 109);
    }

    #[test]
    fn test_config_serialization() {
        let config = MosaicConfig {
            enabled: true,
            regions: vec![MosaicRegion {
                id: "region-1".to_string(),
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.15,
                effect_type: EffectType::Mosaic,
                strength: 16,
                color: None,
            }],
            mode: None,
        };

        let json = config.to_json().unwrap();
        let parsed = MosaicConfig::from_json(&json).unwrap();

        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.regions.len(), 1);
        assert_eq!(parsed.regions[0].id, "region-1");
    }

    #[test]
    fn rejects_non_finite_coordinates_and_filter_injection() {
        let mut region = MosaicRegion {
            id: "test".into(),
            x: 0.1,
            y: 0.1,
            width: 0.2,
            height: 0.2,
            effect_type: EffectType::Solid,
            strength: 0,
            color: None,
        };
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            region.x = value;
            assert!(region.validate().is_err());
        }
        region.x = 0.1;
        for color in ["black;null", "#000000:enable=0", "#GGGGGG", "#000"] {
            region.color = Some(color.into());
            assert!(region.validate().is_err());
        }
        region.color = Some("#fF0000".into());
        assert!(region.validate().is_ok());
    }

    #[test]
    fn tiny_regions_and_edges_are_covered_without_zero_pixel_sizes() {
        let region = MosaicRegion {
            id: "edge".into(),
            x: 0.99,
            y: 0.99,
            width: 0.01,
            height: 0.01,
            effect_type: EffectType::Mosaic,
            strength: 64,
            color: None,
        };
        assert!(region.validate().is_ok());
        let pixel = region.to_pixel(64, 64);
        assert_eq!(
            (pixel.x, pixel.y, pixel.width, pixel.height),
            (63, 63, 1, 1)
        );
    }
}
