mod login;
mod server;
mod uploader;

use pyo3::prelude::*;
use time::macros::format_description;
use uploader::{PyCredit, StudioPre};

use crate::uploader::UploadLine;
use axum::http::HeaderMap;
use biliup::credential::Credential;
use biliup::downloader::util::{CallbackFn, LifecycleFile, Segmentable};
use biliup::downloader::{hls, httpflv};
use biliup::uploader::credential::save_login_info;
use pyo3::types::{PyList, PyMapping};
use std::collections::HashMap;
use std::fmt::Display;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info};

use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::header;
use biliup::downloader::httpflv::Connection;
use biliup_cli::server::common::construct_headers;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::{PyTypeError, PyValueError};
use tracing_subscriber::layer::SubscriberExt;

pyo3::create_exception!(
    stream_gears,
    StreamGearsError,
    PyRuntimeError,
    "Raised when a download, login or upload fails. Subclass of RuntimeError."
);

/// `{:#}` 让 `error_stack::Report` 带上整条原因链，而不只是最外层的 "Unknown Error"。
fn stream_gears_err(err: impl Display) -> PyErr {
    StreamGearsError::new_err(format!("{err:#}"))
}

fn new_runtime() -> PyResult<tokio::runtime::Runtime> {
    tokio::runtime::Runtime::new()
        .map_err(|e| StreamGearsError::new_err(format!("failed to start tokio runtime: {e}")))
}

fn parse_json_arg(name: &str, value: &str) -> PyResult<serde_json::Value> {
    serde_json::from_str(value)
        .map_err(|e| PyValueError::new_err(format!("{name} is not valid JSON: {e}")))
}

/// 从 mapping 里按键、从其它对象上按属性取字段；字段不存在时返回 `None`。
pub(crate) fn get_field<'py>(
    obj: &Bound<'py, PyAny>,
    key: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    if let Ok(mapping) = obj.cast::<PyMapping>() {
        return if mapping.contains(key)? {
            mapping.get_item(key).map(Some)
        } else {
            Ok(None)
        };
    }
    if obj.hasattr(key)? {
        obj.getattr(key).map(Some)
    } else {
        Ok(None)
    }
}

