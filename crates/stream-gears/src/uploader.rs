use biliup::uploader::bilibili::{Credit, ResponseData, Studio};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::types::PyMapping;

use crate::get_field;
use biliup::client::StatelessClient;
use biliup::error::Result as AppResult;
use biliup::uploader::{
    VideoFile,
    credential::login_by_cookies,
    line::{self, Line, Probe},
};
use bon::Builder;
use futures::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;

#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UploadLine {
    Bldsa,
    Cnbldsa,
    Andsa,
    Atdsa,
    Bda2,
    Cnbd,
    Anbd,
    Atbd,
    Tx,
    Cntx,
    Antx,
    Attx,
    Txa,
    Alia,
    Estx,
    Akbd,
}

impl UploadLine {
    fn line(self) -> Line {
        match self {
            Self::Bldsa => line::bldsa(),
            Self::Cnbldsa => line::cnbldsa(),
            Self::Andsa => line::andsa(),
            Self::Atdsa => line::atdsa(),
            Self::Bda2 => line::bda2(),
            Self::Cnbd => line::cnbd(),
            Self::Anbd => line::anbd(),
            Self::Atbd => line::atbd(),
            Self::Tx => line::tx(),
            Self::Cntx => line::cntx(),
            Self::Antx => line::antx(),
            Self::Attx => line::attx(),
            Self::Txa => line::txa(),
            Self::Alia => line::alia(),
            Self::Estx => line::estx(),
            Self::Akbd => line::akbd(),
        }
    }
}

/// `desc_v2` 的元素：`{"type", "raw_text", "biz_id"}` 形式的 dict，
/// 或带 `type_id` / `raw_text` / `biz_id` 属性的对象。`biz_id` 可省略。
pub struct PyCredit {
    type_id: i8,
    raw_text: String,
    biz_id: Option<String>,
}

impl<'a, 'py> FromPyObject<'a, 'py> for PyCredit {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        let obj = obj.to_owned();
        let type_key = if obj.cast::<PyMapping>().is_ok() {
            "type"
        } else {
            "type_id"
        };
        let required = |key: &str| -> PyResult<Bound<'py, PyAny>> {
            get_field(&obj, key)?
                .ok_or_else(|| PyTypeError::new_err(format!("desc_v2 item is missing `{key}`")))
        };
        Ok(PyCredit {
            type_id: required(type_key)?.extract()?,
            raw_text: required("raw_text")?.extract()?,
            biz_id: get_field(&obj, "biz_id")?
                .map(|value| value.extract::<Option<String>>())
                .transpose()?
                .flatten(),
        })
    }
}

#[derive(Builder)]
pub struct StudioPre {
    video_path: Vec<PathBuf>,
    cookie_file: PathBuf,
    line: Option<UploadLine>,
    limit: usize,
    title: String,
    tid: u16,
    tid_v2: Option<u32>,
    tag: String,
    copyright: u8,
    source: String,
    desc: String,
    dynamic: String,
    cover: String,
    dtime: Option<u32>,
    dolby: u8,
    lossless_music: u8,
    no_reprint: u8,
    charging_pay: u8,
    #[builder(default = false)]
    up_close_reply: bool,
    #[builder(default = false)]
    up_selection_reply: bool,
    #[builder(default = false)]
    up_close_danmu: bool,
    desc_v2_credit: Vec<PyCredit>,
    extra_fields: Option<HashMap<String, serde_json::Value>>,
}

pub async fn upload(
    studio_pre: StudioPre,
    submit: Option<&str>,
    proxy: Option<&str>,
) -> AppResult<ResponseData> {
    // let file = std::fs::File::options()
    //     .read(true)
    //     .write(true)
    //     .open(&cookie_file);
    let StudioPre {
        video_path,
        cookie_file,
        line,
        limit,
        title,
        tid,
        tid_v2,
        tag,
        copyright,
        source,
        desc,
        dynamic,
        cover,
        dtime,
        dolby,
        lossless_music,
        no_reprint,
        charging_pay,
        up_close_reply,
        up_selection_reply,
        up_close_danmu,
        desc_v2_credit,
        extra_fields,
    } = studio_pre;

    let bilibili = login_by_cookies(&cookie_file, proxy)
        .await
        .map_err(|error| {
            biliup::error::Kind::Custom(format!("cookies file {}: {error}", cookie_file.display()))
        })?;
    let client = StatelessClient::default();
    let line = match line {
        Some(line) => line.line(),
        None => Probe::probe(&client.client).await.unwrap_or_else(|error| {
            tracing::warn!(%error, "AUTO line probe failed; using default");
            Line::default()
        }),
    };
    let mut videos = Vec::with_capacity(video_path.len());
    for path in video_path {
        let parcel = line.pre_upload(&bilibili, VideoFile::new(&path)?).await?;
        videos.push(
            parcel
                .upload(client.clone(), limit, |stream| {
                    stream.map(|chunk| {
                        chunk
                            .map(|bytes| {
                                let len = bytes.len();
                                (bytes, len)
                            })
                            .map_err(Into::into)
                    })
                })
                .await?,
        );
    }

    let mut desc_v2 = Vec::new();
    for credit in desc_v2_credit {
        desc_v2.push(Credit {
            type_id: credit.type_id,
            raw_text: credit.raw_text,
            biz_id: credit.biz_id,
        });
    }

    let mut studio: Studio = Studio::builder()
        .desc(desc)
        .maybe_dtime(dtime)
        .copyright(copyright)
        .cover(cover)
        .dynamic(dynamic)
        .source(source)
        .tag(tag)
        .tid(tid)
        .maybe_tid_v2(tid_v2)
        .title(title)
        .videos(videos)
        .dolby(dolby)
        .lossless_music(lossless_music)
        .no_reprint(no_reprint)
        .charging_pay(charging_pay)
        .up_close_reply(up_close_reply)
        .up_selection_reply(up_selection_reply)
        .up_close_danmu(up_close_danmu)
        .desc_v2(desc_v2)
        .maybe_extra_fields(extra_fields)
        .build();

    if !studio.cover.is_empty() {
        let url = bilibili.cover_up(&std::fs::read(&studio.cover)?).await?;
        println!("{url}");
        studio.cover = url;
    }

    match submit.unwrap_or("web").to_ascii_lowercase().as_str() {
        "app" => bilibili.submit_by_app(&studio, None).await,
        "bcutandroid" => bilibili.submit_by_bcut_android(&studio, None).await,
        _ => bilibili.submit_by_web(&studio, None).await,
    }
}
