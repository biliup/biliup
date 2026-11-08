//! Brand file downloads, adapted from Uncut brand/importer.ts and agentDownload.ts (MIT).
//! Use the shared public HTTPS client: no credentials, private addresses or browser CORS dependency.
use super::{ConnectionPool, Error, Result, invalid};
use quick_xml::{Reader, events::Event};
use regex::Regex;
use std::{path::Path, time::Duration};

const MAX_DOWNLOAD: usize = 50 * 1024 * 1024;
static DOWNLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

fn logo_font_extension(value: &str) -> bool {
    matches!(
        value,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ttf" | "otf" | "woff" | "woff2"
    )
}
pub(crate) fn mime_extension(mime: &str) -> Option<&'static str> {
    Some(match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "font/woff2" | "application/font-woff2" => "woff2",
        "font/woff" | "application/font-woff" | "application/x-font-woff" => "woff",
        "font/ttf" | "font/sfnt" | "application/x-font-ttf" => "ttf",
        "font/otf"
        | "application/x-font-otf"
        | "application/x-font-opentype"
        | "application/vnd.ms-opentype" => "otf",
        _ => return None,
    })
}

/// Validate uploads too: the same brand asset may later be inlined in a graphic.
pub(crate) fn validate(bytes: &[u8], extension: &str) -> Result<()> {
    let at = |offset: usize, signature: &[u8]| {
        bytes.get(offset..offset + signature.len()) == Some(signature)
    };
    let valid = match extension {
        "svg" => return validate_svg(bytes),
        "png" => at(0, b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => at(0, b"\xff\xd8\xff"),
        "gif" => at(0, b"GIF87a") || at(0, b"GIF89a"),
        "webp" => at(0, b"RIFF") && at(8, b"WEBP"),
        "woff" => at(0, b"wOFF"),
        "woff2" => at(0, b"wOF2"),
        "ttf" => at(0, b"\0\x01\0\0") || at(0, b"true") || at(0, b"ttcf"),
        "otf" => at(0, b"OTTO") || at(0, b"\0\x01\0\0"),
        "mp4" => at(4, b"ftyp"),
        "mov" => [b"ftyp", b"moov", b"wide", b"mdat", b"free"]
            .iter()
            .any(|s| at(4, *s)),
        "webm" => at(0, b"\x1a\x45\xdf\xa3"),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(invalid(&format!(
            "文件内容不是有效的 {} 文件",
            extension.to_uppercase()
        )))
    }
}

pub async fn read(path: &Path, max: usize) -> Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| invalid("品牌文件不存在或不可读取"))?;
    if file
        .metadata()
        .await
        .map_err(|_| invalid("无法读取品牌文件信息"))?
        .len()
        > max as u64
    {
        return Err(invalid("品牌文件超过本次读取大小限制"));
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| invalid("品牌文件读取失败"))?;
    if bytes.len() > max {
        return Err(invalid("品牌文件超过本次读取大小限制"));
    }
    Ok(bytes)
}

