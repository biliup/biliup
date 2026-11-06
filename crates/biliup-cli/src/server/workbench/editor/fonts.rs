//! Uncut export/fonts.ts, adapted to managed brand assets and portable bundled fonts (MIT).
//! Export jobs freeze immutable asset hashes; editing a kit cannot change a queued export.
use super::{ConnectionPool, Error, invalid};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use tokio::io::AsyncReadExt;
#[derive(Clone, Serialize, Deserialize)]
pub struct Face {
    family: String,
    weight: Option<String>,
    format: String,
    file: String,
    #[serde(default)]
    style: Option<String>,
}
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    fonts: Vec<Face>,
    logos: BTreeMap<String, String>,
    warnings: Vec<String>,
}
#[derive(Serialize)]
pub struct FontSource {
    family: String,
    weight: Option<String>,
    format: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
}
#[derive(Serialize)]
pub struct RenderAssets {
    pub fonts: Vec<FontSource>,
    pub logos: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}
pub fn bundled() -> Vec<FontSource> {
    let faces: [(&str, &str, &[u8]); 3] = [
        (
            "DM Sans",
            "100 1000",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../public/editor-fonts/DMSans-Variable.woff2"
            )),
        ),
        (
            "DM Mono",
            "400",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../public/editor-fonts/DMMono-Regular.woff2"
            )),
        ),
        (
            "DM Mono",
            "500",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../public/editor-fonts/DMMono-Medium.woff2"
            )),
        ),
    ];
    faces
        .into_iter()
        .map(|(family, weight, bytes)| FontSource {
            family: family.into(),
            weight: Some(weight.into()),
            format: "woff2".into(),
            url: format!("data:font/woff2;base64,{}", STANDARD.encode(bytes)),
            style: None,
        })
        .collect()
}
fn font_format(file: &str) -> Option<(&'static str, &'static str)> {
    match file.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "woff2" => Some(("woff2", "font/woff2")),
        "woff" => Some(("woff", "font/woff")),
        "ttf" => Some(("truetype", "font/ttf")),
        "otf" => Some(("opentype", "font/otf")),
        _ => None,
    }
}
fn logo_mime(file: &str) -> Option<&'static str> {
    match file.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}
