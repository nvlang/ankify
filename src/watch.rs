//! File system monitoring and change detection
//!
//! This module provides functionality for watching Typst files and automatically
//! processing them when changes are detected. It includes debouncing to handle
//! rapid successive changes and efficient change detection.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::Ankify;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, error, info, instrument, warn};

/// File system watcher for automatic card processing
///
/// Monitors specified directories and files for changes, automatically processing
/// Typst files when they are modified. Includes debouncing to prevent excessive
/// processing during rapid edits.
#[derive(Debug)]
pub struct FileWatcher {
    /// Configuration for processing
    config: Config,
    /// Paths to watch
    watch_paths: Vec<PathBuf>,
    /// Debounce duration to wait after changes
    debounce_duration: Duration,
    /// Last modification times for debouncing
    last_changes: HashMap<PathBuf, Instant>,
}

impl FileWatcher {
    /// Create a new file watcher with the specified configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for card processing
    /// * `watch_paths` - Paths to monitor for changes
    /// * `debounce_duration` - Time to wait after changes before processing
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use ankify::{Config, FileWatcher};
    /// use std::time::Duration;
    ///
    /// # async fn example() -> ankify::Result<()> {
    /// let config = Config::default();
    /// let watcher = FileWatcher::new(
    ///     config,
    ///     vec!["./documents".into()],
    ///     Duration::from_secs(2),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: Config, watch_paths: Vec<PathBuf>, debounce_duration: Duration) -> Self {
        Self {
            config,
            watch_paths,
            debounce_duration,
            last_changes: HashMap::new(),
        }
    }

    /// Start watching for file changes
    ///
    /// This method will run indefinitely, monitoring the specified paths for changes
    /// and automatically processing Typst files when they are modified.
    ///
    /// # Errors
    ///
    /// Returns an error if the file watcher cannot be initialized or if there are
    /// issues setting up the watch operations.
    #[instrument(skip(self))]
    pub async fn start_watching(&mut self) -> Result<()> {
        info!(paths = ?self.watch_paths, "Starting file watcher");

        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
            .map_err(|e| Error::WatchError(format!("Failed to create watcher: {}", e)))?;

        // Add all watch paths
        for path in &self.watch_paths {
            info!(?path, "Adding watch path");
            watcher.watch(path, RecursiveMode::Recursive).map_err(|e| {
                Error::WatchError(format!("Failed to watch path {}: {}", path.display(), e))
            })?;
        }

        // Process events in a separate thread and use a tokio channel for communication
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn a blocking task to handle the sync receiver
        let _handle = tokio::task::spawn_blocking(move || {
            while let Ok(event) = rx.recv() {
                if event_tx.send(event).is_err() {
                    break; // Receiver dropped
                }
            }
        });

        // Process events
        let mut debounce_interval = interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                _ = debounce_interval.tick() => {
                    self.process_debounced_changes().await;
                }
                event = event_rx.recv() => {
                    match event {
                        Some(event) => {
                            if let Err(e) = self.handle_file_event(event).await {
                                error!(error = %e, "Error handling file event");
                            }
                        }
                        None => {
                            warn!("File watcher disconnected");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a single file system event
    async fn handle_file_event(
        &mut self,
        event: std::result::Result<Event, notify::Error>,
    ) -> Result<()> {
        let event = event.map_err(|e| Error::WatchError(format!("File event error: {}", e)))?;

        debug!(?event, "Received file event");

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    if self.should_process_file(&path) {
                        debug!(?path, "Marking file for debounced processing");
                        self.last_changes.insert(path, Instant::now());
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    debug!(?path, "File removed, clearing from debounce cache");
                    self.last_changes.remove(&path);
                }
            }
            _ => {
                // Ignore other event types
            }
        }

        Ok(())
    }

    /// Process files that have been debounced
    async fn process_debounced_changes(&mut self) {
        let now = Instant::now();
        let mut files_to_process = Vec::new();

        // Find files that have been stable for the debounce duration
        self.last_changes.retain(|path, last_change| {
            if now.duration_since(*last_change) >= self.debounce_duration {
                files_to_process.push(path.clone());
                false // Remove from debounce cache
            } else {
                true // Keep in cache
            }
        });

        // Process stable files
        for file_path in files_to_process {
            info!(?file_path, "Processing debounced file change");
            if let Err(e) = self.process_file(&file_path).await {
                error!(path = ?file_path, error = %e, "Error processing file");
            }
        }
    }

    /// Process a single file
    async fn process_file(&self, file_path: &Path) -> Result<()> {
        debug!(?file_path, "Processing file");

        // Create a new Ankify instance for this file
        let mut ankify = Ankify::new(self.config.clone()).await?;

        // Process the specific file
        ankify.process_file(file_path).await?;

        info!(?file_path, "Successfully processed file");
        Ok(())
    }

    /// Check if a file should be processed
    fn should_process_file(&self, path: &Path) -> bool {
        // Check if it's a Typst file
        if let Some(extension) = path.extension() {
            if extension == "typ" {
                // Check if the file exists and is readable
                if path.exists() && path.is_file() {
                    debug!(?path, "File should be processed");
                    return true;
                }
            }
        }

        false
    }
}

/// Watch configuration builder
///
/// Provides a convenient way to configure file watching options.
#[derive(Debug)]
pub struct WatchConfig {
    /// Paths to watch
    pub paths: Vec<PathBuf>,
    /// Debounce duration
    pub debounce_duration: Duration,
    /// Whether to process existing files on startup
    pub process_existing: bool,
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            debounce_duration: Duration::from_secs(2),
            process_existing: false,
            include_patterns: vec!["*.typ".to_string()],
            exclude_patterns: vec![".*".to_string(), "target/**".to_string()],
        }
    }
}

