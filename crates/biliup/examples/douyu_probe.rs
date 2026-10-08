//! Live smoke check: cargo run -p biliup --example douyu_probe -- --room 288016
//! Add --cookie-file /path/to/cookies.json to check an authenticated session.
//! Output is safe for public CI logs; API messages, URLs and credentials are omitted.
use biliup::downloader::live::{Douyu, LiveCredentials, LiveOptions, LivePlugin, LiveRequest, LiveStatus};
use futures::StreamExt;
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, process::Command, time::Duration};

const MAX_BYTES: usize = 8 * 1024 * 1024;

struct Options {
    room: String,
    codec: String,
    rate: u32,
    cdn: String,
    cookie: Option<String>,
    seconds: u64,
    output_dir: PathBuf,
}

impl Options {
    fn parse() -> Result<Self, &'static str> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.len() % 2 != 0 {
            return Err("arguments_require_values");
        }
        let mut values = HashMap::new();
        for pair in args.chunks_exact(2) {
            if !matches!(
                pair[0].as_str(),
                "--room" | "--codec" | "--rate" | "--cdn" | "--cookie-file" | "--seconds" | "--output-dir"
            ) {
                return Err("unknown_argument");
            }
            values.insert(pair[0].as_str(), pair[1].as_str());
        }
        let room = values.get("--room").copied().unwrap_or("288016");
        if room.is_empty()
            || !room
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err("room_requires_safe_id_or_alias");
        }
        let codec = values.get("--codec").copied().unwrap_or("AVC").to_ascii_uppercase();
        if !matches!(codec.as_str(), "AVC" | "HEVC") {
            return Err("codec_requires_AVC_or_HEVC");
        }
        let rate = values
            .get("--rate")
            .copied()
            .unwrap_or("0")
            .parse()
            .map_err(|_| "rate_requires_unsigned_integer")?;
        let seconds = values
            .get("--seconds")
            .copied()
            .unwrap_or("5")
            .parse::<u64>()
            .map_err(|_| "seconds_requires_unsigned_integer")?
            .clamp(1, 15);
        let cookie = values
            .get("--cookie-file")
            .map(|path| std::fs::read_to_string(path).map_err(|_| "cookie_file_read_failed"))
            .transpose()?;
        Ok(Self {
            room: room.to_owned(),
            codec,
            rate,
            cdn: values.get("--cdn").copied().unwrap_or("hw-h5").to_owned(),
            cookie,
            seconds,
            output_dir: values
                .get("--output-dir")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::temp_dir().join("biliup-douyu-probe")),
        })
    }
}

/// A bounded capture may end halfway through a tag; retain only complete tags.
fn trim_flv(data: &mut Vec<u8>) -> Result<(), &'static str> {
    if data.len() < 13 || &data[..3] != b"FLV" {
        return Err("media_response_is_not_FLV");
    }
    let mut complete = u32::from_be_bytes(data[5..9].try_into().unwrap()) as usize + 4;
    if !(13..=1028).contains(&complete) || complete > data.len() {
        return Err("invalid_FLV_header");
    }
    while complete + 11 <= data.len() {
        let size = (usize::from(data[complete + 1]) << 16)
            | (usize::from(data[complete + 2]) << 8)
            | usize::from(data[complete + 3]);
        let end = complete + 11 + size + 4;
        if end > data.len() {
            break;
        }
        complete = end;
    }
    data.truncate(complete);
    Ok(())
}

async fn run(options: Options) -> Result<(), &'static str> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|_| "http_client_creation_failed")?;
    if let Some(cookie) = &options.cookie {
        let valid = Douyu::validate_cookie(cookie, &client)
            .await
            .map_err(|_| "cookie_validation_request_failed")?;
        println!("{}", json!({"room": options.room, "cookie_valid": valid}));
        if !valid {
            return Err("cookie_session_is_invalid");
        }
    }
    let mut live_options = LiveOptions::default();
    live_options.douyu.codec = options.codec.clone();
    live_options.douyu.rate = options.rate;
    live_options.douyu.cdn = options.cdn;
    let request = LiveRequest {
        client: client.clone(),
        url: format!("https://www.douyu.com/{}", options.room),
        name: "douyu-probe".to_owned(),
        options: live_options,
        credentials: LiveCredentials {
            douyu_cookie: options.cookie.clone(),
            ..Default::default()
        },
    };
    let stream = match Douyu::new()
        .check_stream(request)
        .await
        .map_err(|_| "douyu_stream_check_failed")?
    {
        LiveStatus::Live { stream } => stream,
        LiveStatus::Offline => {
            println!(
                "{}",
                json!({"room": options.room, "requested_codec": options.codec,
                "requested_rate": options.rate, "offline": true})
            );
            return Ok(());
        }
    };
    let mut request = client.get(&stream.raw_stream_url);
    for (name, value) in &stream.stream_headers {
        request = request.header(name, value);
    }
    let response = request.send().await.map_err(|_| "media_request_failed")?;
    if !response.status().is_success() {
        return Err("media_HTTP_status_is_not_success");
    }
    let mut chunks = response.bytes_stream();
    let mut data = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.seconds);
    while data.len() < MAX_BYTES {
        match tokio::time::timeout_at(deadline, chunks.next()).await {
            Ok(Some(Ok(bytes))) => {
                data.extend_from_slice(&bytes[..bytes.len().min(MAX_BYTES - data.len())]);
            }
            Ok(Some(Err(_))) => {
                break;
            }
            _ => break,
        }
    }
    trim_flv(&mut data)?;
    std::fs::create_dir_all(&options.output_dir).map_err(|_| "output_directory_creation_failed")?;
    let path = options.output_dir.join(format!(
        "{}-{}-{}-{}.flv",
        options.room,
        if options.cookie.is_some() {
            "authenticated"
        } else {
            "anonymous"
        },
        options.codec,
        options.rate
    ));
    std::fs::write(&path, &data).map_err(|_| "sample_file_write_failed")?;
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name,width,height,r_frame_rate",
            "-of",
            "json",
        ])
        .arg(&path)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "ffprobe_not_found_install_FFmpeg"
            } else {
                "ffprobe_launch_failed"
            }
        })?;
    if !probe.status.success() {
        return Err("ffprobe_failed");
    }
    let metadata: Value = serde_json::from_slice(&probe.stdout).map_err(|_| "ffprobe_invalid_JSON")?;
    let video = metadata
        .get("streams")
        .and_then(Value::as_array)
        .and_then(|streams| streams.iter().find(|stream| stream["codec_type"] == "video"))
        .ok_or("sample_has_no_video_stream")?;
    let decode = Command::new("ffmpeg")
        .args(["-v", "error", "-xerror", "-i"])
        .arg(&path)
        .args(["-t", "2", "-f", "null", "-"])
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "ffmpeg_not_found_install_FFmpeg"
            } else {
                "ffmpeg_launch_failed"
            }
        })?;
    let decoded = decode.status.success() && decode.stderr.is_empty();
    println!(
        "{}",
        json!({"room": options.room, "requested_codec": options.codec,
        "requested_rate": options.rate, "offline": false, "bytes": data.len(),
        "codec_name": video["codec_name"], "width": video["width"], "height": video["height"],
        "fps": video["r_frame_rate"], "decode_ok": decoded})
    );
    if !decoded {
        return Err("ffmpeg_video_decode_failed");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let result = match Options::parse() {
        Ok(options) => run(options).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", json!({"error": error}));
            std::process::ExitCode::FAILURE
        }
    }
}
