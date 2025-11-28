use time::macros::format_description;

use biliup::uploader::util::SubmitOption;
use biliup_cli::cli::{Cli, Commands, expand_path};
use biliup_cli::downloader::{download, generate_json};
use biliup_cli::uploader::{append, list, login, renew, show, upload_by_command, upload_by_config};

use clap::Parser;

use biliup_cli::server::errors::AppResult;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> AppResult<()> {
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

    let console_filter = tracing_subscriber::EnvFilter::new(&cli.rust_log);
    // let (file_filter_layer, file_reload_handle) = reload::Layer::new(file_filter);
    let (console_filter_layer, console_reload_handle) = reload::Layer::new(console_filter);
    tracing_subscriber::registry()
        .with(console_filter_layer)
        .with(tracing_subscriber::fmt::layer().with_timer(timer))
        .init();

    let user_cookie = expand_path(cli.user_cookie);

    match cli.command {
        Commands::Login => login(user_cookie, cli.proxy.as_deref()).await?,
        Commands::Renew => {
            renew(user_cookie, cli.proxy.as_deref()).await?;
        }
        Commands::Upload {
            video_path,
            config: None,
            line,
            limit,
            studio,
            submit,
        } => {
            let video_path: Vec<_> = video_path.into_iter().map(expand_path).collect();
            upload_by_command(
                studio,
                user_cookie,
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
            let config = expand_path(config);
            upload_by_config(config, user_cookie, submit, cli.proxy.as_deref()).await?;
        }
        Commands::Append {
            video_path,
            vid,
            line,
            limit,
            studio: _,
            submit,
        } => {
            let video_path: Vec<_> = video_path.into_iter().map(expand_path).collect();
            append(
                user_cookie,
                vid,
                video_path,
                line,
                limit,
                submit.unwrap_or(SubmitOption::App),
                cli.proxy.as_deref(),
            )
            .await?
        }
        Commands::Show { vid } => show(user_cookie, vid, cli.proxy.as_deref()).await?,
        Commands::DumpFlv { file_name } => {
            let file_name = expand_path(file_name);
            generate_json(file_name)?
        }
        Commands::Download {
            url,
            output,
            split_size,
            split_time,
        } => download(&url, output, split_size, split_time).await?,
        Commands::Server { bind, port, auth } => {
            biliup_cli::run((&bind, port), auth, console_reload_handle).await?
        }
        Commands::List {
            is_pubing,
            pubed,
            not_pubed,
            from_page,
            max_pages,
        } => {
            list(
                user_cookie,
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