pub async fn download_with_hook(
    url: &str,
    headers: HeaderMap,
    file_name: &str,
    segment: Segmentable,
    file_name_hook: CallbackFn<'_>,
    proxy: Option<&str>,
) -> PyResult<()> {
    let client = StatelessClient::new(headers, proxy);
    let response = client
        .retryable(url)
        .await
        .map_err(|e| StreamGearsError::new_err(format!("failed to connect to {url}: {e}")))?;
    let mut connection = Connection::new(response);
    let bytes = connection
        .read_frame(9)
        .await
        .map_err(|e| StreamGearsError::new_err(format!("failed to read from {url}: {e}")))?;
    match header(&bytes) {
        Ok((_i, header)) => {
            debug!("header: {header:#?}");
            info!("Downloading {}...", url);
            let file = LifecycleFile::with_hook(file_name, "flv", file_name_hook);
            httpflv::parse_flv(connection, file, segment, None)
                .await
                .map_err(|e| {
                    StreamGearsError::new_err(format!("FLV download from {url} failed: {e}"))
                })?;
            info!("Done... {}", file_name);
        }
        Err(nom::Err::Incomplete(needed)) => {
            return Err(StreamGearsError::new_err(format!(
                "incomplete FLV header from {url}: got {} bytes, needed {needed:?} more",
                bytes.len()
            )));
        }
        Err(e) => {
            error!("{e}");
            let file = LifecycleFile::with_hook(file_name, "ts", file_name_hook);
            hls::download(url, &client, file, segment, None)
                .await
                .map_err(|e| {
                    StreamGearsError::new_err(format!("HLS download from {url} failed: {e}"))
                })?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
#[pyclass(get_all, set_all)]
pub struct PySegment {
    time: Option<u64>,
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

/// `download` 的 `segment` 参数：`PySegment`、带 `time` / `size` 键的 dict，
/// 或带 `time` / `size` 属性的对象。
struct SegmentArg {
    time: Option<u64>,
    size: Option<u64>,
}

impl<'a, 'py> FromPyObject<'a, 'py> for SegmentArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(segment) = obj.cast::<PySegment>() {
            let segment = segment.borrow();
            return Ok(SegmentArg {
                time: segment.time,
                size: segment.size,
            });
        }
        let obj = obj.to_owned();
        if obj.cast::<PyMapping>().is_err() && !(obj.hasattr("time")? || obj.hasattr("size")?) {
            return Err(PyTypeError::new_err(format!(
                "segment must be a PySegment, a dict or an object with `time` / `size` attributes, not {}",
                obj.get_type().name()?
            )));
        }
        let field = |key| -> PyResult<Option<u64>> {
            Ok(get_field(&obj, key)?
                .map(|value| value.extract::<Option<u64>>())
                .transpose()?
                .flatten())
        };
        Ok(SegmentArg {
            time: field("time")?,
            size: field("size")?,
        })
    }
}

#[pyfunction]
#[pyo3(signature = (url,header_map,file_name,segment,proxy = None))]
fn download(
    py: Python<'_>,
    url: &str,
    header_map: HashMap<String, String>,
    file_name: &str,
    segment: SegmentArg,
    proxy: Option<String>,
) -> PyResult<()> {
    download_with_callback(py, url, header_map, file_name, segment, None, proxy)
}

/// 回调在每个分段文件写完时被调用。回调抛出的异常会立即记进日志，下载继续；
/// 下载结束后重新抛出第一个回调异常。下载本身也失败时抛 `StreamGearsError`，
/// 回调异常挂在它的 `__cause__` 上。
#[pyfunction]
#[pyo3(signature = (url,header_map,file_name,segment,file_name_callback_fn = None,proxy = None))]
fn download_with_callback(
    py: Python<'_>,
    url: &str,
    header_map: HashMap<String, String>,
    file_name: &str,
    segment: SegmentArg,
    file_name_callback_fn: Option<Py<PyAny>>,
    proxy: Option<String>,
) -> PyResult<()> {
    if let Some(callback_fn) = &file_name_callback_fn
        && !callback_fn.bind(py).is_callable()
    {
        return Err(PyTypeError::new_err(format!(
            "file_name_callback_fn must be callable, not {}",
            callback_fn.bind(py).get_type().name()?
        )));
    }
    let callback_error: Arc<Mutex<Option<PyErr>>> = Arc::default();
    let file_name_hook = file_name_callback_fn.map(|callback_fn| -> CallbackFn {
        let callback_error = Arc::clone(&callback_error);
        Box::new(move |fmt_file_name| {
            Python::attach(|py| {
                if let Err(err) = callback_fn.call1(py, (fmt_file_name,)) {
                    error!("file_name_callback_fn({fmt_file_name:?}) raised {err}");
                    if let Ok(mut first) = callback_error.lock() {
                        first.get_or_insert(err);
                    }
                }
            })
        })
    });

    let result = py.detach(|| {
        let map = construct_headers(&header_map).map_err(StreamGearsError::new_err)?;
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

        let collector = formatting_layer.with(file_layer);
        tracing::subscriber::with_default(collector, || -> PyResult<()> {
            let rt = new_runtime()?;
            // `httpflv::parse_flv` 遇到畸形的 tag 数据仍会 `expect` panic；不在这里拦下，
            // Python 侧收到的是 `except Exception` 接不住的 PanicException。
            panic::catch_unwind(AssertUnwindSafe(|| {
                rt.block_on(download_with_hook(
                    url,
                    map,
                    file_name,
                    segmentable,
                    file_name_hook.unwrap_or(Box::new(|_| {})),
                    proxy.as_deref(),
                ))
            }))
            .unwrap_or_else(|payload| {
                let reason = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown panic");
                Err(StreamGearsError::new_err(format!(
                    "download from {url} failed: {reason}"
                )))
            })
        })
    });

    let callback_error = callback_error
        .lock()
        .ok()
        .and_then(|mut first| first.take());
    match (result, callback_error) {
        (Ok(()), None) => Ok(()),
        (Ok(()), Some(callback_error)) => Err(callback_error),
        (Err(err), callback_error) => {
            if callback_error.is_some() {
                err.set_cause(py, callback_error);
            }
            Err(err)
        }
    }
}

#[pyfunction]
#[pyo3(signature = (file, proxy=None))]
fn login_by_cookies(py: Python<'_>, file: String, proxy: Option<String>) -> PyResult<bool> {
    py.detach(|| {
        new_runtime()?
            .block_on(login::login_by_cookies(&file, proxy.as_deref()))
            .map(|_| true)
            .map_err(stream_gears_err)
    })
}

#[pyfunction]
#[pyo3(signature = (country_code, phone, proxy=None))]
fn send_sms(
    py: Python<'_>,
    country_code: u32,
    phone: u64,
    proxy: Option<String>,
) -> PyResult<String> {
    py.detach(|| {
        new_runtime()?
            .block_on(login::send_sms(country_code, phone, proxy.as_deref()))
            .map(|res| res.to_string())
            .map_err(stream_gears_err)
    })
}

/// 成功时返回 `True`，并把登录信息写入 `file`（默认当前目录的 `cookies.json`）。
/// 登录失败抛 `StreamGearsError`（消息里带原因），`ret` 不是合法 JSON 时抛 `ValueError`。
#[pyfunction]
#[pyo3(signature = (code, ret, proxy=None, file=PathBuf::from("cookies.json")))]
#[pyo3(text_signature = "(code, ret, proxy=None, file='cookies.json')")]
fn login_by_sms(
    py: Python<'_>,
    code: u32,
    ret: String,
    proxy: Option<String>,
    file: PathBuf,
) -> PyResult<bool> {
    let ret = parse_json_arg("ret", &ret)?;
    py.detach(|| {
        new_runtime()?
            .block_on(login::login_by_sms(code, ret, proxy.as_deref(), &file))
            .map_err(stream_gears_err)
    })
}

#[pyfunction]
#[pyo3(signature = (proxy=None))]
fn get_qrcode(py: Python<'_>, proxy: Option<String>) -> PyResult<String> {
    py.detach(|| {
        new_runtime()?
            .block_on(login::get_qrcode(proxy.as_deref()))
            .map(|res| res.to_string())
            .map_err(stream_gears_err)
    })
}

/// 返回登录信息的 JSON 字符串；给了 `file` 时同时写入该文件。
#[pyfunction]
#[pyo3(signature = (ret, proxy=None, file=None))]
fn login_by_qrcode(
    py: Python<'_>,
    ret: String,
    proxy: Option<String>,
    file: Option<PathBuf>,
) -> PyResult<String> {
    let ret = parse_json_arg("ret", &ret)?;
    py.detach(|| {
        let rt = new_runtime()?;
        let info = rt
            .block_on(Credential::new(proxy.as_deref()).login_by_qrcode(ret))
            .map_err(stream_gears_err)?;
        if let Some(file) = &file {
            rt.block_on(save_login_info(file, &info))
                .map_err(stream_gears_err)?;
        }
        serde_json::to_string_pretty(&info).map_err(stream_gears_err)
    })
}

#[pyfunction]
#[pyo3(signature = (sess_data, bili_jct, proxy=None, file=PathBuf::from("cookies.json")))]
#[pyo3(text_signature = "(sess_data, bili_jct, proxy=None, file='cookies.json')")]
fn login_by_web_cookies(
    py: Python<'_>,
    sess_data: String,
    bili_jct: String,
    proxy: Option<String>,
    file: PathBuf,
) -> PyResult<bool> {
    py.detach(|| {
        new_runtime()?
            .block_on(login::login_by_web_cookies(
                &sess_data,
                &bili_jct,
                proxy.as_deref(),
                &file,
            ))
            .map_err(stream_gears_err)
    })
}

#[pyfunction]
#[pyo3(signature = (sess_data, dede_user_id, proxy=None, file=PathBuf::from("cookies.json")))]
#[pyo3(text_signature = "(sess_data, dede_user_id, proxy=None, file='cookies.json')")]
fn login_by_web_qrcode(
    py: Python<'_>,
    sess_data: String,
    dede_user_id: String,
    proxy: Option<String>,
    file: PathBuf,
) -> PyResult<bool> {
    py.detach(|| {
        new_runtime()?
            .block_on(login::login_by_web_qrcode(
                &sess_data,
                &dede_user_id,
                proxy.as_deref(),
                &file,
            ))
            .map_err(stream_gears_err)
    })
}

#[allow(clippy::too_many_arguments)]
#[pyfunction]
#[pyo3(signature = (video_path, cookie_file, title, tid=171, tid_v2=None, tag="".to_string(), copyright=2, source="".to_string(), desc="".to_string(), dynamic="".to_string(), cover="".to_string(), dolby=0, lossless_music=0, no_reprint=0, charging_pay=0, up_close_reply=false, up_selection_reply=false, up_close_danmu=false, limit=3, desc_v2=vec![], dtime=None, line=None, extra_fields="".to_string(), submit=None, proxy=None))]
fn upload(
    py: Python<'_>,
    video_path: Vec<PathBuf>,
    cookie_file: PathBuf,
    title: String,
    tid: u16,
    tid_v2: Option<u32>,
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
    let extra_fields = parse_extra_fields(extra_fields.as_deref()).map_err(|e| {
        PyValueError::new_err(format!("extra_fields is not a valid JSON object: {e}"))
    })?;
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
                .maybe_tid_v2(tid_v2)
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
                .extra_fields(extra_fields)
                .build();

            // let submit = match submit {
            //     Some(value) => SubmitOption::from_str(&value, true).unwrap(),
            //     None => SubmitOption::App,
            // };

            rt.block_on(uploader::upload(
                studio_pre,
                submit.as_deref(),
                proxy.as_deref(),
            ))
            .map(|_| ())
            .map_err(stream_gears_err)
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
        &[] => {
            args.push("biliup".to_string());
            args.push("server".to_string());
        }
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
    m.add("StreamGearsError", m.py().get_type::<StreamGearsError>())?;
    Ok(())
}

/// `None`、空串或只含空白视为没有额外字段；其余必须是 JSON 对象。
fn parse_extra_fields(s: Option<&str>) -> serde_json::Result<HashMap<String, serde_json::Value>> {
    match s.map(str::trim) {
        None | Some("") => Ok(HashMap::new()),
        Some(value) => serde_json::from_str(value),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_extra_fields;
    use serde_json::json;

    #[test]
    fn missing_or_blank_extra_fields_are_empty() {
        assert!(parse_extra_fields(None).unwrap().is_empty());
        assert!(parse_extra_fields(Some("")).unwrap().is_empty());
        assert!(parse_extra_fields(Some("  \n")).unwrap().is_empty());
    }

    #[test]
    fn extra_fields_json_object_is_parsed() {
        let fields = parse_extra_fields(Some(r#"{"a": 1, "b": {"c": "d"}}"#)).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields["a"], json!(1));
        assert_eq!(fields["b"], json!({"c": "d"}));
    }

    #[test]
    fn malformed_extra_fields_are_rejected() {
        for input in [r#"{"a":1,}"#, "{", "[1, 2]", "null", "1", r#""a""#] {
            assert!(parse_extra_fields(Some(input)).is_err(), "{input}");
        }
    }
}