impl WatchConfig {
    /// Create a new watch configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a path to watch
    pub fn add_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Set the debounce duration
    pub fn debounce_duration(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }

    /// Set whether to process existing files on startup
    pub fn process_existing(mut self, process: bool) -> Self {
        self.process_existing = process;
        self
    }

    /// Add an include pattern
    pub fn include_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add an exclude pattern
    pub fn exclude_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// Build a FileWatcher with this configuration
    pub fn build(self, config: Config) -> FileWatcher {
        FileWatcher::new(config, self.paths, self.debounce_duration)
    }
}

/// Utility function to start watching with default settings
///
/// # Arguments
///
/// * `config` - Ankify configuration
/// * `paths` - Paths to watch
///
/// # Examples
///
/// ```rust,no_run
/// use ankify::{Config, watch_files};
///
/// # async fn example() -> ankify::Result<()> {
/// let config = Config::default();
/// watch_files(config, vec!["./documents"]).await?;
/// # Ok(())
/// # }
/// ```
pub async fn watch_files(config: Config, paths: Vec<impl Into<PathBuf>>) -> Result<()> {
    let paths = paths.into_iter().map(|p| p.into()).collect();
    let mut watcher = FileWatcher::new(config, paths, Duration::from_secs(2));
    watcher.start_watching().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_watch_config_builder() {
        let config = WatchConfig::new()
            .add_path("./test")
            .debounce_duration(Duration::from_secs(1))
            .process_existing(true)
            .include_pattern("*.typ")
            .exclude_pattern(".*");

        assert_eq!(config.paths.len(), 2); // default "." + added "./test"
        assert_eq!(config.debounce_duration, Duration::from_secs(1));
        assert!(config.process_existing);
        assert!(config.include_patterns.contains(&"*.typ".to_string()));
        assert!(config.exclude_patterns.contains(&".*".to_string()));
    }

    #[test]
    fn test_should_process_file() {
        let config = Config::default();
        let watcher = FileWatcher::new(config, vec![], Duration::from_secs(1));

        // Should not process non-existent files
        assert!(!watcher.should_process_file(Path::new("nonexistent.typ")));

        // Should not process non-Typst files
        assert!(!watcher.should_process_file(Path::new("test.txt")));
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let config = Config::default();
        let temp_dir = TempDir::new().unwrap();

        let watcher = FileWatcher::new(
            config,
            vec![temp_dir.path().to_path_buf()],
            Duration::from_millis(100),
        );

        assert_eq!(watcher.watch_paths.len(), 1);
        assert_eq!(watcher.debounce_duration, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_debounce_logic() {
        let config = Config::default();
        let mut watcher = FileWatcher::new(config, vec![], Duration::from_millis(50));

        let test_path = PathBuf::from("test.typ");

        // Add a change
        watcher
            .last_changes
            .insert(test_path.clone(), Instant::now());

        // Should not process immediately
        watcher.process_debounced_changes().await;
        assert!(watcher.last_changes.contains_key(&test_path));

        // Wait for debounce duration and try again
        sleep(Duration::from_millis(60)).await;
        watcher.process_debounced_changes().await;

        // Should be removed from cache (even though processing fails)
        assert!(!watcher.last_changes.contains_key(&test_path));
    }
}
