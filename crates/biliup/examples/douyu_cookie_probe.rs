//! Read-only login check:
//! cargo run -p biliup --example douyu_cookie_probe -- --cookie-file /path/to/cookies.json
//! Explicit renewal test: add --passport-file PATH --refresh --output NEW_PATH.
//! Never prints credentials, account IDs, raw responses, or signed URLs.
use biliup::downloader::live::{DouyuCookieInput, DouyuCookieRefresh, DouyuRefreshClient};
use serde_json::json;
use std::io::Write;

struct Options {
    cookie_file: String,
    passport_file: Option<String>,
    output: Option<String>,
    refresh: bool,
}

impl Options {
    fn parse() -> Result<Self, &'static str> {
        let mut args = std::env::args().skip(1);
        let mut cookie_file = None;
        let mut passport_file = None;
        let mut output = None;
        let mut refresh = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--cookie-file" => {
                    cookie_file = Some(args.next().ok_or("argument_requires_value")?)
                }
                "--passport-file" => {
                    passport_file = Some(args.next().ok_or("argument_requires_value")?)
                }
                "--output" => output = Some(args.next().ok_or("argument_requires_value")?),
                "--refresh" => refresh = true,
                _ => return Err("unknown_argument"),
            }
        }
        if output.is_some() && !refresh {
            return Err("output_requires_refresh");
        }
        Ok(Self {
            cookie_file: cookie_file.ok_or("cookie_file_required")?,
            passport_file,
            output,
            refresh,
        })
    }
}

fn read_cookie(path: &str) -> Result<DouyuCookieInput, String> {
    let input = std::fs::read_to_string(path).map_err(|_| "cookie_file_read_failed".to_owned())?;
    DouyuCookieInput::parse(&input).map_err(|error| error.to_string())
}

/// Browser-export shape, with passport-only secrets kept separate from Web scope.
/// A new file is required so an existing credential file cannot be overwritten.
fn save_output(path: &str, result: &DouyuCookieRefresh) -> Result<(), &'static str> {
    let mut cookies = Vec::new();
    for pair in result.cookie.split(';') {
        let (name, value) = pair.trim().split_once('=').ok_or("output_cookie_invalid")?;
        cookies.push(json!({"name":name,"value":value,"domain":"www.douyu.com","path":"/"}));
    }
    cookies
        .push(json!({"name":"LTP0","value":result.ltp0,"domain":"passport.douyu.com","path":"/"}));
    cookies.push(
        json!({"name":"dy_did","value":result.dy_did,"domain":"passport.douyu.com","path":"/"}),
    );
    let bytes = serde_json::to_vec_pretty(&cookies).map_err(|_| "output_serialization_failed")?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "new_output_file_creation_failed")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "output_file_write_failed")
}

async fn run(options: Options) -> Result<(), String> {
    if options
        .output
        .as_deref()
        .is_some_and(|path| std::path::Path::new(path).exists())
    {
        return Err("output_file_already_exists".into());
    }
    let web = read_cookie(&options.cookie_file)?;
    let client = DouyuRefreshClient::new().map_err(|error| error.to_string())?;
    if options.refresh {
        let passport = options
            .passport_file
            .as_deref()
            .map(read_cookie)
            .transpose()?
            .unwrap_or_else(|| web.clone());
        let ltp0 = passport.ltp0.as_deref().ok_or("passport_ltp0_missing")?;
        let did = passport
            .dy_did
            .as_deref()
            .ok_or("passport_dy_did_missing")?;
        let uid = web.account_id.as_deref().ok_or("web_account_id_missing")?;
        let result = client
            .refresh(&web.cookie, ltp0, did, Some(uid))
            .await
            .map_err(|error| error.to_string())?;
        if let Some(output) = &options.output {
            save_output(output, &result).map_err(str::to_owned)?;
        }
        println!(
            "{}",
            json!({
                "refreshed":true,"cookie_valid":true,"account_match":true,
                "has_ltp0":!result.ltp0.is_empty(),"has_dy_did":!result.dy_did.is_empty(),
                "output_saved":options.output.is_some(),
            })
        );
        return Ok(());
    }
    let identity = client
        .validate(&web.cookie, web.account_id.as_deref())
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        json!({
            "cookie_valid":identity.is_some(),"has_ltp0":web.ltp0.is_some(),
            "has_dy_did":web.dy_did.is_some(),
            "auto_refresh_ready":web.ltp0.is_some() && web.dy_did.is_some(),
        })
    );
    if identity.is_none() {
        return Err("cookie_session_is_invalid".into());
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let options = match Options::parse() {
        Ok(options) => options,
        Err(error) => {
            println!("{}", json!({"error":error}));
            std::process::exit(2);
        }
    };
    if let Err(error) = run(options).await {
        println!("{}", json!({"error":error}));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_output_is_private_and_cannot_overwrite_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cookies.json");
        let result = DouyuCookieRefresh {
            cookie: "acf_uid=42; acf_auth=fixture".into(),
            ltp0: "ticket".into(),
            dy_did: "device".into(),
            account_id: "42".into(),
            updated_fields: 2,
        };
        save_output(path.to_str().unwrap(), &result).unwrap();
        let imported = read_cookie(path.to_str().unwrap()).unwrap();
        assert_eq!(imported.ltp0.as_deref(), Some("ticket"));
        assert!(!imported.cookie.contains("LTP0"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            save_output(path.to_str().unwrap(), &result),
            Err("new_output_file_creation_failed")
        );
    }
}
