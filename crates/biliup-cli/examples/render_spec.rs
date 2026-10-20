//! Runs a frozen render job through the composition engine without a server
//! or database, for acceptance checks against earlier exports:
//!
//!     cargo run -p biliup-cli --example render_spec -- render-spec.json BASE_DIR OUT_DIR
//!
//! `render-spec.json` is a job snapshot (`RenderSpec`); its relative paths are
//! resolved against BASE_DIR. OUT_DIR receives `result.mp4` and, for every
//! segment, the `danmaku-<n>.ass` the export burnt in.

use biliup_cli::server::workbench::renders::engine::{self, EngineRequest};
use biliup_cli::server::workbench::renders::model::RenderSpec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [spec, base, out] = args.as_slice() else {
        eprintln!("usage: render_spec render-spec.json BASE_DIR OUT_DIR");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    if let Err(error) = runtime.block_on(run(spec, base, out)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run(spec: &Path, base: &Path, out: &Path) -> Result<(), String> {
    let spec: RenderSpec = serde_json::from_slice(&std::fs::read(spec).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let resolve = |path: &Path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        }
    };
    let mut sources = spec.sources.clone();
    for source in &mut sources {
        source.path = resolve(&source.path);
        source.danmaku_path = source.danmaku_path.as_deref().map(resolve);
    }
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    // The ASS of every segment, exactly as the export computes it.
    for (number, source) in sources.iter().enumerate() {
        let preview = engine::preview(source, &spec.recipe, &CancellationToken::new()).await?;
        std::fs::write(out.join(format!("danmaku-{number}.ass")), &preview.ass)
            .map_err(|e| e.to_string())?;
        println!(
            "segment {number}: {} comments, origin {} ms",
            preview.comment_count, preview.time_origin_ms
        );
    }
    let started = std::time::Instant::now();
    let done = engine::render(
        EngineRequest {
            sources,
            settings: spec.recipe.clone(),
            assets: spec
                .assets
                .iter()
                .map(|asset| (asset.id.to_string(), resolve(&asset.path)))
                .collect::<HashMap<_, _>>(),
            output_path: out.join("result.mp4"),
            font_path: spec.font_path.as_deref().map(resolve),
        },
        CancellationToken::new(),
        |progress| {
            if let Some(ratio) = progress.ratio {
                eprint!("\r{} {:5.1}%", progress.phase, ratio * 100.);
            }
        },
    )
    .await?;
    eprintln!();
    println!(
        "{} bytes, {} ms, rendered in {:.0?}",
        done.output_bytes,
        done.duration_ms,
        started.elapsed()
    );
    Ok(())
}
