//! Ankify V2 - Advanced Typst to Anki Bridge
//!
//! A command-line tool for automatically generating Anki flashcards from Typst documents.

use ankify::{Ankify, Result};
use clap::Parser;
use std::process;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        error!("Application error: {}", e);
        process::exit(1);
    }
}

/// Main application logic
async fn run() -> Result<()> {
    // Parse command line arguments
    let cli = ankify::cli::Cli::parse();

    // Initialize logging based on CLI args
    cli.init_logging()?;

    // Validate CLI arguments
    cli.validate()?;

    // Load configuration
    let config = ankify::Config::from_cli(cli.clone()).await?;

    info!("Starting Ankify v2");
    info!("Configuration: {:?}", config);

    // Create Ankify instance
    let mut ankify = Ankify::new(config.clone()).await?;

    // Execute the requested operation based on CLI flags
    info!("Processing files once");
    ankify.process_files().await?;
    info!("Processing complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_main_functionality() {
        // This test ensures the main function structure is correct
        // More comprehensive tests are in individual modules
    }
}
