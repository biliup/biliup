//! Immutable graphic sources. References, undo and exported revisions never observe overwritten HTML.
use super::{ConnectionPool, Error, invalid};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_BYTES: usize = 256 * 1024;
const MAX_DOC_BYTES: usize = 1_400_000;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub name: Option<String>,
    pub html: Option<String>,
    pub from: Option<String>,
    pub edits: Option<Vec<Replacement>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    old: String,
    new: String,
}
pub(crate) struct Prepared {
    pub src: String,
    pub name: String,
    pub html: String,
}
pub fn references(project: &Value) -> BTreeSet<String> {
    project["tracks"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|track| track["items"].as_array().into_iter().flatten())
        .filter(|item| item["type"] == "graphic")
        .filter_map(|item| item["src"].as_str().map(str::to_string))
        .collect()
}
pub async fn write(pool: &ConnectionPool, project: i64, input: Write) -> Result<Value, Error> {
    let Prepared { src, name, html } = prepare(pool, project, input).await?;
    sqlx::query("INSERT OR IGNORE INTO editor_graphics(project_id,src,name,html,created_at) VALUES(?,?,?,?,?)").bind(project).bind(&src).bind(&name).bind(&html).bind(super::now_ms()).execute(pool).await?;
    Ok(json!({"src":src,"name":name}))
}
pub(crate) async fn prepare(
    pool: &ConnectionPool,
    project: i64,
    input: Write,
) -> Result<Prepared, Error> {
    super::get(pool, project).await?;
    let html = match (input.html, input.from, input.edits) {
        (Some(html), None, None) => html,
        (None, Some(from), Some(edits)) if !edits.is_empty() && edits.len() <= 100 => {
            let mut html = read(pool, project, &from).await?["html"]
                .as_str()
                .unwrap()
                .to_string();
            for edit in edits {
                if edit.old.is_empty() || html.matches(&edit.old).count() != 1 {
                    return Err(invalid("每次替换必须唯一匹配原图形中的一处文本"));
                }
                html = html.replacen(&edit.old, &edit.new, 1);
                if html.len() > MAX_BYTES {
                    return Err(invalid("图形不能超过 256 KiB"));
                }
            }
            html
        }
        _ => return Err(invalid("请提交图形源码，或来源图形及替换内容")),
    };
    if html.trim().is_empty() || html.len() > MAX_BYTES {
        return Err(invalid("图形不能为空且不能超过 256 KiB"));
    }
    let name = input.name.unwrap_or_else(|| "动态图形".into());
    if name.len() > 240 || name.chars().any(char::is_control) {
        return Err(invalid("图形名称无效"));
    }
    resolve(&html).map_err(|error| invalid(&error))?;
    let src = format!("graphics/{:x}.html", Sha256::digest(html.as_bytes()));
    Ok(Prepared { src, name, html })
}
pub async fn read(pool: &ConnectionPool, project: i64, src: &str) -> Result<Value, Error> {
    let (name, html): (String, String) =
        sqlx::query_as("SELECT name,html FROM editor_graphics WHERE project_id=? AND src=?")
            .bind(project)
            .bind(src)
            .fetch_optional(pool)
            .await?
            .ok_or(Error::NotFound)?;
    Ok(json!({"src":src,"name":name,"html":html}))
}
pub async fn load(
    pool: &ConnectionPool,
    project: i64,
    document: &Value,
) -> Result<BTreeMap<String, String>, Error> {
    load_references(pool, project, references(document)).await
}
pub async fn load_for_render(
    pool: &ConnectionPool,
    project: i64,
    document: &Value,
    items: Option<&BTreeSet<String>>,
) -> Result<BTreeMap<String, String>, Error> {
    let sources = document["tracks"].as_array().into_iter().flatten()
        .filter(|track| track["hidden"] != true && track["kind"] != "audio")
        .flat_map(|track| track["items"].as_array().into_iter().flatten())
        .filter(|item| item["type"] == "graphic")
        .filter(|item| items.is_none_or(|ids| item["id"].as_str().is_some_and(|id| ids.contains(id))))
        .filter_map(|item| item["src"].as_str().map(str::to_string)).collect();
    load_references(pool, project, sources).await
}
async fn load_references(
    pool: &ConnectionPool,
    project: i64,
    sources: BTreeSet<String>,
) -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    let mut bytes = 0usize;
    for src in sources {
        let value = read(pool, project, &src).await?;
        let html = resolve(value["html"].as_str().unwrap()).map_err(|error| invalid(&error))?;
        bytes += html.len();
        if bytes > 32 * 1024 * 1024 {
            return Err(invalid("工程图形内容超过 32 MiB"));
        }
        out.insert(src, html);
    }
    Ok(out)
}
macro_rules! library {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../resources/editor/graphic-libs/",
            $name
        ))
    };
}
pub(crate) fn library(name: &str) -> Option<&'static str> {
    match name {
        "three.js" => Some(library!("three.js")),
        "confetti.js" => Some(library!("confetti.js")),
        "globe.js" => Some(library!("globe.js")),
        "spring.js" => Some(library!("spring.js")),
        "random.js" => Some(library!("random.js")),
        "motion.js" => Some(library!("motion.js")),
        "ui.js" => Some(library!("ui.js")),
        _ => None,
    }
}
pub fn resolve(html: &str) -> Result<String, String> {
    let marker = regex::Regex::new(
        r#"(?i)<script\s+src\s*=\s*["']biliup:lib/([^"'<>\s]+)["']\s*>\s*</script\s*>"#,
    )
    .unwrap();
    let mut seen = BTreeSet::new();
    let mut failure = None;
    let html = marker
        .replace_all(html, |capture: &regex::Captures<'_>| {
            let name = &capture[1];
            if !seen.insert(name.to_string()) {
                return String::new();
            }
            match library(name) {
                Some(code) => format!(
                    "<script>{}</script>",
                    regex::Regex::new("(?i)</script")
                        .unwrap()
                        .replace_all(code, "<\\/script")
                ),
                None => {
                    failure = Some(format!("图形库未提供：{name}"));
                    String::new()
                }
            }
        })
        .into_owned();
    if let Some(error) = failure {
        return Err(error);
    }
    if html.len() + 16_000 > MAX_DOC_BYTES {
        return Err("嵌入图形库后的文档超过大小限制".into());
    }
    Ok(html)
}
