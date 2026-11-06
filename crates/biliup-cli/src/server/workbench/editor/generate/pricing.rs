//! Public generation rates, checked 2026-11-04. These are estimates, not provider invoices.
use serde::Serialize;
use std::collections::BTreeMap;

pub(super) const IMAGE_QUALITY: &str = "medium";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Pricing {
    as_of: &'static str,
    url: &'static str,
    #[serde(flatten)]
    rate: Rate,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Rate {
    Image { quality: &'static str, usd_by_size: BTreeMap<&'static str, f64> },
    Video { usd_per_second: f64, seconds_min: u32, seconds_max: u32 },
    Audio { usd_per_minute: f64 },
    Usage,
}

pub(super) fn info(generator: &str, model: &str, provider: &str) -> Pricing {
    let (url, rate) = match (generator, model) {
        ("generate.gpt-image", "gpt-image-2") => (
            "https://developers.openai.com/api/docs/guides/image-generation#calculating-costs",
            Rate::Image { quality: IMAGE_QUALITY, usd_by_size: BTreeMap::from([
                ("1024x1024", 0.053), ("1024x1536", 0.041), ("1536x1024", 0.041),
            ]) },
        ),
        ("generate.omni", "gemini-omni-1.1-flash") => (
            "https://ai.google.dev/gemini-api/docs/pricing#gemini-omni-1.1-flash",
            // The request uses 720p; duration is chosen by the model, not a fixed input.
            Rate::Video { usd_per_second: 5792. * 17.50 / 1_000_000., seconds_min: 3, seconds_max: 10 },
        ),
        ("generate.sound", "eleven_text_to_sound_v2") => (
            "https://elevenlabs.io/pricing/api",
            // The public rate is per minute; the FAQ meters sound effects per generation.
            // Show the duration-equivalent reference price, not a guaranteed request total.
            Rate::Audio { usd_per_minute: 0.12 },
        ),
        _ => (
            match provider {
                "google" => "https://ai.google.dev/gemini-api/docs/pricing",
                "elevenlabs" => "https://elevenlabs.io/pricing/api",
                _ => "https://developers.openai.com/api/docs/pricing",
            },
            Rate::Usage,
        ),
    };
    Pricing { as_of: "2026-11-04", url, rate }
}

impl Pricing {
    pub(super) fn describe(&self, size: Option<&str>, seconds: Option<f64>) -> String {
        match &self.rate {
            Rate::Image { usd_by_size, .. } => match size.and_then(|size| usd_by_size.get(size)) {
                Some(usd) => format!("预计图像输出费用约 US${usd:.3}，另计提示词输入费用"),
                None => "图像费用按实际尺寸与用量计费".into(),
            },
            Rate::Video { usd_per_second, seconds_min, seconds_max } => format!(
                "预计视频输出费用约 US${:.3}–US${:.3}（720p，{seconds_min}–{seconds_max} 秒），另计输入与文本输出费用",
                usd_per_second * f64::from(*seconds_min), usd_per_second * f64::from(*seconds_max),
            ),
            Rate::Audio { usd_per_minute } => match seconds {
                Some(seconds) => format!("按公开时长费率折算约 US${:.3}；供应商按生成请求结算，以账户账单为准", seconds * usd_per_minute / 60.),
                None => format!("公开音效费率 US${usd_per_minute:.3}/分钟；以账户账单为准"),
            },
            Rate::Usage => "按当前模型实际用量计费，无法预先确定金额".into(),
        }
    }

    pub(super) fn source(&self) -> String {
        format!("费率来源（{}，未含税费或账户优惠）：{}", self.as_of, self.url)
    }
}
