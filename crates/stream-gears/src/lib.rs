mod danmaku;
mod login;
mod server;
mod uploader;

use pyo3::prelude::*;
use time::macros::format_description;
use uploader::{PyCredit, StudioPre};

use crate::uploader::UploadLine;
use axum::http::HeaderMap;
use biliup::credential::Credential;
use biliup::downloader::extractor::CallbackFn;
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use pyo3::types::{PyList, PyType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, info};

use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup_cli::server::common::construct_headers;
use pyo3::exceptions::PyRuntimeError;
use tracing_subscriber::layer::SubscriberExt;

#[tokio::main]
pub async fn download_with_hook(
    url: &str,
    headers: HeaderMap,
    file_name: &str,
    segment: Segmentable,
    file_name_hook: CallbackFn,
    proxy: Option<&str>,
) -> PyResult<()> {
    let client = StatelessClient::new(headers, proxy);
    let response = client.retryable(url).await.unwrap();
    let mut connection = Connection::new(response);
    // let buf = &mut [0u8; 9];
    let bytes = connection.read_frame(9).await.unwrap();
    // response.read_exact(buf)?;
    // let out = File::create(format!("{}.flv", file_name)).expect("Unable to create file.");
    // let mut writer = BufWriter::new(out);
    // let mut buf = [0u8; 8 * 1024];
    // response.copy_to(&mut writer)?;
    // io::copy(&mut resp, &mut out).expect("Unable to copy the content.");
    match header(&bytes) {
        Ok((_i, header)) => {
            debug!("header: {header:#?}");
            info!("Downloading {}...", url);
            let file = LifecycleFile::with_hook(file_name, "flv", file_name_hook);
            httpflv::download(connection, file, segment).await;
        }
        Err(nom::Err::Incomplete(needed)) => {
            error!("needed: {needed:?}")
        }
        Err(e) => {
            error!("{e}");
            let file = LifecycleFile::with_hook(file_name, "ts", file_name_hook);
            hls::download(url, &client, file, segment).await.unwrap();
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
#[pyclass(set_all)]
pub struct PySegment {
    // #[pyo3(attribute("time"))]
    time: Option<u64>,
    // #[pyo3(attribute("size"))]
    size: Option<u64>,
}

#[pymethods]
impl PySegment {
    #[new]
    fn new() -> Self {
        PySegment {
            time: None,
            size: None,
        }
    }
}

#[pyfunction]
#[pyo3(signature = (url,header_map,file_name,segment,proxy = None))]
fn download(
    py: Python<'_>,
    url: &str,
    header_map: HashMap<String, String>,
    file_name: &str,
    segment: PySegment,
    proxy: Option<String>,
) -> PyResult<()> {
    download_with_callback(py, url, header_map, file_name, segment, None, proxy)
}

#[pyfunction]
#[pyo3(signature = (url,header_map,file_name,segment,file_name_callback_fn = None,proxy = None))]
fn download_with_callback(
    py: Python<'_>,
    url: &str,
    header_map: HashMap<String, String>,
    file_name: &str,
    segment: PySegment,
    file_name_callback_fn: Option<Py<PyType>>,
    proxy: Option<String>,
) -> PyResult<()> {
    py.detach(|| {
        let map = construct_headers(&header_map);
        // 输出到控制台中
        // use of deprecated function `time::util::local_offset::set_soundness`: no longer needed; TZ is refreshed manually
        // unsafe {
        //     time::util::local_offset::set_soundness(time::util::local_offset::Soundness::Unsound);
        // }
        let local_time = tracing_subscriber::fmt::time::LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ));
        let formatting_layer = tracing_subscriber::FmtSubscriber::builder()
            // will be written to stdout.
            // builds the subscriber.
            .with_timer(local_time.clone())
            .finish();
        let file_appender = tracing_appender::rolling::never("", "download.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(local_time)
            .with_writer(non_blocking);

        // println!("Input segment: {:?}", segment);
        // println!("Input segment time: {:?}, size: {:?}", segment.time, segment.size);

        let segmentable = match (segment.time, segment.size) {
            (Some(time), Some(size)) => {
                // 已支持同时创建时间和大小
                Segmentable::new(Some(Duration::from_secs(time)), Some(size))
            }
            (Some(time), None) => Segmentable::new(Some(Duration::from_secs(time)), None),
            (None, Some(size)) => Segmentable::new(None, Some(size)),
            (None, None) => {
                // 如果都没有，使用默认值
                Segmentable::default()
            }
        };

        let file_name_hook = file_name_callback_fn.map(|callback_fn| -> CallbackFn {
            Box::new(move |fmt_file_name| {
                Python::attach(|py| match callback_fn.call1(py, (fmt_file_name,)) {
                    Ok(_) => {}
                    Err(_) => {
                        tracing::error!("Unable to invoke the callback function.")
                    }
                })
            })
        });

        let collector = formatting_layer.with(file_layer);
        tracing::subscriber::with_default(collector, || -> PyResult<()> {
            match download_with_hook(
                url,
                map,
                file_name,
                segmentable,
                file_name_hook.unwrap_or(Box::new(|_| {})),
                proxy.as_deref(),
            ) {
                Ok(res) => Ok(res),
                Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "{}",
                    err
                ))),
            }
        })
    })
}

