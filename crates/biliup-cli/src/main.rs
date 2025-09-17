mod cli;
mod downloader;
mod uploader;

use anyhow::Result;
use time::macros::format_description;

use crate::cli::{Cli, Commands};
use crate::downloader::{download, generate_json};
use crate::uploader::{append, list, login, renew, show, upload_by_command, upload_by_config};
use biliup::uploader::util::SubmitOption;

use clap::Parser;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    // a builder for `FmtSubscriber`.
    // let subscriber = FmtSubscriber::builder()
    //     // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
    //     // will be written to stdout.
    //     .with_max_level(Level::INFO)
    //     // completes the builder.
    //     .finish();

    // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let cli = Cli::parse();

    // use of deprecated function `time::util::local_offset::set_soundness`: no longer needed; TZ is refreshed manually
    // unsafe {
    //     time::util::local_offset::set_soundness(time::util::local_offset::Soundness::Unsound);
    // }

    let timer = tracing_subscriber::fmt::time::LocalTime::new(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ));

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&cli.rust_log))
        .with(tracing_subscriber::fmt::layer().with_timer(timer))
        .init();

    match cli.command {
        Commands::Login => login(cli.user_cookie, cli.proxy.as_deref()).await?,
        Commands::Renew => {
            renew(cli.user_cookie, cli.proxy.as_deref()).await?;
        }
        Commands::Upload {
            video_path,
            config: None,
            line,
            limit,
            studio,
            submit,
        } => {
            upload_by_command(
                studio,
                cli.user_cookie,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::Upload {
            video_path: _,
            config: Some(config),
            submit,
            ..
        } => {
            upload_by_config(config, cli.user_cookie, submit, cli.proxy.as_deref()).await?;
        }
        Commands::Append {
            video_path,
            vid,
            line,
            limit,
            studio: _,
            submit,
        } => {
            append(
                cli.user_cookie,
                vid,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::Show { vid } => show(cli.user_cookie, vid, cli.proxy.as_deref()).await?,
        Commands::DumpFlv { file_name } => generate_json(file_name)?,
        Commands::Download {
            url,
            output,
            split_size,
            split_time,
        } => download(&url, output, split_size, split_time).await?,
        Commands::Server { bind, port } => biliup_cli::run((&bind, port)).await?,
        Commands::List {
            is_pubing,
            pubed,
            not_pubed,
            from_page,
            max_pages,
        } => {
            list(
                cli.user_cookie,
                is_pubing,
                pubed,
                not_pubed,
                cli.proxy.as_deref(),
                from_page,
                max_pages,
            )
            .await?
        }
    };
    Ok(())
}
