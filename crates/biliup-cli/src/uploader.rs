use crate::cli::UploadLine;
use biliup::client::StatelessClient;
use biliup::error::Kind;
use biliup::uploader::bilibili::{BiliBili, Studio, Vid, Video};
use biliup::uploader::credential::{Credential, LoginInfo};
use biliup::uploader::line::Probe;
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, credential, line, load_config};
use biliup_cli::server::errors::{AppError, AppResult};
use bytes::{Buf, Bytes};
use clap::ValueEnum;
use dialoguer::Input;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use error_stack::ResultExt;
use futures::{Stream, StreamExt};
use image::Luma;
use indicatif::{ProgressBar, ProgressStyle};
use qrcode::QrCode;
use qrcode::render::unicode;
use reqwest::Body;
use std::ffi::OsStr;
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::Poll;
use std::time::Instant;
use tracing::{info, warn};

pub async fn login(user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<()> {
    let client = Credential::new(proxy);
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("选择一种登录方式")
        .default(1)
        .item("账号密码")
        .item("短信登录")
        .item("扫码登录")
        .item("浏览器登录")
        .item("网页Cookie登录1")
        .item("网页Cookie登录2")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    let info = match selection {
        0 => login_by_password(client).await?,
        1 => login_by_sms(client).await?,
        2 => login_by_qrcode(client).await?,
        3 => login_by_browser(client).await?,
        4 => login_by_web_cookies(client).await?,
        5 => login_by_webqr_cookies(client).await?,
        _ => panic!(),
    };
    let file = std::fs::File::create(user_cookie).change_context_lazy(|| AppError::Unknown)?;
    serde_json::to_writer_pretty(&file, &info).change_context_lazy(|| AppError::Unknown)?;
    info!("登录成功，数据保存在{:?}", file);
    Ok(())
}

pub async fn renew(user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<()> {
    let client = Credential::new(proxy);
    let mut file = fopen_rw(user_cookie)?;
    let login_info: LoginInfo =
        serde_json::from_reader(&file).change_context_lazy(|| AppError::Unknown)?;
    let new_info = client
        .renew_tokens(login_info)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    file.rewind().change_context_lazy(|| AppError::Unknown)?;
    file.set_len(0).change_context_lazy(|| AppError::Unknown)?;
    serde_json::to_writer_pretty(std::io::BufWriter::new(&file), &new_info)
        .change_context_lazy(|| AppError::Unknown)?;
    info!("{new_info:?}");
    Ok(())
}

pub async fn upload_by_command(
    mut studio: Studio,
    user_cookie: PathBuf,
    video_path: Vec<PathBuf>,
    line: Option<UploadLine>,
    limit: usize,
    submit: SubmitOption,
    proxy: Option<&str>,
) -> AppResult<()> {
    let bili = login_by_cookies(user_cookie, proxy).await?;
    if studio.title.is_empty() {
        studio.title = video_path[0]
            .file_stem()
            .and_then(OsStr::to_str)
            .map(|s| s.to_string())
            .unwrap();
    }
    cover_up(&mut studio, &bili).await?;
    studio.videos = upload(&video_path, &bili, line, limit).await?;

    match submit {
        SubmitOption::BCutAndroid => bili
            .submit_by_bcut_android(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
        _ => bili
            .submit_by_app(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
    };

    Ok(())
}

pub async fn upload_by_config(
    config: PathBuf,
    user_cookie: PathBuf,
    submit_override: Option<SubmitOption>,
    proxy: Option<&str>,
) -> AppResult<()> {
    // println!("number of concurrent futures: {limit}");
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let config = load_config(&config).change_context_lazy(|| AppError::Unknown)?;
    for (filename_patterns, mut studio) in config.streamers {
        let mut paths = Vec::new();
        for entry in glob::glob(&filename_patterns)
            .change_context_lazy(|| AppError::Unknown)?
            .filter_map(Result::ok)
        {
            paths.push(entry);
        }
        if paths.is_empty() {
            warn!("未搜索到匹配的视频文件：{filename_patterns}");
            continue;
        }
        cover_up(&mut studio, &bilibili).await?;

        studio.videos = upload(
            &paths,
            &bilibili,
            config
                .line
                .as_ref()
                .and_then(|l| UploadLine::from_str(l, true).ok()),
            config.limit,
        )
        .await?;
        // 命令行参数优先，如果没有提供则使用配置文件中的设置
        let submit_option = submit_override.clone().unwrap_or(config.submit.clone());
        match submit_option {
            SubmitOption::BCutAndroid => bilibili
                .submit_by_bcut_android(&studio, proxy)
                .await
                .change_context_lazy(|| AppError::Unknown)?,
            _ => bilibili
                .submit_by_app(&studio, proxy)
                .await
                .change_context_lazy(|| AppError::Unknown)?,
        };
    }
    Ok(())
}

pub async fn append(
    user_cookie: PathBuf,
    vid: Vid,
    video_path: Vec<PathBuf>,
    line: Option<UploadLine>,
    limit: usize,
    submit: SubmitOption,
    proxy: Option<&str>,
) -> AppResult<()> {
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let mut uploaded_videos = upload(&video_path, &bilibili, line, limit).await?;
    let mut studio = bilibili
        .studio_data(&vid, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    studio.videos.append(&mut uploaded_videos);
    match submit {
        SubmitOption::App => bilibili
            .edit_by_app(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
        _ => bilibili
            .edit_by_web(&studio)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
    };
    // studio.edit(&login_info).await?;
    Ok(())
}

pub async fn show(user_cookie: PathBuf, vid: Vid, proxy: Option<&str>) -> AppResult<()> {
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let video_info = bilibili
        .video_data(&vid, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&video_info).change_context_lazy(|| AppError::Unknown)?
    );
    Ok(())
}

pub async fn list(
    user_cookie: PathBuf,
    is_pubing: bool,
    pubed: bool,
    not_pubed: bool,
    proxy: Option<&str>,
    from_page: u32,
    max_pages: Option<u32>,
) -> AppResult<()> {
    let status = match (is_pubing, pubed, not_pubed) {
        (true, false, false) => "is_pubing",
        (false, true, false) => "pubed",
        (false, false, true) => "not_pubed",
        (false, false, false) => "is_pubing,pubed,not_pubed",
        _ => {
            tracing::warn!("选项互斥，默认列出所有状态的稿件");
            "is_pubing,pubed,not_pubed"
        }
    };

    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    bilibili
        .recent_archives(status, from_page, max_pages)
        .await
        .change_context_lazy(|| AppError::Unknown)?
        .iter()
        .for_each(|arc| println!("{}", arc.to_string_pretty()));
    Ok(())
}

async fn login_by_cookies(user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<BiliBili> {
    let result = credential::login_by_cookies(&user_cookie, proxy).await;
    Ok(match result {
        Err(Kind::IO(_)) => result.change_context_lazy(|| {
            AppError::Custom(String::from("open cookies file: ") + &user_cookie.to_string_lossy())
        })?,
        _ => {
            let bili = result.change_context_lazy(|| AppError::Unknown)?;
            info!(
                "user: {}",
                bili.my_info()
                    .await
                    .change_context_lazy(|| AppError::Unknown)?["data"]["name"]
                    .as_str()
                    .unwrap_or_default()
            );
            bili
        }
    })
}

pub async fn cover_up(studio: &mut Studio, bili: &BiliBili) -> AppResult<()> {
    if !studio.cover.is_empty() {
        let url = bili
            .cover_up(
                &std::fs::read(Path::new(&studio.cover))
                    .change_context_lazy(|| AppError::Custom(format!("cover: {}", studio.cover)))?,
            )
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        info!("{url}");
        studio.cover = url;
    }
    Ok(())
}

pub async fn upload(
    video_path: &[PathBuf],
    bili: &BiliBili,
    line: Option<UploadLine>,
    limit: usize,
) -> AppResult<Vec<Video>> {
    info!("number of concurrent futures: {limit}");
    let mut videos = Vec::new();
    let client = StatelessClient::default();
    let line = match line {
        Some(UploadLine::Bldsa) => line::bldsa(),
        Some(UploadLine::Cnbldsa) => line::cnbldsa(),
        Some(UploadLine::Andsa) => line::andsa(),
        Some(UploadLine::Atdsa) => line::atdsa(),
        Some(UploadLine::Bda2) => line::bda2(),
        Some(UploadLine::Cnbd) => line::cnbd(),
        Some(UploadLine::Anbd) => line::anbd(),
        Some(UploadLine::Atbd) => line::atbd(),
        Some(UploadLine::Tx) => line::tx(),
        Some(UploadLine::Cntx) => line::cntx(),
        Some(UploadLine::Antx) => line::antx(),
        Some(UploadLine::Attx) => line::attx(),
        // Some(UploadLine::Bda) => line::bda(),
        Some(UploadLine::Txa) => line::txa(),
        Some(UploadLine::Alia) => line::alia(),
        _ => Probe::probe(&client.client).await.unwrap_or_default(),
    };
    // let line = line::kodo();
    for video_path in video_path {
        info!("{line:?}");
        let video_file = VideoFile::new(video_path).change_context_lazy(|| {
            AppError::Custom(format!("file {}", video_path.to_string_lossy()))
        })?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();
        let uploader = line
            .pre_upload(bili, video_file)
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        //Progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").change_context_lazy(|| AppError::Unknown)?);
        // pb.enable_steady_tick(Duration::from_secs(1));
        // pb.tick()

        let instant = Instant::now();

        let video = uploader
            .upload(client.clone(), limit, |vs| {
                vs.map(|chunk| {
                    let pb = pb.clone();
                    let chunk = chunk?;
                    let len = chunk.len();
                    Ok((Progressbar::new(chunk, pb), len))
                })
            })
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        pb.finish_and_clear();
        let t = instant.elapsed().as_millis();
        info!(
            "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
            t as f64 / 1000.,
            total_size as f64 / 1000. / t as f64
        );
        videos.push(video);
    }
    Ok(videos)
}

pub async fn login_by_password(credential: Credential) -> AppResult<LoginInfo> {
    let username: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入账号")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    let password: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入密码")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_password(&username, &password)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_sms(credential: Credential) -> AppResult<LoginInfo> {
    let country_code: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入手机国家代码")
        .default(86)
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let phone: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入手机号")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let res = credential
        .send_sms_handle_recaptcha(phone, country_code, |url| async move {
            println!("{url}");
            println!("请复制此链接至浏览器打开并启动开发者工具，完成滑动验证后查看网络请求");

            let challenge: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("请输入get.php响应中的challenge值")
                .interact_text()
                .map_err(|e| e.to_string())?;

            let valiate: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("请输入ajax.php响应中的validate值")
                .interact_text()
                .map_err(|e| e.to_string())?;

            Ok((challenge, valiate))
        })
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    let input: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入验证码")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    // println!("{}", payload);
    credential
        .login_by_sms(input, res)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_qrcode(credential: Credential) -> AppResult<LoginInfo> {
    let value = credential
        .get_qrcode()
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    let code = QrCode::new(
        value["data"]["url"]
            .as_str()
            .unwrap()
            .replace("https", "http"),
    )
    .unwrap();
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    println!("{}", image);
    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();
    println!(
        "在Windows下建议使用Windows Terminal(支持utf8，可完整显示二维码)\n否则可能无法正常显示，此时请打开./qrcode.png扫码"
    );
    // Save the image.
    image.save("qrcode.png").unwrap();
    credential
        .login_by_qrcode(value)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_browser(credential: Credential) -> AppResult<LoginInfo> {
    let value = credential
        .get_qrcode()
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    println!(
        "{}",
        value["data"]["url"]
            .as_str()
            .ok_or_else(|| AppError::Custom(value.to_string()))?
    );
    println!("请复制此链接至浏览器中完成登录");
    credential
        .login_by_qrcode(value)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_web_cookies(credential: Credential) -> AppResult<LoginInfo> {
    let sess_data: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入SESSDATA")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let bili_jct: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入bili_jct")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_web_cookies(&sess_data, &bili_jct)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_webqr_cookies(credential: Credential) -> AppResult<LoginInfo> {
    let sess_data: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入SESSDATA")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let dede_user_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入DedeUserID")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_web_qrcode(&sess_data, &dede_user_id)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

impl From<Progressbar> for Body {
    fn from(async_stream: Progressbar) -> Self {
        Body::wrap_stream(async_stream)
    }
}

#[inline]
pub fn fopen_rw<P: AsRef<Path>>(path: P) -> AppResult<std::fs::File> {
    let path = path.as_ref();
    std::fs::File::options()
        .read(true)
        .write(true)
        .open(path)
        .change_context_lazy(|| {
            AppError::Custom(String::from("open cookies file: ") + &path.to_string_lossy())
        })
}

#[derive(Clone)]
struct Progressbar {
    bytes: Bytes,
    pb: ProgressBar,
}

impl Progressbar {
    pub fn new(bytes: Bytes, pb: ProgressBar) -> Self {
        Self { bytes, pb }
    }

    pub fn progress(&mut self) -> AppResult<Option<Bytes>> {
        let pb = &self.pb;

        let content_bytes = &mut self.bytes;

        let n = content_bytes.remaining();

        let pc = 4096;
        if n == 0 {
            Ok(None)
        } else if n < pc {
            pb.inc(n as u64);
            Ok(Some(content_bytes.copy_to_bytes(n)))
        } else {
            pb.inc(pc as u64);

            Ok(Some(content_bytes.copy_to_bytes(pc)))
        }
    }
}

impl Stream for Progressbar {
    type Item = AppResult<Bytes>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.progress()? {
            None => Poll::Ready(None),
            Some(s) => Poll::Ready(Some(Ok(s))),
        }
    }
}
