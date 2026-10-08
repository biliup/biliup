//! FFmpeg complex filters for ordered, normalized masking regions.

use super::config::{EffectType, PixelRegion};

/// Every graph has exactly one named output. Each region acts on the previous
/// result, so overlapping regions cannot reveal pixels from the original input.
pub fn build_filter_graph(regions: &[PixelRegion], _width: u32, _height: u32) -> String {
    // RGB avoids the chroma subsampling rounding in crop/overlay that otherwise
    // leaves an unmasked stripe for odd x/y/width/height values.
    let mut parts = vec!["[0:v:0]format=rgb24[base0]".to_string()];
    for (i, region) in regions.iter().enumerate() {
        let next = i + 1;
        match region.effect_type {
            EffectType::Solid => {
                let color = region.color.as_deref().unwrap_or("#000000");
                parts.push(format!(
                    "[base{i}]drawbox=x={}:y={}:w={}:h={}:color={color}:t=fill[base{next}]",
                    region.x, region.y, region.width, region.height,
                ));
            }
            effect => {
                parts.push(format!("[base{i}]split=2[keep{i}][crop{i}]"));
                let transform = match effect {
                    EffectType::Mosaic => {
                        let block = region.strength.max(4);
                        format!(
                            "scale={}:{}:flags=neighbor,scale={}:{}:flags=neighbor",
                            (region.width / block).max(1),
                            (region.height / block).max(1),
                            region.width,
                            region.height,
                        )
                    }
                    EffectType::Blur => format!("gblur=sigma={}", region.strength),
                    EffectType::Solid => unreachable!(),
                };
                parts.push(format!(
                    "[crop{i}]crop={}:{}:{}:{}:exact=1,{transform}[effect{i}]",
                    region.width, region.height, region.x, region.y,
                ));
                parts.push(format!(
                    "[keep{i}][effect{i}]overlay=x={}:y={}:format=rgb:shortest=1[base{next}]",
                    region.x, region.y,
                ));
            }
        }
    }
    parts.push(format!(
        "[base{}]pad=ceil(iw/2)*2:ceil(ih/2)*2,format=yuv420p[masked]",
        regions.len(),
    ));
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(effect_type: EffectType) -> PixelRegion {
        PixelRegion {
            x: 11,
            y: 13,
            width: 21,
            height: 15,
            effect_type,
            strength: 64,
            color: None,
        }
    }

    #[test]
    fn tiny_mosaic_never_scales_to_zero() {
        let filter = build_filter_graph(&[region(EffectType::Mosaic)], 64, 64);
        assert!(filter.contains("scale=1:1:flags=neighbor"));
        assert!(filter.contains("crop=21:15:11:13:exact=1"));
        assert!(filter.contains("split=2"));
        assert!(filter.ends_with("[masked]"));
    }

    #[test]
    fn mixed_regions_are_all_connected_in_configuration_order() {
        let filter = build_filter_graph(
            &[
                region(EffectType::Mosaic),
                region(EffectType::Solid),
                region(EffectType::Blur),
            ],
            64,
            64,
        );
        assert!(filter.contains("[base1]drawbox="));
        assert!(filter.contains("t=fill[base2];[base2]split=2"));
        assert!(filter.contains("gblur=sigma=64"));
        assert!(filter.ends_with("[base3]pad=ceil(iw/2)*2:ceil(ih/2)*2,format=yuv420p[masked]"));
    }

    #[test]
    fn all_solid_regions_and_empty_graph_have_single_mapped_output() {
        let solid = region(EffectType::Solid);
        let filter = build_filter_graph(&[solid.clone(), solid], 64, 64);
        assert_eq!(filter.matches("drawbox=").count(), 2);
        assert!(!filter.contains("split="));
        assert_eq!(filter.matches("[masked]").count(), 1);
        assert!(build_filter_graph(&[], 64, 64).ends_with("[masked]"));
    }
}
