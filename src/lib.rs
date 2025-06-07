// See the file LICENSE for the full license governing this code.

//! # Ankify V2
//!
//! Advanced Typst to Anki bridge with caching, templating, and watch mode support.
//!
//! ## Features
//!
//! - **Type-safe**: Comprehensive type definitions for all data structures
//! - **Caching**: Intelligent hash-based caching to avoid unnecessary updates
//! - **Templating**: Flexible rendering system with multiple output formats
//! - **Watch mode**: Continuous file monitoring for automatic updates
//! - **Error handling**: Robust error handling with detailed diagnostics
//! - **Testing**: Comprehensive unit and integration tests
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - [`cli`]: Command-line interface and argument parsing
//! - [`config`]: Configuration management and option merging
//! - [`typst`]: Typst integration and card extraction
//! - [`anki`]: AnkiConnect integration and API calls
//! - [`cache`]: Auxiliary file management and caching logic
//! - [`render`]: Card content rendering system
//! - [`watch`]: File system monitoring and change detection
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ankify::{Config, Ankify, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = Config::default();
//!     let mut ankify = Ankify::new(config).await?;
//!     ankify.process_files().await?;
//!     Ok(())
//! }
//! ```

pub mod anki;
pub mod cache;
pub mod cli;
pub mod config;
pub mod render;
pub mod typst;
pub mod utils;
pub mod watch;

mod error;
mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use types::*;
pub use watch::{watch_files, FileWatcher, WatchConfig};

use std::path::Path;
use tracing::{info, instrument};

/// Main entry point for the Ankify library.
///
/// This struct coordinates all the different components to process Typst files
/// and synchronize them with Anki.
#[derive(Debug)]
pub struct Ankify {
    config: config::Config,
    anki_client: anki::Client,
    cache: cache::Cache,
    renderer: render::Renderer,
}

impl Ankify {
    /// Create a new Ankify instance with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object containing all options
    ///
    /// # Returns
    ///
    /// A new Ankify instance or an error if initialization fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use ankify::{Config, Ankify, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let config = Config::default();
    ///     let ankify = Ankify::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    #[instrument]
    pub async fn new(config: config::Config) -> Result<Self> {
        let anki_client = anki::Client::new(&config.ankiconnect_url)?;
        let cache = cache::Cache::new(&config.aux_file)?;
        let renderer = render::Renderer::new(&config.cache_dir).await?;

        Ok(Self {
            config,
            anki_client,
            cache,
            renderer,
        })
    }

    /// Process all files matching the configured patterns.
    ///
    /// This is the main entry point for processing Typst files and updating Anki.
    /// It will:
    ///
    /// 1. Extract cards from all matching Typst files
    /// 2. Compare against the cache to determine what needs updating
    /// 3. Render card content as needed
    /// 4. Execute AnkiConnect API calls
    /// 5. Update the cache
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if processing fails.
    #[instrument(skip(self))]
    pub async fn process_files(&mut self) -> Result<()> {
        info!("Starting file processing");

        // Extract cards from all matching files
        let cards = self.extract_all_cards().await?;
        info!(
            "Extracted {} cards from {} files",
            cards.len(),
            self.get_matching_files()?.len()
        );

        // Determine what operations need to be performed
        let operations = self.cache.plan_operations(&cards)?;
        info!("Planned {} operations", operations.len());

        // Execute operations
        self.execute_operations(operations).await?;

        info!("File processing complete");
        Ok(())
    }

    /// Process a single file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the Typst file to process
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if processing fails.
    #[instrument(skip(self))]
    pub async fn process_file(&mut self, file_path: &Path) -> Result<()> {
        info!(?file_path, "Processing single file");

        // Extract cards from the file
        let cards = typst::extract_cards(file_path).await?;
        info!("Extracted {} cards from file", cards.len());

        // Determine what operations need to be performed
        let operations = self.cache.plan_operations(&cards)?;
        info!("Planned {} operations", operations.len());

        // Execute operations
        self.execute_operations(operations).await?;

        info!(?file_path, "File processing complete");
        Ok(())
    }

    /// Start watching files for changes and process them automatically.
    ///
    /// This method will run indefinitely, monitoring the filesystem for changes
    /// to Typst files and processing them as they are modified.
    ///
    /// # Returns
    ///
    /// This method only returns if an unrecoverable error occurs.
    #[instrument(skip(self))]
    pub async fn start_watch_mode(&mut self) -> Result<()> {
        let watch_paths = self
            .config
            .typst_files
            .iter()
            .filter_map(|pattern| {
                // Convert glob pattern to directory path
                if let Some(parent) = std::path::Path::new(pattern).parent() {
                    Some(parent.to_path_buf())
                } else {
                    Some(std::path::PathBuf::from("."))
                }
            })
            .collect();

        let mut watcher = FileWatcher::new(
            self.config.clone(),
            watch_paths,
            std::time::Duration::from_secs(2),
        );

        watcher.start_watching().await
    }

    // Private helper methods

    async fn extract_all_cards(&self) -> Result<Vec<Card>> {
        let files = self.get_matching_files()?;
        let mut all_cards = Vec::new();

        for file in files {
            let cards = typst::extract_cards(&file).await?;
            all_cards.extend(cards);
        }

        Ok(all_cards)
    }

    fn get_matching_files(&self) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        for pattern in &self.config.typst_files {
            for entry in glob::glob(pattern).map_err(|e| {
                Error::InvalidInput(format!("Invalid glob pattern '{}': {}", pattern, e))
            })? {
                files.push(entry.map_err(|e| Error::FileSystem(format!("Glob error: {}", e)))?);
            }
        }

        Ok(files)
    }

    async fn execute_operations(&mut self, operations: Vec<cache::Operation>) -> Result<()> {
        for operation in operations {
            match operation {
                cache::Operation::Create { card } => {
                    let rendered_card = self.render_card(&card).await?;
                    let anki_id = self.anki_client.create_card(&rendered_card).await?;
                    self.cache.record_creation(&card, anki_id)?;
                }
                cache::Operation::Update { card, anki_id } => {
                    let rendered_card = self.render_card(&card).await?;
                    self.anki_client
                        .update_card(anki_id, &rendered_card)
                        .await?;
                    self.cache.record_update(&card)?;
                }
                cache::Operation::Delete { anki_id } => {
                    self.anki_client.delete_card(anki_id.clone()).await?;
                    self.cache.record_deletion(anki_id)?;
                }
                cache::Operation::Skip { .. } => {
                    // Nothing to do
                }
            }
        }

        self.cache.save()?;
        Ok(())
    }

    async fn render_card(&self, card: &Card) -> Result<RenderedCard> {
        let render_function_path = self.config.render.as_ref().map(|p| p.as_path());
        self.renderer.render_card(card, render_function_path).await
    }
}
