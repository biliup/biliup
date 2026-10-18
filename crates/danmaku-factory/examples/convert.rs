//! Converts one XML file with the native Rust converter:
//!
//!     cargo run -p danmaku-factory --example convert -- in.xml out.ass '{"width":1280,"height":720}'
//!
//! The optional JSON holds `ConvertOptions` fields; the rest keep the CLI defaults.

use danmaku_factory::{ConvertOptions, convert_xml_to_ass};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output, rest @ ..] = args.as_slice() else {
        eprintln!("usage: convert IN.xml OUT.ass [OPTIONS_JSON]");
        std::process::exit(2);
    };
    let options: ConvertOptions = match rest.first() {
        Some(json) => serde_json::from_str(json).unwrap_or_else(|e| {
            eprintln!("options: {e}");
            std::process::exit(2);
        }),
        None => ConvertOptions::default(),
    };
    match convert_xml_to_ass(&PathBuf::from(input), &PathBuf::from(output), &options) {
        Ok(stats) => println!(
            "{} danmaku, {} blocked ({})",
            stats.danmaku_count,
            stats.blocked_count,
            danmaku_factory::version()
        ),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
