use biliup::uploader::bilibili::{Credit, ResponseData, Studio};
use pyo3::prelude::*;
use pyo3::pyclass;

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
    Bda,
    Txa,
    Alia,
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
            P::Bda => C::Bda,
            P::Txa => C::Txa,
            P::Alia => C::Alia,
        }
    }
}

#[derive(FromPyObject)]
pub struct PyCredit {
    #[pyo3(item("type"))]
    type_id: i8,
    #[pyo3(item("raw_text"))]
    raw_text: String,
    #[pyo3(item("biz_id"))]
    biz_id: Option<String>,
}

#[derive(Builder)]
pub struct StudioPre {
    video_path: Vec<PathBuf>,
    cookie_file: PathBuf,
    line: Option<UploadLine>,
    limit: usize,
    title: String,
    tid: u16,
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