#[pyfunction]
fn login_by_cookies(file: String, proxy: Option<String>) -> PyResult<bool> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { login::login_by_cookies(&file, proxy.as_deref()).await });
    match result {
        Ok(_) => Ok(true),
        Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "{}",
            err
        ))),
    }
}

#[pyfunction]
fn send_sms(country_code: u32, phone: u64, proxy: Option<String>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result =
        rt.block_on(async { login::send_sms(country_code, phone, proxy.as_deref()).await });
    match result {
        Ok(res) => Ok(res.to_string()),
        Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "{}",
            err
        ))),
    }
}

#[pyfunction]
fn login_by_sms(code: u32, ret: String, proxy: Option<String>) -> PyResult<bool> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        login::login_by_sms(code, serde_json::from_str(&ret).unwrap(), proxy.as_deref()).await
    });
    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[pyfunction]
fn get_qrcode(proxy: Option<String>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { login::get_qrcode(proxy.as_deref()).await });
    match result {
        Ok(res) => Ok(res.to_string()),
        Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "{}",
            err
        ))),
    }
}

#[pyfunction]
fn login_by_qrcode(ret: String, proxy: Option<String>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let info = Credential::new(proxy.as_deref())
            .login_by_qrcode(serde_json::from_str(&ret).unwrap())
            .await?;
        let res = serde_json::to_string_pretty(&info).unwrap();
        Ok::<_, biliup::error::Kind>(res)
    })
    .map_err(|err| pyo3::exceptions::PyRuntimeError::new_err(format!("{:#?}", err)))
}

#[pyfunction]
fn login_by_web_cookies(
    sess_data: String,
    bili_jct: String,
    proxy: Option<String>,
) -> PyResult<bool> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        login::login_by_web_cookies(&sess_data, &bili_jct, proxy.as_deref()).await
    });
    match result {
        Ok(_) => Ok(true),
        Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "{}",
            err
        ))),
    }
}

#[pyfunction]
fn login_by_web_qrcode(
    sess_data: String,
    dede_user_id: String,
    proxy: Option<String>,
) -> PyResult<bool> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        login::login_by_web_qrcode(&sess_data, &dede_user_id, proxy.as_deref()).await
    });
    match result {
        Ok(_) => Ok(true),
        Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
            "{}",
            err
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (video_path, cookie_file, title, tid=171, tag="".to_string(), copyright=2, source="".to_string(), desc="".to_string(), dynamic="".to_string(), cover="".to_string(), dolby=0, lossless_music=0, no_reprint=0, charging_pay=0, up_close_reply=false, up_selection_reply=false, up_close_danmu=false, limit=3, desc_v2=vec![], dtime=None, line=None, extra_fields="".to_string(), submit=None, proxy=None))]
fn upload(
    py: Python<'_>,
    video_path: Vec<PathBuf>,
    cookie_file: PathBuf,
    title: String,
    tid: u16,
    tag: String,
    copyright: u8,
    source: String,
    desc: String,
    dynamic: String,
    cover: String,
    dolby: u8,
    lossless_music: u8,
    no_reprint: u8,
    charging_pay: u8,
    up_close_reply: bool,
    up_selection_reply: bool,
    up_close_danmu: bool,
    limit: usize,
    desc_v2: Vec<PyCredit>,
    dtime: Option<u32>,
    line: Option<UploadLine>,
    extra_fields: Option<String>,
    submit: Option<String>,
    proxy: Option<String>,
) -> PyResult<()> {
    py.detach(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        // 输出到控制台中
        // use of deprecated function `time::util::local_offset::set_soundness`: no longer needed; TZ is refreshed manually
        // unsafe {
        //     time::util::local_offset::set_soundness(time::util::local_offset::Soundness::Unsound);
        // }
        let local_time = tracing_subscriber::fmt::time::LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ));
        let formatting_layer = tracing_subscriber::FmtSubscriber::builder()
            // will be written to stdout.
            // builds the subscriber.
            .with_timer(local_time.clone())
            .finish();
        let file_appender = tracing_appender::rolling::never("", "upload.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_timer(local_time)
            .with_writer(non_blocking);

        let collector = formatting_layer.with(file_layer);

        tracing::subscriber::with_default(collector, || -> PyResult<()> {
            let studio_pre = StudioPre::builder()
                .video_path(video_path)
                .cookie_file(cookie_file)
                .maybe_line(line)
                .limit(limit)
                .title(title)
                .tid(tid)
                .tag(tag)
                .copyright(copyright)
                .source(source)
                .desc(desc)
                .dynamic(dynamic)
                .cover(cover)
                .maybe_dtime(dtime)
                .dolby(dolby)
                .lossless_music(lossless_music)
                .no_reprint(no_reprint)
                .charging_pay(charging_pay)
                .up_close_reply(up_close_reply)
                .up_selection_reply(up_selection_reply)
                .up_close_danmu(up_close_danmu)
                .desc_v2_credit(desc_v2)
                .extra_fields(parse_extra_fields(extra_fields))
                .build();

            // let submit = match submit {
            //     Some(value) => SubmitOption::from_str(&value, true).unwrap(),
            //     None => SubmitOption::App,
            // };

            match rt.block_on(uploader::upload(
                studio_pre,
                submit.as_deref(),
                proxy.as_deref(),
            )) {
                Ok(_) => Ok(()),
                // Ok(_) => {  },
                Err(err) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "{}",
                    err
                ))),
            }
        })
    })
}

