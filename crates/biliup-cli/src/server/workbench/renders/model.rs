//! Public recipe and private, immutable render job snapshots.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderRecipe {
    pub danmaku: DanmakuSettings,
    pub regions: Vec<MaskRegion>,
}
pub type RenderSettings = RenderRecipe;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DanmakuSettings {
    pub enabled: bool,
    pub font: String,
    pub font_size: f64,
    pub opacity: f64,
    pub outline: f64,
    pub scroll_seconds: f64,
    pub display_area: f64,
    pub density: i32,
    pub offset_ms: i64,
    pub segment_offsets: BTreeMap<i64, i64>,
}
impl Default for DanmakuSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            font: "Noto Sans CJK SC".into(),
            font_size: 38.,
            opacity: 0.8,
            outline: 2.,
            scroll_seconds: 12.,
            display_area: 0.4,
            density: -1,
            offset_ms: 0,
            segment_offsets: BTreeMap::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectType {
    Mosaic,
    Blur,
    Solid,
    Image,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MaskRegion {
    pub id: String,
    pub effect_type: EffectType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub strength: f64,
    pub color: String,
    pub opacity: f64,
    pub asset_id: Option<i64>,
    pub lock_aspect: bool,
    pub intervals: Vec<TimeInterval>,
}
impl Default for MaskRegion {
    fn default() -> Self {
        Self {
            id: String::new(),
            effect_type: EffectType::Mosaic,
            x: 0.,
            y: 0.,
            width: 0.2,
            height: 0.2,
            strength: 12.,
            color: "#000000".into(),
            opacity: 1.,
            asset_id: None,
            lock_aspect: true,
            intervals: vec![],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeInterval {
    pub from_ms: i64,
    pub to_ms: i64,
}

impl RenderRecipe {
    pub fn validate(&self) -> Result<(), String> {
        let between = |n: f64, low: f64, high: f64| n.is_finite() && n >= low && n <= high;
        let d = &self.danmaku;
        if (d.enabled && d.font != "Noto Sans CJK SC")
            || d.font.trim().is_empty()
            || d.font.len() > 128
            || d.font.chars().any(char::is_control)
        {
            return Err("弹幕字体名称无效".into());
        }
        if !between(d.font_size, 8., 200.)
            || !between(d.opacity, 0., 1.)
            || !between(d.outline, 0., 4.)
            || !between(d.scroll_seconds, 2., 60.)
            || !between(d.display_area, 0.05, 1.)
            || !(-1..=1000).contains(&d.density)
            || d.offset_ms.unsigned_abs() > 3_600_000
            || d.segment_offsets.len() > 10_000
            || d.segment_offsets
                .iter()
                .any(|(id, ms)| *id <= 0 || ms.unsigned_abs() > 3_600_000)
        {
            return Err("弹幕样式或时间偏移超出范围".into());
        }
        if self.regions.len() > 32 {
            return Err("最多添加 32 个遮挡区域".into());
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.regions {
            if r.id.is_empty() || r.id.len() > 128 || !ids.insert(&r.id) {
                return Err("遮挡区域 id 为空或重复".into());
            }
            if !between(r.x, 0., 1.)
                || !between(r.y, 0., 1.)
                || !between(r.width, 0.001, 1.)
                || !between(r.height, 0.001, 1.)
                || r.x + r.width > 1.000001
                || r.y + r.height > 1.000001
                || !between(r.opacity, 0., 1.)
                || !between(r.strength, 1., 100.)
            {
                return Err("遮挡区域的位置、大小、强度或透明度无效".into());
            }
            if r.color.len() != 7
                || !r.color.starts_with('#')
                || !r.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err("遮挡颜色必须是 #RRGGBB".into());
            }
            if r.effect_type == EffectType::Image && !r.asset_id.is_some_and(|id| id > 0) {
                return Err("图片遮挡需要选择图片".into());
            }
            if r.intervals.len() > 1000
                || r.intervals
                    .iter()
                    .any(|t| t.from_ms < 0 || t.to_ms <= t.from_ms)
            {
                return Err("遮挡时段必须有非负起点及更晚的终点".into());
            }
        }
        Ok(())
    }
    pub fn has_effects(&self) -> bool {
        self.danmaku.enabled || !self.regions.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSource {
    pub segment_id: i64,
    pub path: PathBuf,
    pub danmaku_path: Option<PathBuf>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub source_origin_ms: Option<i64>,
    pub trim_start_ms: i64,
    pub trim_end_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderAsset {
    pub id: i64,
    pub path: PathBuf,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSpec {
    pub session_id: i64,
    pub clip_id: Option<i64>,
    pub in_ms: i64,
    pub out_ms: i64,
    pub recipe: RenderRecipe,
    pub sources: Vec<RenderSource>,
    pub assets: Vec<RenderAsset>,
    #[serde(default)]
    pub font_path: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssetView {
    pub id: i64,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: i64,
    pub session_id: i64,
    pub clip_id: Option<i64>,
    pub state: String,
    pub phase: String,
    pub ratio: Option<f64>,
    pub error: Option<String>,
    pub output_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
    pub download_url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct JobRecord {
    pub view: JobView,
    pub spec: RenderSpec,
    pub output_path: Option<PathBuf>,
}
