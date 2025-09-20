use crate::client::StatefulClient;
use futures::Future;
use reqwest::header;
use std::io::Seek;
use std::path::Path;

use crate::error::{Kind, Result};
use crate::uploader::bilibili::{BiliBili, ResponseData};
use base64::{Engine as _, engine::general_purpose};
use cookie::Cookie;
use md5::{Digest, Md5};
use reqwest::header::{COOKIE, ORIGIN, REFERER, USER_AGENT};

use rsa::{Pkcs1v15Encrypt, RsaPublicKey, pkcs8::DecodePublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;
use url::Url;

// const APP_KEY: &str = "ae57252b0c09105d";
// const APPSEC: &str = "c75875c596a69eb55bd119e74b07cfe3";
// const APP_KEY: &str = "783bbb7264451d82";
// const APPSEC: &str = "2653583c8873dea268ab9386918b1d65";
// const APP_KEY: &str = "4409e2ce8ffd12b8";
// const APPSEC: &str = "59b43e04ad6965f34319062b478f83dd";
// const APP_KEY: &str = "37207f2beaebf8d7";
// const APPSEC: &str = "e988e794d4d4b6dd43bc0e89d6e90c43";
// const APP_KEY: &str = "bca7e84c2d947ac6";
// const APPSEC: &str = "60698ba2f68e01ce44738920a0ffe768";
// const APP_KEY: &str = "bb3101000e232e27";
// const APPSEC: &str = "36efcfed79309338ced0380abd824ac1";
pub(crate) enum AppKeyStore {
    BiliTV,
    Android,
    BCutAndroid,
}

impl AppKeyStore {
    pub fn app_key(&self) -> &'static str {
        match self {
            AppKeyStore::BiliTV => "4409e2ce8ffd12b8",
            AppKeyStore::Android => "783bbb7264451d82",
            AppKeyStore::BCutAndroid => "5dce947fe22167f9",
        }
    }

    pub fn appsec(&self) -> &'static str {
        match self {
            AppKeyStore::BiliTV => "59b43e04ad6965f34319062b478f83dd",
            AppKeyStore::Android => "2653583c8873dea268ab9386918b1d65",
            AppKeyStore::BCutAndroid => "5491a31c6bc11fb764a9b1f8d4acf092",
        }
    }
}

pub fn bilibili_from_cookies(file: impl AsRef<Path>, proxy: Option<&str>) -> Result<BiliBili> {
    let file = std::fs::File::options().read(true).open(file)?;
    let login_info: LoginInfo = serde_json::from_reader(std::io::BufReader::new(&file))?;
    bilibili_from_info(login_info, proxy)
}

pub fn bilibili_from_info(login_info: LoginInfo, proxy: Option<&str>) -> Result<BiliBili> {
    let client = Credential::new(proxy);
    client.set_cookie(&login_info.cookie_info);
    info!("通过cookie登录");
    Ok(BiliBili {
        client: client.0.client,
        login_info,
    })
}