#[pyfunction]
pub fn main_loop(py: Python<'_>) -> PyResult<()> {
    // 获取 Python 的 sys.argv
    let sys = py.import("sys")?;
    let bound = sys.getattr("argv")?;
    let argv: &Bound<PyList> = bound.cast()?;

    let mut args: Vec<String> = argv
        .iter()
        .map(|x| x.extract::<String>())
        .collect::<PyResult<Vec<_>>>()?;

    // if args.len() == 1 {
    //     args.push("server".to_string());
    // }
    match args.as_slice() {
        &[] => todo!(),
        &[_] => {
            args.push("server".to_string());
        }
        [_, command, ..] => {
            if command == "start" {
                args[1] = "server".to_string();
            }
        }
    }

    py.detach(|| {
        server::_main(args.as_slice()).map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))
    })

    // for item in plugin_list.iter() {
    //     // 每个元素都是一个类（type）
    //     let ty: &Bound<PyType> = item.downcast()?;
    //
    //     // 确认是否是 DownloadBase 的子类
    //     if ty.is_subclass(download_base)? {
    //         let name: String = ty.getattr("__name__")?.extract()?;
    //         println!("发现插件类: {name}");
    //
    //         // 如果要实例化（若不是抽象类且构造器无参）
    //         // let obj = ty.call0()?;
    //         // 或者需要参数：ty.call1((arg1, arg2,))?;
    //
    //         // 如果要调用方法（示例）
    //         // if let Ok(method) = obj.getattr("download") {
    //         //     let result = method.call0()?;
    //         //     println!("download() 返回: {}", result.repr()?.extract::<String>()?);
    //         // }
    //     }
    // }
}

/// A Python module implemented in Rust.
#[pymodule]
fn stream_gears(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // let file_appender = tracing_appender::rolling::daily("", "upload.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // tracing_subscriber::fmt()
    //     .with_writer(non_blocking)
    //     .init();
    m.add_function(wrap_pyfunction!(upload, m)?)?;
    // m.add_function(wrap_pyfunction!(upload_by_app, m)?)?;
    m.add_function(wrap_pyfunction!(download, m)?)?;
    m.add_function(wrap_pyfunction!(download_with_callback, m)?)?;
    m.add_function(wrap_pyfunction!(login_by_cookies, m)?)?;
    m.add_function(wrap_pyfunction!(send_sms, m)?)?;
    m.add_function(wrap_pyfunction!(login_by_qrcode, m)?)?;
    m.add_function(wrap_pyfunction!(get_qrcode, m)?)?;
    m.add_function(wrap_pyfunction!(login_by_sms, m)?)?;
    m.add_function(wrap_pyfunction!(login_by_web_cookies, m)?)?;
    m.add_function(wrap_pyfunction!(login_by_web_qrcode, m)?)?;
    m.add_function(wrap_pyfunction!(main_loop, m)?)?;
    m.add_function(wrap_pyfunction!(server::config_bindings, m)?)?;
    m.add_class::<UploadLine>()?;
    m.add_class::<PySegment>()?;
    Ok(())
}

fn parse_extra_fields(s: Option<String>) -> HashMap<String, serde_json::Value> {
    match s {
        Some(value) => serde_json::from_str(&value).unwrap_or_default(), // 如果有值，尝试解析
        None => HashMap::new(), // 如果是 None，直接返回空的 HashMap
    }
}
