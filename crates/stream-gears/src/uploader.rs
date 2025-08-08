use anyhow::{Context, Result};
use biliup::client::StatelessClient;
use biliup::error::Kind;
use biliup::uploader::bilibili::{Credit, ResponseData, Studio};
use biliup::uploader::credential::login_by_cookies;
use biliup::uploader::line::Probe;
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, line};
use futures::StreamExt;
use pyo3::prelude::*;
use pyo3::pyclass;

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;
use tracing::info;

use typed_builder::TypedBuilder;

#[pyclass]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UploadLine {
    Bda2,
    Qn,
    Bldsa,
    Tx,
    Txa,
    Bda,
    Alia,
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

#[derive(TypedBuilder)]
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
    #[builder(default)]
    extra_fields: Option<HashMap<String, serde_json::Value>>,
}

pub async fn upload(studio_pre: StudioPre, submit: Option<&str>, proxy: Option<&str>) -> Result<ResponseData> {
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

    let bilibili = login_by_cookies(&cookie_file, proxy).await;
    let bilibili = match bilibili {
        Err(Kind::IO(_)) => bilibili.with_context(|| {
            String::from("open cookies file: ") + &cookie_file.to_string_lossy()
        })?,
        _ => bilibili?,
    };

    let client = StatelessClient::default();
    let mut videos = Vec::new();
    let line = match line {
        Some(UploadLine::Bda2) => line::bda2(),
        Some(UploadLine::Qn) => line::qn(),
        Some(UploadLine::Bda) => line::bda(),
        Some(UploadLine::Tx) => line::tx(),
        Some(UploadLine::Txa) => line::txa(),
        Some(UploadLine::Bldsa) => line::bldsa(),
        Some(UploadLine::Alia) => line::alia(),
        _ => Probe::probe(&client.client).await.unwrap_or_default(),
    };
    for video_path in video_path {
        println!("{:?}", video_path.canonicalize()?.to_str());
        info!("{line:?}");
        let video_file = VideoFile::new(&video_path)?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let uploader = line.pre_upload(&bilibili, video_file).await?;

        let instant = Instant::now();

        let video = uploader
            .upload(client.clone(), limit, |vs| {
                vs.map(|vs| {
                    let chunk = vs?;
                    let len = chunk.len();
                    Ok((chunk, len))
                })
            })
            .await?;
        let t = instant.elapsed().as_millis();
        info!(
            "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
            t as f64 / 1000.,
            total_size as f64 / 1000. / t as f64
        );
        videos.push(video);
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
        .dtime(dtime)
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
        .desc_v2(Some(desc_v2))
        .extra_fields(extra_fields)
        .build();

    if !studio.cover.is_empty() {
        let url = bilibili
            .cover_up(
                &std::fs::read(&studio.cover)
                    .with_context(|| format!("cover: {}", studio.cover))?,
            )
            .await?;
        println!("{url}");
        studio.cover = url;
    }

    let submit = match submit {
        Some(submit) => SubmitOption::from_str(submit).unwrap_or(SubmitOption::App),
        _ => SubmitOption::App,
    };

    let submit_result = match submit {
        SubmitOption::BCutAndroid => bilibili.submit_by_bcut_android(&studio, proxy).await?,
        _ => bilibili.submit_by_app(&studio, proxy).await?,
    };
    Ok(submit_result)
}