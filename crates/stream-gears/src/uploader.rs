use biliup::uploader::bilibili::{Credit, ResponseData, Studio};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::types::PyMapping;

use crate::get_field;
use biliup_cli::server::common;
use biliup_cli::server::common::upload::submit_to_bilibili;
use biliup_cli::server::errors::{AppError, AppResult};
use bon::Builder;
use error_stack::ResultExt;
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

impl From<UploadLine> for biliup_cli::UploadLine {
    fn from(val: UploadLine) -> Self {
        use UploadLine as P;
        use biliup_cli::UploadLine as C;
        match val {
            P::Bldsa => C::Bldsa,
            P::Cnbldsa => C::Cnbldsa,
            P::Andsa => C::Andsa,
            P::Atdsa => C::Atdsa,
            P::Bda2 => C::Bda2,
            P::Cnbd => C::Cnbd,
            P::Anbd => C::Anbd,
            P::Atbd => C::Atbd,
            P::Tx => C::Tx,
            P::Cntx => C::Cntx,
            P::Antx => C::Antx,
            P::Attx => C::Attx,
            P::Txa => C::Txa,
            P::Alia => C::Alia,
            P::Estx => C::Estx,
            P::Akbd => C::Akbd,
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

    let (bilibili, videos) = common::upload::upload(
        &cookie_file,
        proxy,
        line.map(Into::into),
        video_path.as_slice(),
        limit,
    )
    .await?;

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
        let url = bilibili
            .cover_up(
                &std::fs::read(&studio.cover)
                    .change_context_lazy(|| AppError::Unknown)
                    .attach_with(|| format!("cover: {}", studio.cover))?,
            )
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        println!("{url}");
        studio.cover = url;
    }

    submit_to_bilibili(&bilibili, &studio, submit).await
}
