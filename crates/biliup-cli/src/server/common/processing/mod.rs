//! Native steps in the ordinary segment_processor chain.
pub mod audio;
pub mod danmaku;
pub mod encoding;
pub mod engine;
pub mod gpu;
pub mod masking;
pub mod model;
pub mod transcode;
use crate::server::config::{Config, ConfigPatch};
use crate::server::infrastructure::models::hook_step::HookStep;

/// Automatic processing and manual exports share this persistent output root.
pub const OUTPUT_DIR: &str = "renders";

pub fn requires_masking(steps: &[HookStep]) -> bool {
    steps
        .iter()
        .any(|step| matches!(step, HookStep::Render { render } if !render.regions.is_empty()))
}

/// Translate the previous setting into the same native hook. Merge it with a
/// configured render step so masking and danmaku are encoded together.
pub fn steps(
    config: &Config,
    patch: &Option<ConfigPatch>,
    configured: &[HookStep],
) -> Result<Vec<HookStep>, String> {
    let mut steps = configured.to_vec();
    if let Some(mut legacy) = masking::recipe(config, patch)? {
        if let Some(HookStep::Render { render }) = steps
            .iter_mut()
            .find(|s| matches!(s, HookStep::Render { .. }))
        {
            render.regions.splice(0..0, legacy.regions.drain(..));
            render.validate()?;
        } else {
            steps.insert(0, HookStep::Render { render: legacy });
        }
    }
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_masks_and_native_danmaku_share_one_step() {
        let mut config = Config::default();
        config.mosaic_config = Some(serde_json::json!({"enabled":true,"regions":[{
            "id":"private","x":0.1,"y":0.1,"width":0.2,"height":0.2,
            "effectType":"solid","strength":0,"color":"#000000"}]}));
        let configured: Vec<HookStep> = serde_json::from_value(serde_json::json!([
            {"render":{"danmaku":{"enabled":true}}},{"remux":"mp4"}]))
        .unwrap();
        let merged = steps(&config, &None, &configured).unwrap();
        assert_eq!(merged.len(), 2);
        let HookStep::Render { render } = &merged[0] else {
            panic!("missing native step")
        };
        assert!(render.danmaku.enabled);
        assert_eq!(render.regions.len(), 1);
        render.validate().unwrap();
        assert!(matches!(&merged[1], HookStep::Remux { .. }));
        config.mosaic_config = Some(serde_json::json!({"enabled":true,"regions":[]}));
        assert!(steps(&config, &None, &configured).is_err());
    }
}
