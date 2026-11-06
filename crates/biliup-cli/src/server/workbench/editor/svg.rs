//! SVG metadata, adapted from Uncut's export/svg.ts (MIT). Images never execute as documents.
use regex::Regex;
pub const MAX_BYTES: usize = 10 * 1024 * 1024;
pub fn size(text: &str) -> Result<(u32, u32), String> {
    if text.len() > MAX_BYTES
        || text.to_ascii_lowercase().contains("<!entity")
        || text.to_ascii_lowercase().contains("<!doctype")
    {
        return Err("SVG 超过大小限制或包含不支持的文档声明".into());
    }
    let root = Regex::new(r"(?is)^\s*(?:<\?xml[^>]*>\s*)?(?:<!--.*?-->\s*)*<svg\b([^>]*)>")
        .unwrap()
        .captures(text)
        .ok_or("SVG 根元素无效")?;
    if !Regex::new(r"(?is)</svg>\s*$").unwrap().is_match(text) {
        return Err("SVG 文档不完整".into());
    }
    let attr = |name: &str| -> Option<String> {
        let pattern = format!(r#"(?i)\s{name}\s*=\s*(?:"([^"]*)"|'([^']*)')"#);
        let re = Regex::new(&pattern).ok()?;
        let c = re.captures(&root[1])?;
        Some(c.get(1).or_else(|| c.get(2))?.as_str().to_string())
    };
    let number = |v: Option<String>| -> Option<f64> {
        let v = v?;
        let value = v.trim().trim_end_matches("px").trim().parse::<f64>().ok()?;
        (value.is_finite() && value > 0.).then_some(value)
    };
    let width = number(attr("width"));
    let height = number(attr("height"));
    let view = attr("viewBox")
        .map(|v| {
            v.split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|_| "SVG viewBox 无效")?;
    let (width, height) = match view
        .filter(|v| v.len() == 4 && v.iter().all(|n| n.is_finite()) && v[2] > 0. && v[3] > 0.)
    {
        Some(v) => match (width, height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, w * v[3] / v[2]),
            (None, Some(h)) => (h * v[2] / v[3], h),
            _ => (v[2], v[3]),
        },
        None => (width.unwrap_or(300.), height.unwrap_or(150.)),
    };
    if !(1. ..=16384.).contains(&width) || !(1. ..=16384.).contains(&height) {
        return Err("SVG 尺寸超出范围".into());
    }
    Ok((width.round() as u32, height.round() as u32))
}
