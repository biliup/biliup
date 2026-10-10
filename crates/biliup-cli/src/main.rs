use biliup_cli::server::errors::AppResult;
use biliup_cli::{danmaku_worker, entry};

fn main() -> AppResult<()> {
    // Before the runtime, logging and argument parsing: a danmaku conversion
    // started by the server runs nothing else.
    if let Some(code) = danmaku_worker::run_if_requested(std::env::args_os()) {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> AppResult<()> {
    let cli = entry::parse(std::env::args_os()).unwrap_or_else(|e| e.exit());
    entry::run(cli).await
}
