//! `biliup user list` / `biliup user reset-password <用户名>`：忘记密码时的找回途径（#1717 取舍 6）。

use crate::cli::UserAction;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::users::{Backend, UpdateUserError, validate_new_password};
use error_stack::{Report, ResultExt, bail};
use std::io::{BufRead, IsTerminal};
use std::path::Path;

const DATABASE: &str = "data/data.sqlite3";

pub async fn run(action: UserAction) -> AppResult<()> {
    if !Path::new(DATABASE).is_file() {
        bail!(AppError::Custom(format!(
            "当前目录下没有 {DATABASE}；请在 biliup 服务的工作目录下执行（Docker 下用 docker exec）"
        )));
    }
    let backend = Backend::new(ConnectionManager::new_pool(DATABASE).await?);
    match action {
        UserAction::List => list(&backend).await,
        UserAction::ResetPassword { username } => {
            let password = read_new_password()?;
            reset_password(&backend, &username, password).await
        }
    }
}

async fn list(backend: &Backend) -> AppResult<()> {
    let users = backend
        .list_users()
        .await
        .change_context(AppError::Unknown)?;
    if users.is_empty() {
        println!("还没有任何 Web 用户；打开 Web 界面即可注册第一个超级管理员。");
        return Ok(());
    }
    println!("{:<6} {:<32} {:<9} 状态", "ID", "用户名", "角色");
    for user in users {
        println!(
            "{:<6} {:<32} {:<9} {}",
            user.id,
            user.username,
            user.role,
            if user.disabled { "已禁用" } else { "启用" }
        );
    }
    Ok(())
}

async fn reset_password(backend: &Backend, username: &str, password: String) -> AppResult<()> {
    match backend.reset_password(username, password).await {
        Ok(user) => {
            println!("已重置 {} 的密码，其所有登录会话已失效。", user.username);
            if user.disabled {
                println!(
                    "注意：该用户目前处于禁用状态，需要超级管理员在 Web 界面里启用后才能登录。"
                );
            }
            Ok(())
        }
        Err(UpdateUserError::NotFound) => Err(Report::new(AppError::Custom(format!(
            "用户 {username} 不存在；用 `biliup user list` 查看现有用户"
        )))),
        Err(UpdateUserError::Invalid(reason)) => Err(Report::new(AppError::Custom(reason.into()))),
        Err(error) => Err(Report::new(error).change_context(AppError::Unknown)),
    }
}

fn read_new_password() -> AppResult<String> {
    let password = if std::io::stdin().is_terminal() {
        dialoguer::Password::new()
            .with_prompt("新密码")
            .with_confirmation("再输入一次", "两次输入不一致")
            .interact()
            .change_context(AppError::Unknown)?
    } else {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .change_context(AppError::Unknown)?;
        line.trim_end_matches(['\r', '\n']).to_string()
    };
    validate_new_password(&password)
        .map_err(|reason| Report::new(AppError::Custom(reason.into())))?;
    Ok(password)
}
