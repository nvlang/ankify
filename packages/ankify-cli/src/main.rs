//! Binary crate, providing the command-line interface for Ankify.

use ankify::sync::{sync, SyncConfig};
use clap::{Arg, Command};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    let matches = Command::new("ankify")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("file")
                .help("The Typst source file to process")
                .required(true)
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cache-file")
                .long("cache-file")
                .help("Custom cache file path")
                .value_name("PATH"),
        )
        .arg(
            Arg::new("ankiconnect-url")
                .long("ankiconnect-url")
                .help("Custom AnkiConnect URL")
                .value_name("URL")
                .default_value("http://127.0.0.1:8765"),
        )
        .arg(
            Arg::new("root")
                .long("root")
                .help("Root directory for Typst compilation")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("font-path")
                .long("font-path")
                .help("Additional font paths for Typst")
                .value_name("PATH")
                .action(clap::ArgAction::Append),
        )
        .get_matches();

    // Extract arguments
    let source_file = PathBuf::from(matches.get_one::<String>("file").unwrap());
    let verbose = matches.get_flag("verbose");
    let cache_file = matches.get_one::<String>("cache-file").map(PathBuf::from);
    let ankiconnect_url = matches
        .get_one::<String>("ankiconnect-url")
        .map(String::from);

    // Build extra arguments for Typst
    let mut extra_args = Vec::new();

    if let Some(root) = matches.get_one::<String>("root") {
        extra_args.push("--root".to_string());
        extra_args.push(root.clone());
    }

    if let Some(font_paths) = matches.get_many::<String>("font-path") {
        for path in font_paths {
            extra_args.push("--font-path".to_string());
            extra_args.push(path.clone());
        }
    }

    // Validate source file exists
    if !source_file.exists() {
        error!("Source file does not exist: {}", source_file.display());
        process::exit(1);
    }

    // Create sync configuration
    let mut config = SyncConfig::new(&source_file)
        .with_verbose(verbose)
        .with_cli_mode(true)
        .with_extra_args(extra_args);

    if let Some(cache_file) = cache_file {
        config = config.with_cache_file(cache_file);
    }

    if let Some(url) = ankiconnect_url {
        config = config.with_ankiconnect_url(url);
    }

    // Perform the sync
    match sync(config).await {
        Ok(result) => {
            info!("Sync completed successfully!");
            info!("Notes added: {}", result.notes_added);
            info!("Notes updated: {}", result.notes_updated);
            info!("Notes unchanged: {}", result.notes_unchanged);
            if result.decks_created > 0 {
                info!("Decks created: {}", result.decks_created);
            }

            if !result.warnings.is_empty() {
                info!("Warnings:");
                for warning in &result.warnings {
                    info!("  - {}", warning);
                }
            }
        }
        Err(e) => {
            error!("Sync failed: {}", e);
            process::exit(1);
        }
    }
}
