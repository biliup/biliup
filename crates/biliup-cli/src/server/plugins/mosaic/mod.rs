// 马赛克插件模块
//
// 提供录制画面的区域遮挡功能（马赛克、模糊、纯色）

pub mod config;
pub mod ffmpeg_filter;
pub mod processor;

pub use config::{EffectType, MosaicConfig, MosaicRegion, PixelRegion};
pub use processor::{MosaicPlugin, is_unmasked, masked_path, masking_required, quarantine};