pub(super) fn family(value: &str) -> bool {
    let count = value.chars().count();
    count > 0
        && count <= 80
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-'))
}
pub async fn snapshot(pool: &ConnectionPool, root: &Path, project: i64) -> Result<Snapshot, Error> {
    let mut snapshot = kit_snapshot(pool, root, project).await?;
    for font in super::project_fonts::list(pool, project).await? {
        let file = format!("{}.{}", font.hash, font.extension);
        let Some((format, _)) = font_format(&file) else {
            continue;
        };
        snapshot.fonts.push(Face {
            family: font.family,
            weight: Some(font.weight),
            style: Some(font.style),
            format: format.into(),
            file,
        });
    }
    Ok(snapshot)
}
async fn kit_snapshot(pool: &ConnectionPool, root: &Path, project: i64) -> Result<Snapshot, Error> {
    let active = super::resources::list(pool, project, "active").await?;
    let Some(id) = active
        .iter()
        .find(|a| a["scope"] == "project")
        .and_then(|a| a["value"]["kit"].as_str())
    else {
        return Ok(Snapshot::default());
    };
    let kits = super::resources::list(pool, project, "kit").await?;
    let Some(entry) = kits.iter().rev().find(|kit| kit["id"] == id) else {
        return Ok(Snapshot::default());
    };
    let scope = entry["scope"]
        .as_str()
        .ok_or_else(|| invalid("品牌范围无效"))?;
    let kit = &entry["value"];
    let mut out = Snapshot::default();
    for font in kit["fonts"].as_array().into_iter().flatten() {
        let Some(name) = font["file"].as_str().filter(|v| !v.is_empty()) else {
            continue;
        };
        let name_family = font["family"].as_str().unwrap_or("");
        let weight = font["weight"]
            .as_u64()
            .filter(|v| (100..=1000).contains(v))
            .map(|v| v.to_string());
        if !family(name_family) {
            out.warnings.push(format!("字体名称无效：{name_family}"));
            continue;
        }
        let Some((format, _)) = font_format(name) else {
            out.warnings.push(format!("字体格式不支持：{name}"));
            continue;
        };
        match super::resources::file(pool, root, project, scope, id, name).await {
            Ok(path) => out.fonts.push(Face {
                family: name_family.into(),
                weight,
                format: format.into(),
                file: path.file_name().unwrap().to_string_lossy().into(),
                style: None,
            }),
            Err(Error::NotFound) => out.warnings.push(format!("品牌字体文件不存在：{name}")),
            Err(error) => return Err(error),
        }
    }
    for slot in ["light", "dark", "mark"] {
        let Some(name) = kit["logos"][slot].as_str().filter(|v| !v.is_empty()) else {
            continue;
        };
        if logo_mime(name).is_none() {
            out.warnings.push(format!("标志格式不支持：{name}"));
            continue;
        }
        match super::resources::file(pool, root, project, scope, id, name).await {
            Ok(path) => {
                out.logos.insert(
                    slot.into(),
                    path.file_name().unwrap().to_string_lossy().into(),
                );
            }
            Err(Error::NotFound) => out.warnings.push(format!("品牌标志文件不存在：{name}")),
            Err(error) => return Err(error),
        }
    }
    Ok(out)
}
async fn data(root: &Path, file: &str, mime: &str, max: usize) -> Result<String, String> {
    let (hash, ext) = file.split_once('.').ok_or("品牌文件标识无效")?;
    if hash.len() != 64
        || !hash.bytes().all(|b| b.is_ascii_hexdigit())
        || !ext.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err("品牌文件标识无效".into());
    }
    let mut handle = tokio::fs::File::open(root.join("editor-brand-assets").join(file))
        .await
        .map_err(|_| "品牌文件不可读取")?
        .take(max as u64 + 1);
    let mut bytes = Vec::new();
    handle
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "品牌文件不可读取")?;
    if bytes.is_empty() || bytes.len() > max {
        return Err(format!("文件为空或超过 {} KiB", max / 1024));
    }
    if mime == "image/svg+xml" {
        super::svg::size(std::str::from_utf8(&bytes).map_err(|_| "SVG 文件编码无效")?)?;
    }
    let valid_image = match mime {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        _ => true,
    };
    if !valid_image {
        return Err("标志文件格式与扩展名不符".into());
    }
    if mime.starts_with("font/")
        && !bytes.starts_with(b"wOF2")
        && !bytes.starts_with(b"wOFF")
        && !bytes.starts_with(b"OTTO")
        && !bytes.starts_with(&[0, 1, 0, 0])
    {
        return Err("字体文件头无效".into());
    }
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}
pub async fn materialize(root: &Path, snapshot: &Snapshot) -> RenderAssets {
    let mut out = RenderAssets {
        fonts: bundled(),
        logos: BTreeMap::new(),
        warnings: snapshot.warnings.clone(),
    };
    for face in &snapshot.fonts {
        let Some((_, mime)) = font_format(&face.file) else {
            continue;
        };
        match data(root, &face.file, mime, 2 * 1024 * 1024).await {
            Ok(url) => out.fonts.push(FontSource {
                family: face.family.clone(),
                weight: face.weight.clone(),
                format: face.format.clone(),
                url,
                style: face.style.clone(),
            }),
            Err(error) => out.warnings.push(format!("字体 {}：{error}", face.family)),
        }
    }
    for (slot, file) in &snapshot.logos {
        let Some(mime) = logo_mime(file) else {
            continue;
        };
        match data(root, file, mime, 400 * 1024).await {
            Ok(url) => {
                out.logos.insert(slot.clone(), url);
            }
            Err(error) => out.warnings.push(format!("品牌标志 {slot}：{error}")),
        }
    }
    out
}
pub async fn resources(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
) -> Result<RenderAssets, Error> {
    Ok(materialize(root, &snapshot(pool, root, project).await?).await)
}
