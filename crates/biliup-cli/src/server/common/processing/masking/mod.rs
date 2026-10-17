//! Privacy markers and compatibility for the existing mosaic_config setting.
pub mod config;
use crate::server::common::processing::model::{EffectType, MaskRegion, RenderRecipe};
use crate::server::config::{Config, ConfigPatch};
pub use config::{MosaicConfig, MosaicRegion};
use std::path::{Path, PathBuf};

fn config_value<'a>(
    config: &'a Config,
    override_cfg: &'a Option<ConfigPatch>,
) -> Option<&'a serde_json::Value> {
    match override_cfg
        .as_ref()
        .and_then(|patch| patch.mosaic_config.as_ref())
    {
        Some(value) => value.as_ref(),
        None => config.mosaic_config.as_ref(),
    }
}

/// An explicit malformed setting is protected too: parse/validation failures must
/// be reported by the processing step rather than silently disabling requested masking.
pub fn masking_required(config: &Config, override_cfg: &Option<ConfigPatch>) -> bool {
    config_value(config, override_cfg).is_some_and(|value| {
        value.get("enabled").and_then(serde_json::Value::as_bool) != Some(false)
    })
}

pub fn is_unmasked(path: &Path) -> bool {
    let path = if path.extension().is_some_and(|ext| ext == "part") {
        path.with_file_name(path.file_stem().expect("part has a stem"))
    } else {
        path.to_path_buf()
    };
    path.extension().is_some_and(|ext| ext == "unmasked")
        || path.file_stem().is_some_and(|stem| {
            Path::new(stem)
                .extension()
                .is_some_and(|ext| ext == "unmasked")
        })
}

pub fn masked_path(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    while path
        .extension()
        .is_some_and(|ext| ext == "part" || ext == "unmasked")
    {
        path = path.with_file_name(path.file_stem().expect("marker has a stem"));
    }
    if let (Some(stem), Some(ext)) = (path.file_stem(), path.extension()) {
        let base = Path::new(stem);
        if base.extension().is_some_and(|ext| ext == "unmasked") {
            let mut name = base
                .file_stem()
                .expect("unmasked marker has a stem")
                .to_os_string();
            name.push(".");
            name.push(ext);
            return path.with_file_name(name);
        }
    }
    path
}

/// Mark the real file, not merely its event path. Fail if the quarantine filename
/// already exists so a duplicate/colliding event cannot overwrite preserved footage.
pub fn quarantine(path: &Path) -> std::io::Result<PathBuf> {
    if is_unmasked(path) {
        return Ok(path.to_path_buf());
    }
    let mut name = path.as_os_str().to_os_string();
    name.push(".unmasked");
    let protected = PathBuf::from(name);
    if protected.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "unmasked recording already exists",
        ));
    }
    std::fs::rename(path, &protected)?;
    Ok(protected)
}

pub fn recipe(
    config: &Config,
    override_cfg: &Option<ConfigPatch>,
) -> Result<Option<RenderRecipe>, String> {
    let Some(value) = config_value(config, override_cfg) else {
        return Ok(None);
    };
    if value.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
        return Ok(None);
    }
    let legacy: MosaicConfig = serde_json::from_value(value.clone())
        .map_err(|e| format!("解析 mosaic_config 失败：{e}"))?;
    legacy.validate()?;
    Ok(Some(from_regions(&legacy.regions)))
}

fn from_regions(regions: &[MosaicRegion]) -> RenderRecipe {
    RenderRecipe {
        regions: regions
            .iter()
            .enumerate()
            .map(|(n, r)| MaskRegion {
                id: format!("legacy-mask-{n}"),
                x: r.x as f64,
                y: r.y as f64,
                width: r.width as f64,
                height: r.height as f64,
                strength: (r.strength as f64).max(1.),
                color: r.color.clone().unwrap_or_else(|| "#000000".into()),
                effect_type: match r.effect_type {
                    config::EffectType::Mosaic => EffectType::Mosaic,
                    config::EffectType::Blur => EffectType::Blur,
                    config::EffectType::Solid => EffectType::Solid,
                },
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
