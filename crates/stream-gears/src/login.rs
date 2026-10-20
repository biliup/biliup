use biliup::error::Result as AppResult;
use biliup::uploader::bilibili::BiliBili;
use biliup::uploader::credential::{Credential, save_login_info};
use std::path::Path;

pub async fn login_by_cookies(file: &str, proxy: Option<&str>) -> AppResult<BiliBili> {
    let login_info = biliup::uploader::credential::login_by_cookies(file, proxy)
        .await
        .map_err(|error| biliup::error::Kind::Custom(format!("cookies file {file}: {error}")))?;
    Ok(login_info)
}

pub async fn send_sms(
    country_code: u32,
    phone: u64,
    proxy: Option<&str>,
) -> AppResult<serde_json::Value> {
    let ret = Credential::new(proxy).send_sms(phone, country_code).await?;
    Ok(ret)
}

pub async fn login_by_sms(
    code: u32,
    res: serde_json::Value,
    proxy: Option<&str>,
    file: &Path,
) -> AppResult<bool> {
    let info = Credential::new(proxy).login_by_sms(code, res).await?;
    save_login_info(file, &info).await?;
    Ok(true)
}

pub async fn get_qrcode(proxy: Option<&str>) -> AppResult<serde_json::Value> {
    let qrcode = Credential::new(proxy).get_qrcode().await?;
    Ok(qrcode)
}

pub async fn login_by_web_cookies(
    sess_data: &str,
    bili_jct: &str,
    proxy: Option<&str>,
    file: &Path,
) -> AppResult<bool> {
    let info = Credential::new(proxy)
        .login_by_web_cookies(sess_data, bili_jct)
        .await?;
    save_login_info(file, &info).await?;
    Ok(true)
}

pub async fn login_by_web_qrcode(
    sess_data: &str,
    dede_user_id: &str,
    proxy: Option<&str>,
    file: &Path,
) -> AppResult<bool> {
    let info = Credential::new(proxy)
        .login_by_web_qrcode(sess_data, dede_user_id)
        .await?;
    save_login_info(file, &info).await?;
    Ok(true)
}