fn validate_svg(bytes: &[u8]) -> Result<()> {
    if bytes.len() > super::super::svg::MAX_BYTES {
        return Err(invalid("SVG 文件不能超过 10 MiB"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("SVG 编码无效"))?;
    let urls = Regex::new(r#"(?i)url\s*\(([^)]*)\)"#).unwrap();
    let safe_style = |value: &str| -> bool {
        !value.contains(['\\', '@'])
            && urls
                .captures_iter(value)
                .all(|c| c[1].trim().trim_matches(['\'', '"']).starts_with('#'))
    };
    let mut reader = Reader::from_str(text);
    let mut depth = 0usize;
    let mut root = false;
    let mut style_depth = None;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| invalid("SVG 文档格式无效"))?;
        match &event {
            Event::Start(node) | Event::Empty(node) => {
                let tag = String::from_utf8_lossy(node.local_name().as_ref()).to_ascii_lowercase();
                if depth == 0 {
                    if root || tag != "svg" {
                        return Err(invalid("SVG 根元素无效"));
                    }
                    root = true;
                }
                if matches!(
                    tag.as_str(),
                    "script"
                        | "foreignobject"
                        | "iframe"
                        | "object"
                        | "embed"
                        | "html"
                        | "link"
                        | "animate"
                        | "animatetransform"
                        | "animatemotion"
                        | "set"
                ) {
                    return Err(invalid("SVG 不能包含脚本或外部文档"));
                }
                for attr in node.attributes() {
                    let attr = attr.map_err(|_| invalid("SVG 属性无效"))?;
                    let name = String::from_utf8_lossy(attr.key.as_ref()).to_ascii_lowercase();
                    let local = name.rsplit(':').next().unwrap_or(&name);
                    let value = attr
                        .unescape_value()
                        .map_err(|_| invalid("SVG 属性编码无效"))?;
                    let value = value.trim();
                    if local.starts_with("on")
                        || local == "base"
                        || value.to_ascii_lowercase().contains("javascript:")
                    {
                        return Err(invalid("SVG 不能包含脚本或外部引用"));
                    }
                    if matches!(local, "href" | "src")
                        && !value.is_empty()
                        && !value.starts_with('#')
                        && ![
                            "data:image/png;base64,",
                            "data:image/jpeg;base64,",
                            "data:image/gif;base64,",
                            "data:image/webp;base64,",
                        ]
                        .iter()
                        .any(|prefix| value.starts_with(prefix))
                    {
                        return Err(invalid("SVG 不能引用外部资源，请选择内嵌图片或 PNG"));
                    }
                    if !name.starts_with("xmlns") && !safe_style(value) {
                        return Err(invalid("SVG 包含不支持的样式引用"));
                    }
                }
            }
            Event::Text(value) => {
                let value = value.unescape().map_err(|_| invalid("SVG 文本编码无效"))?;
                if (depth == 0 && !value.trim().is_empty())
                    || (style_depth.is_some() && !safe_style(&value))
                {
                    return Err(invalid("SVG 文本或样式无效"));
                }
            }
            Event::CData(value) => {
                if depth == 0
                    || style_depth.is_some() && !safe_style(&String::from_utf8_lossy(value))
                {
                    return Err(invalid("SVG 样式无效"));
                }
            }
            Event::DocType(_) | Event::PI(_) => {
                return Err(invalid("SVG 不能包含文档声明或处理指令"));
            }
            Event::Eof => {
                if !root || depth != 0 {
                    return Err(invalid("SVG 文档不完整"));
                }
                return Ok(());
            }
            _ => {}
        }
        // Maintain nesting separately so a self-closing SVG root is valid too.
        match event {
            Event::Start(node) => {
                depth += 1;
                if node.local_name().as_ref().eq_ignore_ascii_case(b"style") {
                    style_depth = Some(depth)
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return Err(invalid("SVG 元素嵌套无效"));
                }
                if style_depth == Some(depth) {
                    style_depth = None
                }
                depth -= 1;
            }
            _ => {}
        }
    }
}

pub async fn download(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    target: &str,
    kit: &str,
    url: &str,
) -> Result<String> {
    super::exists(pool, project).await?;
    super::scope(project, target)?;
    if !super::identifier(kit) {
        return Err(invalid("品牌标识无效"));
    }
    if url.len() > 8192 {
        return Err(invalid("文件地址过长"));
    }
    let _permit = DOWNLOADS
        .try_acquire()
        .map_err(|_| invalid("品牌文件正在下载，请稍后重试"))?;
    let (name, bytes) = tokio::time::timeout(Duration::from_secs(90), async {
        let mut response = super::super::public_http::get(url.trim(), None, None)
            .await
            .map_err(|e| invalid(&e.replace("音源", "文件")))?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(invalid(&format!(
                "文件下载失败（{}）",
                response.status().as_u16()
            )));
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let raw = response
            .url()
            .path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap_or("download");
        let decoded = urlencoding::decode(raw).map_err(|_| invalid("文件地址编码无效"))?;
        let url_ext = decoded
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let extension = mime_extension(&mime)
            .or_else(|| {
                (matches!(
                    mime.as_str(),
                    "" | "application/octet-stream" | "binary/octet-stream"
                ) && logo_font_extension(&url_ext))
                .then_some(url_ext.as_str())
            })
            .ok_or_else(|| {
                invalid(if mime == "text/html" {
                    "这是网页，请输入图片或字体文件的直接链接"
                } else {
                    "链接没有返回支持的图片或字体文件"
                })
            })?;
        let stem = decoded
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&decoded);
        let stem = stem
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '?' | '*' | '"' | '<' | '>' | '|')
                {
                    '-'
                } else {
                    c
                }
            })
            .take(80)
            .collect::<String>();
        let stem = stem.trim_matches(['.', ' ', '-']);
        let stem = if stem.is_empty() { "download" } else { stem };
        let name = format!("{stem}.{extension}");
        if response
            .content_length()
            .is_some_and(|len| len > MAX_DOWNLOAD as u64)
        {
            return Err(invalid("下载的品牌文件不能超过 50 MiB"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| invalid("品牌文件下载中断"))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_DOWNLOAD {
                return Err(invalid("下载的品牌文件不能超过 50 MiB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(invalid("下载的文件为空"));
        }
        Ok::<_, Error>((name, bytes))
    })
    .await
    .map_err(|_| invalid("品牌文件下载超时"))??;
    super::add_file(pool, root, project, target, kit, &name, &bytes).await
}