pub async fn login_by_cookies(file: impl AsRef<Path>, proxy: Option<&str>) -> Result<BiliBili> {
    // let path = file.as_ref();
    let mut file = std::fs::File::options().read(true).write(true).open(file)?;
    let login_info: LoginInfo = serde_json::from_reader(std::io::BufReader::new(&file))?;

    let client: Credential = Credential::new(proxy);
    let need_refresh = client.validate_tokens(&login_info).await?;

    if need_refresh {
        let new_info = client.renew_tokens(login_info).await?;
        file.rewind()?;
        file.set_len(0)?;
        serde_json::to_writer_pretty(std::io::BufWriter::new(&file), &new_info)?;
        bilibili_from_info(new_info, proxy)
    } else {
        info!("无需更新cookie");
        bilibili_from_info(login_info, proxy)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResponseValue {
    Login(LoginInfo),
    OAuth(OAuthInfo),
    Value(serde_json::Value),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LoginInfo {
    pub cookie_info: serde_json::Value,
    // message: String,
    pub sso: Vec<String>,
    // status: u8,
    pub token_info: TokenInfo,
    // url: String,
    pub platform: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TokenInfo {
    pub access_token: String,
    expires_in: u32,
    pub mid: u64,
    refresh_token: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OAuthInfo {
    pub mid: u64,
    pub access_token: String,
    pub expires_in: u32,
    pub refresh: bool,
}

#[derive(Debug)]
pub struct Credential(StatefulClient);

impl Credential {
    pub fn new(proxy: Option<&str>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Referer",
            header::HeaderValue::from_static("https://www.bilibili.com/"),
        );
        Self(StatefulClient::new(headers, proxy))
    }

    pub async fn validate_tokens(&self, login_info: &LoginInfo) -> Result<bool> {
        let payload = {
            let mut payload = json!({
                "access_key": login_info.token_info.access_token,
                "actionKey": "appkey",
                "appkey": AppKeyStore::Android.app_key(),
                "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            });

            let urlencoded = serde_urlencoded::to_string(&payload)?;
            let sign = Self::sign(&urlencoded, AppKeyStore::Android.appsec());
            payload["sign"] = Value::from(sign);
            payload
        };

        let response = self
            .0
            .client
            .get("https://passport.bilibili.com/x/passport-login/oauth2/info")
            .query(&payload)
            .send()
            .await?
            .json()
            .await?;
        // if response.code != 0 {
        //     return Err(CustomError::Custom(response.to_string()));
        // }

        let refresh = match response {
            ResponseData {
                data: Some(ResponseValue::OAuth(OAuthInfo { refresh, .. })),
                ..
            } => refresh,
            _ => return Err(Kind::Custom(response.to_string())),
        };

        info!("验证cookie");
        Ok(refresh)
    }

    pub async fn renew_tokens(&self, login_info: LoginInfo) -> Result<LoginInfo> {
        let keypair = match login_info.platform.as_deref() {
            Some("BiliTV") => AppKeyStore::BiliTV,
            Some("Android") => AppKeyStore::Android,
            Some(_) => return Err("未知平台".into()),
            None => return Ok(login_info),
        };
        let payload = {
            let mut payload = json!({
                "access_key": login_info.token_info.access_token,
                "actionKey": "appkey",
                "appkey": keypair.app_key(),
                "refresh_token": login_info.token_info.refresh_token,
                "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            });

            let urlencoded = serde_urlencoded::to_string(&payload)?;
            let sign = Self::sign(&urlencoded, keypair.appsec());
            payload["sign"] = Value::from(sign);
            payload
        };
        let response: ResponseData<ResponseValue> = self
            .0
            .client
            .post("https://passport.bilibili.com/x/passport-login/oauth2/refresh_token")
            .form(&payload)
            .send()
            .await?
            .json()
            .await?;
        info!("更新cookie");
        match response.data {
            Some(ResponseValue::Login(info)) if !info.cookie_info.is_null() => {
                self.set_cookie(&info.cookie_info);
                Ok(LoginInfo {
                    platform: login_info.platform,
                    ..info
                })
            }
            _ => Err(Kind::Custom(response.to_string())),
        }
    }

    pub async fn login_by_password(&self, username: &str, password: &str) -> Result<LoginInfo> {
        // The type of `payload` is `serde_json::Value`
        let mut rng = rand::thread_rng();
        let (key_hash, pub_key) = self.get_key().await?;
        let pub_key = RsaPublicKey::from_public_key_pem(&pub_key).unwrap();
        let enc_data = pub_key
            .encrypt(&mut rng, Pkcs1v15Encrypt, (key_hash + password).as_bytes())
            .expect("failed to encrypt");
        let encrypt_password = general_purpose::STANDARD_NO_PAD.encode(enc_data);
        let mut payload = json!({
            "actionKey": "appkey",
            "appkey": AppKeyStore::Android.app_key(),
            "build": 6270200,
            "captcha": "",
            "challenge": "",
            "channel": "bili",
            "device": "phone",
            "mobi_app": "android",
            "password": encrypt_password,
            "permission": "ALL",
            "platform": "android",
            "seccode": "",
            "subid": 1,
            "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "username": username,
            "validate": "",
        });
        let urlencoded = serde_urlencoded::to_string(&payload)?;
        let sign = Self::sign(&urlencoded, AppKeyStore::Android.appsec());
        payload["sign"] = Value::from(sign);
        let response: ResponseData<ResponseValue> = self
            .0
            .client
            .post("https://passport.bilibili.com/x/passport-login/oauth2/login")
            .form(&payload)
            .send()
            .await?
            .json()
            .await?;
        info!("通过密码登录");
        match response.data {
            Some(ResponseValue::Login(info)) if !info.cookie_info.is_null() => {
                self.set_cookie(&info.cookie_info);
                Ok(LoginInfo {
                    platform: Some("Android".to_string()),
                    ..info
                })
            }
            _ => Err(Kind::Custom(response.to_string())),
        }
    }

    pub async fn login_by_sms(
        &self,
        code: u32,
        mut payload: serde_json::Value,
    ) -> Result<LoginInfo> {
        payload["code"] = Value::from(code);
        let urlencoded = serde_urlencoded::to_string(&payload)?;
        let sign = Self::sign(&urlencoded, AppKeyStore::Android.appsec());
        payload["sign"] = Value::from(sign);
        let res: ResponseData<ResponseValue> = self
            .0
            .client
            .post("https://passport.bilibili.com/x/passport-login/login/sms")
            .form(&payload)
            .send()
            .await?
            .json()
            .await?;
        match res.data {
            Some(ResponseValue::Login(info)) => {
                self.set_cookie(&info.cookie_info);
                Ok(LoginInfo {
                    platform: Some("Android".to_string()),
                    ..info
                })
            }
            _ => Err(Kind::Custom(res.to_string())),
        }
    }

    pub async fn send_sms(
        &self,
        phone_number: u64,
        country_code: u32,
    ) -> Result<serde_json::Value> {
        self.send_sms_with_recaptcha(phone_number, country_code, None, None, None)
            .await
    }

    pub async fn send_sms_handle_recaptcha<F, Fut>(
        &self,
        phone_number: u64,
        country_code: u32,
        recaptcha_handler: F,
    ) -> Result<serde_json::Value>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = Result<(String, String)>>,
    {
        let url_string = match self
            .send_sms_with_recaptcha(phone_number, country_code, None, None, None)
            .await
        {
            Ok(res) => return Ok(res),
            Err(Kind::NeedRecaptcha(url)) => url,
            Err(e) => return Err(e),
        };

        let recaptcha = {
            Url::parse(&url_string)
                .map_err(|_| Kind::from("url parse error"))?
                .query_pairs()
                .find(|(k, _)| k == "recaptcha_token")
                .map(|(_, v)| v.to_string())
                .ok_or(Kind::from("cannot find recaptcha_token"))
        }?;

        info!("需要滑动验证码");
        let (challenge, validate) = recaptcha_handler(url_string).await?;

        self.send_sms_with_recaptcha(
            phone_number,
            country_code,
            Some(challenge.as_str()),
            Some(validate.as_str()),
            Some(recaptcha.as_ref()),
        )
        .await
    }

    pub async fn send_sms_with_recaptcha(
        &self,
        phone_number: u64,
        country_code: u32,
        challenge: Option<&str>,
        validate: Option<&str>,
        recaptcha: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut payload = json!({
            "actionKey": "appkey",
            "appkey": AppKeyStore::Android.app_key(),
            "build": 6510400,
            "buvid": &self.0.buvid,
            "channel": "bili",
            "cid": country_code,
            "device": "phone",
            "mobi_app": "android",
            "platform": "android",
            // "platform": "pc",
            "tel": phone_number,
            "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });

        if let (Some(c), Some(v), Some(r)) = (challenge, validate, recaptcha) {
            payload["gee_challenge"] = Value::from(c);
            payload["gee_seccode"] = Value::from(format!("{v}|jordan"));
            payload["gee_validate"] = Value::from(v);
            payload["recaptcha_token"] = Value::from(r);
        }

        let urlencoded = serde_urlencoded::to_string(&payload)?;
        let sign = Self::sign(&urlencoded, AppKeyStore::Android.appsec());
        let urlencoded = format!("{}&sign={}", urlencoded, sign);
        // let mut form = payload.clone();
        // form["sign"] = Value::from(sign);
        let res: ResponseData<ResponseValue> = self
            .0
            .client
            .post("https://passport.bilibili.com/x/passport-login/sms/send")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(urlencoded)
            .send()
            .await?
            .json()
            .await?;
        // println!("{}", res);
        match res.data {
            Some(ResponseValue::Value(mut data))
                if !data["captcha_key"]
                    .as_str()
                    .ok_or("send sms error")?
                    .is_empty() =>
            {
                payload["captcha_key"] = data["captcha_key"].take();
                Ok(payload)
            }
            Some(ResponseValue::Value(data))
                if !data["recaptcha_url"].as_str().unwrap_or("").is_empty() =>
            {
                let url = data["recaptcha_url"].as_str().unwrap().to_string();
                Err(Kind::NeedRecaptcha(url))
            }
            _ => Err(Kind::Custom(res.to_string())),
        }
    }

    pub async fn login_by_qrcode(&self, value: Value) -> Result<LoginInfo> {
        let mut form = json!({
            "appkey": AppKeyStore::BiliTV.app_key(),
            "auth_code": value["data"]["auth_code"],
            "local_id": "0",
            "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        });
        let urlencoded = serde_urlencoded::to_string(&form)?;
        let sign = Self::sign(&urlencoded, AppKeyStore::BiliTV.appsec());
        form["sign"] = Value::from(sign);
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let raw = self
                .0
                .client
                .post("https://passport.bilibili.com/x/passport-tv-login/qrcode/poll")
                .form(&form)
                .send()
                .await?
                .error_for_status()?;
            let full = raw.bytes().await?;

            let res: ResponseData<ResponseValue> = serde_json::from_slice(&full).map_err(|_| {
                Kind::Custom(format!(
                    "error decoding response body, content: {:#?}",
                    String::from_utf8_lossy(&full)
                ))
            })?;
            match res {
                ResponseData {
                    code: 0,
                    data: Some(ResponseValue::Login(info)),
                    ..
                } => {
                    self.set_cookie(&info.cookie_info);
                    break Ok(LoginInfo {
                        platform: Some("BiliTV".to_string()),
                        ..info
                    });
                }
                ResponseData { code: 86039, .. } => {
                    // 二维码尚未确认;
                    // form["ts"] = Value::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                }
                _ => {
                    break Err(Kind::Custom(format!("{res:#?}")));
                }
            }
        }
    }

    /// 获取 Web 端 buvid3 和 buvid4
    pub async fn get_web_buvid(&self) -> Result<(String, String)> {
        let res: ResponseData<Value> = self
            .0
            .client
            .get("https://api.bilibili.com/x/frontend/finger/spi")
            .send()
            .await?
            .json()
            .await?;
        match res.data {
            Some(value) => {
                let buvid3 = value["b_3"].as_str().ok_or("cannot find b_3")?.to_owned();
                let buvid4 = value["b_4"].as_str().ok_or("cannot find b_4")?.to_owned();
                Ok((buvid3, buvid4))
            }
            None => Err(Kind::Custom(format!("cannot find buvid: {:#?}", res))),
        }
    }

    pub async fn get_qrcode(&self) -> Result<Value> {
        let mut form = json!({
            "appkey": AppKeyStore::BiliTV.app_key(),
            "local_id": "0",
            "ts": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        });
        let urlencoded = serde_urlencoded::to_string(&form)?;
        let sign = Self::sign(&urlencoded, AppKeyStore::BiliTV.appsec());
        form["sign"] = Value::from(sign);
        Ok(self
            .0
            .client
            .post("https://passport.bilibili.com/x/passport-tv-login/qrcode/auth_code")
            .form(&form)
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_key(&self) -> Result<(String, String)> {
        let payload = json!({
            "appkey": AppKeyStore::Android.app_key(),
            "sign": Credential::sign(&format!("appkey={}", AppKeyStore::Android.app_key()), AppKeyStore::Android.appsec()),
        });
        let response: Value = self
            .0
            .client
            .get("https://passport.bilibili.com/x/passport-login/web/key")
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;
        let response = response
            .get("data")
            .ok_or_else(|| Kind::Custom(response.to_string()))?;
        let hash = response
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| Kind::Custom(response.to_string()))?;
        let key = response
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| Kind::Custom(response.to_string()))?;
        Ok((hash.to_string(), key.to_string()))
    }

    pub async fn login_by_web_qrcode(
        &self,
        sess_data: &str,
        dede_user_id: &str,
    ) -> Result<LoginInfo> {
        info!("login_by_web_qrcode");
        let qrcode: Value = self.0.client
            .get("https://passport.bilibili.com/qrcode/getLoginUrl")
            .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1 BiliApp")
            .send()
            .await?
            .json()
            .await?;
        let oauth_key = qrcode["data"]["oauthKey"].as_str();
        let cookies = format!("SESSDATA={sess_data}; DedeUserID={dede_user_id}");
        self.0.client
            .post("https://passport.bilibili.com/qrcode/login/confirm")
            .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1 BiliApp")
            .header(COOKIE, cookies)
            .header(REFERER, "https://passport.bilibili.com/mobile/h5-confirm.html")
            .header(ORIGIN, "https://passport.bilibili.com")
            .form(&[("oauthKey", oauth_key)])
            .send()
            .await?.error_for_status()?;
        self.0.client
            .post("https://passport.bilibili.com/qrcode/getLoginInfo")
            .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1 BiliApp")
            .form(&[("oauthKey", oauth_key)])
            .send()
            .await?.error_for_status()?;
        self.login_by_web_cookies(&self.get_cookie("SESSDATA"), &self.get_cookie("bili_jct"))
            .await
    }

    pub async fn login_by_web_cookies(&self, sess_data: &str, bili_jct: &str) -> Result<LoginInfo> {
        info!("获取二维码");
        let qrcode = self.get_qrcode().await?;
        let auth_code = qrcode["data"]["auth_code"]
            .as_str()
            .ok_or("Cannot get auth_code")?;
        self.web_confirm_qrcode(auth_code, sess_data, bili_jct)
            .await?;
        self.login_by_qrcode(qrcode).await
    }

    async fn web_confirm_qrcode(
        &self,
        auth_code: &str,
        sess_data: &str,
        bili_jct: &str,
    ) -> Result<()> {
        let form = json!({
            "auth_code": auth_code,
            "csrf": bili_jct,
            "scanning_type": 3,
        });
        let cookies = format!("SESSDATA={}; bili_jct={}", sess_data, bili_jct);
        info!("自动确认二维码");
        let response = self.0.client
            .post("https://passport.bilibili.com/x/passport-tv-login/h5/qrcode/confirm")
            .header("Cookie", cookies)
            // .header("native_api_from", "h5")
            .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1 BiliApp")
            .form(&form)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Kind::Custom(response.text().await?));
        }
        let res: ResponseData = response.json().await?;
        if res.code != 0 {
            return Err(Kind::Custom(format!("{res:#?}")));
        }
        Ok(())
    }

    pub fn sign(param: &str, app_sec: &str) -> String {
        let mut hasher = Md5::new();
        // process input message
        hasher.update(format!("{}{}", param, app_sec));
        // acquire hash digest in the form of GenericArray,
        // which in this case is equivalent to [u8; 16]
        format!("{:x}", hasher.finalize())
    }

    fn set_cookie(&self, cookie_info: &serde_json::Value) {
        let mut store = self.0.cookie_store.lock().unwrap();
        for cookie in cookie_info["cookies"].as_array().unwrap() {
            let cookie = Cookie::build((
                cookie["name"].as_str().unwrap(),
                cookie["value"].as_str().unwrap(),
            ))
            .domain("bilibili.com")
            .into();

            store
                .insert_raw(&cookie, &Url::parse("https://bilibili.com/").unwrap())
                .unwrap();
        }
    }

    fn get_cookie(&self, name: &str) -> String {
        let store = self.0.cookie_store.lock().unwrap();
        for item in store.iter_any() {
            if item.name() == name {
                return item.value().to_string();
            }
        }
        panic!("{name} not exist");
    }
}

impl Default for Credential {
    fn default() -> Self {
        Self::new(None)
    }
}
