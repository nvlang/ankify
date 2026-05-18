//! This module is the heart of Ankify. It'll provide a `sync` function that is
//! the sole (or at least main) public API of the Ankify library and binary
//! crates. This `sync` function should operate in the following steps:
//!
//! 1.  *Check AnkiConnect.*
//!     Check if the AnkiConnect server is running. If not, return an error.
//!     Otherwise, continue.
//!
//! 2.  *Generate temp file and read cache.*
//!     In parallel:
//!
//!     -   Call the `generate` module to generate the temporary Typst files
//!         that will be used to create the Anki notes.
//!     -   Call the `cache` module to load the cache of existing notes.
//!
//! 3.  *Calling Typst.*
//!     In parallel:
//!
//!     -   Call the `query` module to query the Typst source file for metadata
//!         about the user's Ankify settings, as well as about the notes to be
//!         created.
//!     -   If the cache has any fields in PNG/SVG format (and remember, PNG is
//!         the default), call the `compile` module on the generated Typst file
//!         with the corresponding output format flags. If both PNG and SVG
//!         files are being output, be sure to run the two compilations in
//!         parallel.
//!
//! 4.  *Pick output files.*
//!     Use the output from the `query` module to determine which output files
//!     are relevant.
//!
//! 5.  *Hashing.*
//!     In parallel: Hash each of the relevant output files.
//!
//! 6.  *Decision-making.*
//!     Create a `RequestList` that contains the requests (or lists of requests)
//!     to be sent to be sent to AnkiConnect. To do so, compare the list of
//!     notes provided by the `query` module with the notes in the cache to
//!     determine which notes are new and need to be added to Anki for the first
//!     time, and which notes are already in Anki and simply need to be updated.
//!     Then, taking advantage of the `ankiconnect` module, do the following:
//!
//!     -   For new notes:
//!
//!         1.  Check if the notes' decks are the same as the decks of any
//!             other notes in the cache. If so, we can assume that the
//!             decks already exist in Anki; otherwise, we need to send a
//!             `deckNamesAndIds` request to AnkiConnect to check if the
//!             decks exist, and, if they (or at least some of them) don't,
//!             we need to send a `createDeck` request (or multiple
//!             `createDeck` requests, grouped into a single `RequestList`
//!             with `multi` set to `true`) _before_ adding the notes.
//!         2.  Create an `addNotes` request for the new notes.
//!
//!     -   For existing notes:
//!
//!         -   Compare the new hashes with the hashes from the cache to
//!             determine which output files ones need to be updated in the
//!             Anki database. Note that some fields may not have had an
//!             output file associated with them in the cache, which would
//!             mean that they were either omitted before, or merely
//!             contained plain text. In either case, if the new note has an
//!             output file associated with the field, we should update the
//!             field in the Anki database accordingly. Conversely, if the
//!             cache has an output file associated with the field, but the
//!             new note does not, we should remove the field from the Anki
//!             database; if the new note has plain text in the field, then
//!             we should update the field in the Anki database with the
//!             plain text.
//!         -   Compare the tags of the new note with the tags in the cache
//!             to determine if the tags need to be updated in the Anki
//!             database.
//!
//!         The `updateNote` requests should be grouped into a single
//!         `RequestList` with the `multi` field set to `true`, so that
//!         the requests are sent to AnkiConnect simultaneously.
//!
//!     Note that, if a note's field has an output file associated with it, then
//!     the field should be set to `"<img class=\"ankify\"
//!     src=\"〈output-file〉\"/>"` in the note's `fields` field, and the output
//!     file should be included _in the same request_ (be it an `addNotes` or an
//!     `updateNote` request) in the `picture` array, with `url` set to the path
//!     to the output file, `filename` set to `"〈output-file〉"`, and `fields`
//!     set to `["〈field〉"]`.
//!
//! 7.  *Execution.* Process the `RequestList` and send the corresponding
//!     requests AnkiConnect. Make use of the `ankiconnect` module to understand
//!     the responses from AnkiConnect. While doing all this, be sure to handle
//!     any errors that may occur, and keep the cache up to date with the
//!     changes made to the Anki database (i.e., when `addNotes` or `updateNote`
//!     requests succeed).
//!
//!     If this is running in a CLI context (i.e., if it's called from the
//!     binary crate), then we furthermore want to provide some pretty output to
//!     the user while all this is happening, including progress bars, error
//!     messages, and so on. If the `verbose` setting is enabled in the Ankify
//!     configuration (as returned by the `query` module), or if the user set
//!     the `--verbose` flag in the CLI, we should provide more detailed output
//!     than we would otherwise.
//!
//! 8.  *Cleanup.*
//!     Delete the temporary file that were generated in step 2, and the output
//!     files that were produced in step 3.

//! ## Technical Implementation Overview
//!
//! The sync module implements the above workflow through the following key components:
//!
//! ### Main Entry Point
//! - `sync(config: SyncConfig)` - Public API that orchestrates the entire sync process
//! - `sync_internal(ctx: &mut SyncContext, result: &mut SyncResult)` - Internal implementation
//!
//! ### Core Data Structures
//! - `SyncContext` - Holds all state needed for sync operation (config, cache, clients, temp files)
//! - `ProcessedNote` - Represents a note with metadata, compiled Anki note, field hashes, and new/update status
//! - `RequestList` - Contains AnkiConnect requests to be executed, supports both single and multi-request batches
//!
//! ### Compilation Pipeline
//! 1. `generate_temp_file()` - Creates temporary Typst file that imports source and renders all fields
//! 2. `compile_temp_file()` - Runs Typst compilation to generate PNG/SVG output files for each field
//! 3. `associate_files_with_notes()` - Maps output files to specific note fields using alphabetical ordering
//! 4. `create_media_file()` - Creates MediaFile objects with base64-encoded data and proper filename format
//!
//! ### Field Processing Logic
//! - Fields with `format: "plain"` → Direct text content, no compilation
//! - Fields with `format: "png"` or `format: "svg"` → Compiled to images, referenced by filename
//! - Default format (PNG) applied when no explicit format specified
//! - AnkiConnect automatically generates `<img>` tags from filenames in `picture` array
//!
//! ### Request Generation
//! - `create_request_list()` - Analyzes processed notes to determine required AnkiConnect operations
//! - Deck creation requests generated first for any new decks
//! - `AddNotes` requests for truly new notes (not in cache)
//! - `UpdateNote` requests for existing notes with changed field hashes
//! - Requests batched using `multi: true` for efficiency
//!
//! ### Execution and Caching
//! - `execute_requests()` - Sends requests to AnkiConnect sequentially to maintain proper ordering
//! - `update_cache_with_added_notes()` - Updates cache with new note IDs returned by AnkiConnect
//! - Field hashing using `cache.create_field_hashes_from_note_data()` for change detection
//! - Cache persistence for subsequent runs to enable incremental updates

use crate::ankiconnect::{AnkiAction, AnkiConnect, Field, Note as AnkiNote, NoteId};
use crate::cache::{Cache, CacheEntry, Sha256};
use crate::compile::{compile_temp_file, CompileConfig, Format};
use crate::error::{Error, Result};
use crate::generate::generate_temp_file;
use crate::metadata::{AnkiConnectChecks, CompletedNote, CompletedTypstAnkifyConfiguration};
use crate::query::{
    complete_ankify_notes_metadata, query_ankify_configuration, query_ankify_notes,
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

/// Configuration for the sync operation.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Path to the Typst source file.
    pub source_file: PathBuf,
    /// Whether to enable verbose output.
    pub verbose: bool,
    /// Optional custom cache file path.
    pub cache_file: Option<PathBuf>,
    /// Optional custom AnkiConnect URL.
    pub ankiconnect_url: Option<String>,
    /// Extra arguments to pass to Typst commands.
    pub extra_args: Vec<String>,
    /// Whether this is running in CLI context (affects progress reporting).
    pub cli_mode: bool,
    /// Keep the generated temp file and rendered images instead of deleting
    /// them after a successful sync. Useful for debugging and tests.
    pub keep_artifacts: bool,
}

impl SyncConfig {
    /// Create a new sync configuration.
    pub fn new<P: Into<PathBuf>>(source_file: P) -> Self {
        Self {
            source_file: source_file.into(),
            verbose: false,
            cache_file: None,
            ankiconnect_url: None,
            extra_args: Vec::new(),
            cli_mode: false,
            keep_artifacts: false,
        }
    }

    /// Enable verbose output.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set custom cache file path.
    pub fn with_cache_file<P: Into<PathBuf>>(mut self, cache_file: P) -> Self {
        self.cache_file = Some(cache_file.into());
        self
    }

    /// Set custom AnkiConnect URL.
    pub fn with_ankiconnect_url<S: Into<String>>(mut self, url: S) -> Self {
        self.ankiconnect_url = Some(url.into());
        self
    }

    /// Set extra arguments for Typst commands.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Enable CLI mode for progress reporting.
    pub fn with_cli_mode(mut self, cli_mode: bool) -> Self {
        self.cli_mode = cli_mode;
        self
    }

    /// Keep generated artifacts (temp file, rendered images) after a successful
    /// sync instead of cleaning them up.
    pub fn with_keep_artifacts(mut self, keep_artifacts: bool) -> Self {
        self.keep_artifacts = keep_artifacts;
        self
    }
}

/// Result of a sync operation.
#[derive(Debug)]
pub struct SyncResult {
    /// Number of notes added to Anki.
    pub notes_added: usize,
    /// Number of notes updated in Anki.
    pub notes_updated: usize,
    /// Number of notes that were already up to date.
    pub notes_unchanged: usize,
    /// Number of decks created.
    pub decks_created: usize,
    /// List of any errors that occurred but didn't prevent the sync.
    pub warnings: Vec<String>,
}

impl SyncResult {
    /// Create a new empty sync result.
    pub fn new() -> Self {
        Self {
            notes_added: 0,
            notes_updated: 0,
            notes_unchanged: 0,
            decks_created: 0,
            warnings: Vec::new(),
        }
    }

    /// Get the total number of notes processed.
    pub fn total_notes(&self) -> usize {
        self.notes_added + self.notes_updated + self.notes_unchanged
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestList {
    /// Indicates whether the requests should be sent to AnkiConnect
    /// inside of a `"multi"` request or not.
    pub multi: bool,

    /// List of requests or lists of requests to be sent to AnkiConnect.
    pub requests: Vec<RequestOrRequestList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestOrRequestList {
    /// A single request to be sent to AnkiConnect.
    Single(serde_json::Value),

    /// A list of requests to be sent to AnkiConnect. Note that the `sequential`
    /// field must be respected when processing this list.
    List(RequestList),
}

/// Internal structure for tracking note processing.
#[derive(Debug)]
struct ProcessedNote {
    /// The metadata note from Typst.
    metadata: CompletedNote,
    /// The compiled Anki note with media files.
    anki_note: AnkiNote,
    /// Field content hashes for cache comparison.
    field_hashes: HashMap<Field, Option<Sha256>>,
    /// Whether this is a new note or an update.
    is_new: bool,
}

/// Context for the sync operation.
pub struct SyncContext {
    config: SyncConfig,
    anki_client: AnkiConnect,
    http_client: Client,
    cache: Cache,
    /// Whether the cache should be persisted; cleared when the document
    /// disables caching.
    cache_enabled: bool,
    temp_files: Vec<PathBuf>,
    output_files: HashMap<Format, Vec<PathBuf>>,
}

impl SyncContext {
    /// Create a new sync context.
    async fn new(config: SyncConfig) -> Result<Self> {
        // Load cache
        let cache = if let Some(cache_file) = &config.cache_file {
            Cache::load_from_file(cache_file).await?
        } else {
            // Default cache file location: .ankify/cache.json in the source file's directory
            let source_dir = config
                .source_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let cache_file = source_dir.join(".ankify").join("cache.json");
            Cache::load_from_file(cache_file).await?
        };

        // Create AnkiConnect client
        let anki_client = if let Some(url) = &config.ankiconnect_url {
            AnkiConnect::with_url(url.clone())
        } else {
            AnkiConnect::new()
        };

        let http_client = Client::new();

        Ok(Self {
            config,
            anki_client,
            http_client,
            cache,
            cache_enabled: true,
            temp_files: Vec::new(),
            output_files: HashMap::new(),
        })
    }

    /// Clean up temporary files and output files.
    pub async fn cleanup(&self) -> Result<()> {
        for file in &self.temp_files {
            if file.exists() {
                if let Err(e) = fs::remove_file(file).await {
                    warn!("Failed to remove temporary file {}: {}", file.display(), e);
                }
            }
        }

        for files in self.output_files.values() {
            for file in files {
                if file.exists() {
                    if let Err(e) = fs::remove_file(file).await {
                        warn!("Failed to remove output file {}: {}", file.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Save the cache, unless the document disabled caching.
    async fn save_cache(&self) -> Result<()> {
        if !self.cache_enabled {
            return Ok(());
        }
        self.cache.save().await
    }

    /// Apply the settings from the document's `configure()` block: redirect the
    /// AnkiConnect URL and cache when the document asks for it (a CLI flag
    /// always wins), and raise the log level if verbose output was requested.
    async fn apply_document_configuration(
        &mut self,
        config: &CompletedTypstAnkifyConfiguration,
    ) -> Result<()> {
        // Verbose: either the CLI flag or the document may request it.
        if self.config.verbose || config.verbose {
            crate::logging::enable_verbose_logging();
        }

        // AnkiConnect URL: a CLI flag wins; otherwise use the document's value.
        if self.config.ankiconnect_url.is_none() {
            self.anki_client = AnkiConnect::with_url(config.ankiconnect_url.clone());
        }

        // Cache: a disabled cache becomes ephemeral; otherwise the document's
        // `custom-file` is honoured unless the CLI passed `--cache-file`.
        if !config.cache.enabled.unwrap_or(true) {
            warn!("Caching is disabled; every note will be treated as new");
            self.cache = Cache::new();
            self.cache_enabled = false;
        } else if self.config.cache_file.is_none() {
            if let Some(custom) = &config.cache.custom_file {
                let custom_path = std::path::Path::new(custom);
                let resolved = if custom_path.is_absolute() {
                    custom_path.to_path_buf()
                } else {
                    self.config
                        .source_file
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join(custom_path)
                };
                self.cache = Cache::load_from_file(&resolved).await?;
            }
        }

        Ok(())
    }
}

/// The main sync function that orchestrates the entire synchronization process.
pub async fn sync(config: SyncConfig) -> Result<SyncResult> {
    if config.cli_mode {
        info!("Starting Ankify sync for {}", config.source_file.display());
    }

    let mut ctx = SyncContext::new(config).await?;
    let mut result = SyncResult::new();

    // Ensure cleanup happens even if we encounter errors
    let sync_result = sync_internal(&mut ctx, &mut result).await;

    // Always try to save the cache, even on failure: any notes that did sync
    // before the error should not be re-sent next time.
    if let Err(e) = ctx.save_cache().await {
        warn!("Failed to save cache: {}", e);
    }

    // On success, remove the generated temp file and rendered images (unless
    // the caller asked to keep them). On failure they are kept so the user (or
    // developer) can inspect them.
    if sync_result.is_ok() && !ctx.config.keep_artifacts {
        if let Err(e) = ctx.cleanup().await {
            warn!("Cleanup failed: {}", e);
        }
    }

    sync_result.map(|_| result)
}

/// Internal sync implementation.
async fn sync_internal(ctx: &mut SyncContext, result: &mut SyncResult) -> Result<()> {
    // Honour the user's --root / --font-path flags, defaulting --root to the
    // current directory when the user did not pass one.
    let extra_args = ctx.config.extra_args.clone();
    let mut query_args: Vec<&str> = extra_args.iter().map(String::as_str).collect();
    if !query_args
        .iter()
        .any(|a| *a == "--root" || a.starts_with("--root="))
    {
        query_args.insert(0, ".");
        query_args.insert(0, "--root");
    }

    // Query the document's `configure()` block first: it may redirect the
    // AnkiConnect URL, the cache, and the log level before any are used.
    let ankify_config =
        query_ankify_configuration(&ctx.config.source_file, Some(&query_args)).await?;
    ctx.apply_document_configuration(&ankify_config).await?;

    // Step 1: Check AnkiConnect, using the now-resolved URL.
    check_ankiconnect(&ctx.anki_client, &ctx.http_client).await?;

    // Step 2: Generate the temporary render file.
    let temp_file = generate_temp_file(&crate::generate::GenerateConfig {
        source_file: ctx.config.source_file.clone(),
        output_dir: None,
    })?;
    ctx.temp_files.push(temp_file.clone());

    // Step 3: Query Typst for the notes.
    let metadata_notes = query_ankify_notes(&ctx.config.source_file, Some(&query_args)).await?;

    if metadata_notes.is_empty() {
        if ctx.config.cli_mode {
            info!("No notes found in {}", ctx.config.source_file.display());
        }
        return Ok(());
    }

    if ctx.config.cli_mode {
        info!("Found {} notes to process", metadata_notes.len());
    }

    let completed_metadata_notes = complete_ankify_notes_metadata(metadata_notes, &ankify_config);

    // Validate the notes against Anki's existing decks, models, and tags, per
    // the document's `checks` configuration.
    if let Some(checks) = &ankify_config.checks.ankiconnect {
        run_ankiconnect_checks(
            checks,
            &completed_metadata_notes,
            &ctx.anki_client,
            &ctx.http_client,
            result,
        )
        .await?;
    }

    // Step 3 continued: Compile the temporary file to generate output files
    let compile_config = CompileConfig::new(
        temp_file.clone(),
        ctx.config.source_file.clone(),
        temp_file.parent().unwrap().join("output"),
        completed_metadata_notes.clone(),
    )
    .with_extra_args(ctx.config.extra_args.clone());

    // Create output directory
    tokio::fs::create_dir_all(&compile_config.output_dir)
        .await
        .map_err(|e| Error::custom(format!("Failed to create output directory: {}", e)))?;

    let compile_result = compile_temp_file(&compile_config).await?;
    ctx.output_files.extend(compile_result.output_files);

    // Step 4: Pick output files (done by compilation)
    // Step 5: Hash files (in parallel)
    let processed_notes =
        process_notes_with_hashes(&completed_metadata_notes, &compile_result.notes, &ctx.cache)
            .await?;

    // Step 6: Decision-making - create RequestList
    let request_list = create_request_list(&processed_notes, &ctx.cache, &ctx.anki_client).await?;

    // Step 7: Execution - send requests to AnkiConnect
    execute_requests(
        &request_list,
        &ctx.anki_client,
        &ctx.http_client,
        &mut ctx.cache,
        result,
        &processed_notes,
    )
    .await?;

    // Refresh cache entries for existing notes. New notes were cached when
    // `addNotes` returned their IDs; existing notes must be refreshed here so
    // that an updated note is not detected as "changed" again on the next sync.
    // (Reaching this point means every request succeeded — `execute_requests`
    // propagates any AnkiConnect error.)
    for note in &processed_notes {
        if note.is_new {
            continue;
        }
        if let Some(id) = ctx.cache.get_note_id(&note.metadata.label) {
            ctx.cache
                .update_from_note(&note.metadata, id, note.field_hashes.clone())
                .await?;
        }
    }

    // Any processed note that was neither added nor updated was unchanged.
    result.notes_unchanged = processed_notes
        .len()
        .saturating_sub(result.notes_added)
        .saturating_sub(result.notes_updated);

    if ctx.config.cli_mode {
        info!(
            "Sync completed: {} added, {} updated, {} unchanged",
            result.notes_added, result.notes_updated, result.notes_unchanged
        );
    }

    Ok(())
}

/// Check if AnkiConnect is running and accessible.
async fn check_ankiconnect(anki_client: &AnkiConnect, http_client: &Client) -> Result<()> {
    let request = anki_client.action_to_request(AnkiAction::Version);

    let response = http_client
        .post(anki_client.url())
        .json(&request)
        .send()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to connect to AnkiConnect: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::anki_connect(format!(
            "AnkiConnect returned status: {}",
            response.status()
        )));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to read AnkiConnect response: {}", e)))?;

    let version: u32 = anki_client
        .parse_response(&response_text)
        .map_err(|e| Error::anki_connect(format!("Failed to parse version response: {}", e)))?;

    if version < 6 {
        return Err(Error::anki_connect(format!(
            "AnkiConnect version {} is too old, need at least version 6",
            version
        )));
    }

    Ok(())
}

/// Fetch a list of strings (deck names, model names, or tags) from AnkiConnect.
async fn fetch_string_list(
    anki_client: &AnkiConnect,
    http_client: &Client,
    action: AnkiAction,
) -> Result<Vec<String>> {
    let request = anki_client.action_to_request(action);
    let response = http_client
        .post(anki_client.url())
        .json(&request)
        .send()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to send request: {}", e)))?;
    let text = response
        .text()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to read response: {}", e)))?;
    anki_client
        .parse_response::<Vec<String>>(&text)
        .map_err(|e| Error::anki_connect(format!("Failed to parse response: {}", e)))
}

/// Validate each note's model, deck, and tags against what already exists in
/// Anki, per the document's `checks` configuration.
///
/// A missing model is fatal — Anki cannot create note types on the fly. Missing
/// decks and tags are reported only as warnings, since the sync creates decks
/// and Anki creates tags as notes are added.
async fn run_ankiconnect_checks(
    checks: &AnkiConnectChecks,
    notes: &[CompletedNote],
    anki_client: &AnkiConnect,
    http_client: &Client,
    result: &mut SyncResult,
) -> Result<()> {
    if checks.model.unwrap_or(true) {
        let models = fetch_string_list(anki_client, http_client, AnkiAction::ModelNames).await?;
        let models: HashSet<&str> = models.iter().map(String::as_str).collect();
        for note in notes {
            if !models.contains(note.model.as_str()) {
                return Err(Error::anki_connect(format!(
                    "note '{}' uses model '{}', which does not exist in Anki",
                    note.label, note.model
                )));
            }
        }
    }

    if checks.deck.unwrap_or(true) {
        let decks = fetch_string_list(anki_client, http_client, AnkiAction::DeckNames).await?;
        let decks: HashSet<&str> = decks.iter().map(String::as_str).collect();
        let missing: HashSet<&str> = notes
            .iter()
            .map(|n| n.deck.as_str())
            .filter(|d| !decks.contains(d))
            .collect();
        for deck in missing {
            result
                .warnings
                .push(format!("deck '{}' does not exist yet — it will be created", deck));
        }
    }

    if checks.tags.unwrap_or(true) {
        let tags = fetch_string_list(anki_client, http_client, AnkiAction::GetTags).await?;
        let tags: HashSet<&str> = tags.iter().map(String::as_str).collect();
        let missing: HashSet<&str> = notes
            .iter()
            .flat_map(|n| n.tags.iter())
            .map(String::as_str)
            .filter(|t| !tags.contains(t))
            .collect();
        for tag in missing {
            result
                .warnings
                .push(format!("tag '{}' does not exist yet — it will be created", tag));
        }
    }

    Ok(())
}

/// Process notes and create field hashes for comparison.
async fn process_notes_with_hashes(
    metadata_notes: &[CompletedNote],
    anki_notes: &[AnkiNote],
    cache: &Cache,
) -> Result<Vec<ProcessedNote>> {
    // Create futures for all note processing operations
    let futures: Vec<_> = metadata_notes
        .iter()
        .zip(anki_notes.iter())
        .map(|(metadata_note, anki_note)| async move {
            // Create field hashes using the cache's hashing method which properly handles media vs text
            let field_hashes = cache.create_field_hashes(anki_note, metadata_note).await?;

            // Check if this is a new note or an update
            let existing_entry = cache.get(&metadata_note.label);
            let is_new = existing_entry.is_none();

            Ok::<ProcessedNote, Error>(ProcessedNote {
                metadata: (*metadata_note).clone(),
                anki_note: (*anki_note).clone(),
                field_hashes,
                is_new,
            })
        })
        .collect();

    // Process all notes in parallel
    let results = futures::future::join_all(futures).await;

    // Collect results, propagating any errors
    let mut processed_notes = Vec::new();
    for result in results {
        processed_notes.push(result?);
    }

    Ok(processed_notes)
}

/// Determine whether an existing note differs from its cached state and thus
/// needs an `updateNote` request.
///
/// Note: a note's deck cannot be changed via `updateNote` (AnkiConnect only
/// moves cards between decks via `changeDeck`), so a deck change is not treated
/// as an update here.
fn note_changed(note: &ProcessedNote, cached: &CacheEntry) -> bool {
    // A field's content changed, or a field was added.
    for (field, new_hash) in &note.field_hashes {
        if cached.hash.get(field) != Some(new_hash) {
            return true;
        }
    }
    // A field was removed.
    for field in cached.hash.keys() {
        if !note.field_hashes.contains_key(field) {
            return true;
        }
    }
    // Tags changed.
    if note.anki_note.tags.clone().unwrap_or_default() != cached.tags {
        return true;
    }
    false
}

/// Create the request list for AnkiConnect operations.
async fn create_request_list(
    processed_notes: &[ProcessedNote],
    cache: &Cache,
    anki_client: &AnkiConnect,
) -> Result<RequestList> {
    let mut requests = Vec::new();

    // Collect all decks that need to be created
    let mut decks_to_create = HashSet::new();
    let mut existing_decks = HashSet::new();

    // First, collect existing decks from cache
    for entry in cache.entries().values() {
        existing_decks.insert(entry.deck.as_str().to_string());
    }

    // Check which decks need to be created
    for note in processed_notes {
        if note.is_new {
            let deck_name = note.anki_note.deck_name.as_str().to_string();
            if !existing_decks.contains(&deck_name) {
                decks_to_create.insert(deck_name);
            }
        }
    }

    // Create deck creation requests if needed
    if !decks_to_create.is_empty() {
        let mut deck_requests = Vec::new();
        for deck in decks_to_create {
            let request = anki_client.action_to_request(AnkiAction::CreateDeck { deck });
            deck_requests.push(RequestOrRequestList::Single(request));
        }

        if deck_requests.len() == 1 {
            requests.extend(deck_requests);
        } else {
            requests.push(RequestOrRequestList::List(RequestList {
                multi: true,
                requests: deck_requests,
            }));
        }
    }

    // Filter notes to only include truly new ones (not in cache)
    let truly_new_notes: Vec<_> = processed_notes
        .iter()
        .filter(|note| !cache.contains(&note.metadata.label))
        .collect();

    // Create addNotes request for truly new notes only
    if !truly_new_notes.is_empty() {
        let notes: Vec<AnkiNote> = truly_new_notes
            .iter()
            .map(|pn| pn.anki_note.clone())
            .collect();
        let request = anki_client.action_to_request(AnkiAction::AddNotes { notes });
        requests.push(RequestOrRequestList::Single(request));
    }

    // Create update requests for existing notes that have changed
    let notes_to_update: Vec<_> = processed_notes
        .iter()
        .filter(|note| !note.is_new)
        .filter(|note| match cache.get(&note.metadata.label) {
            Some(cached_entry) => note_changed(note, cached_entry),
            None => true, // Should not happen (filtered by !is_new), treat as changed
        })
        .collect();

    // Create updateNote requests for changed notes
    if !notes_to_update.is_empty() {
        let mut update_requests = Vec::new();
        for note in notes_to_update {
            if let Some(cached_entry) = cache.get(&note.metadata.label) {
                let request = anki_client.action_to_request(AnkiAction::UpdateNote {
                    note: crate::ankiconnect::NoteUpdate {
                        id: cached_entry.id.clone(),
                        fields: Some(note.anki_note.fields.clone()),
                        tags: note.anki_note.tags.clone(),
                        model_name: Some(note.anki_note.model_name.as_str().to_string()),
                        audio: note.anki_note.audio.clone(),
                        video: note.anki_note.video.clone(),
                        picture: note.anki_note.picture.clone(),
                    },
                });
                update_requests.push(RequestOrRequestList::Single(request));
            }
        }

        if !update_requests.is_empty() {
            if update_requests.len() == 1 {
                requests.extend(update_requests);
            } else {
                requests.push(RequestOrRequestList::List(RequestList {
                    multi: true,
                    requests: update_requests,
                }));
            }
        }
    }

    Ok(RequestList {
        multi: requests.len() > 1,
        requests,
    })
}

/// Execute the request list against AnkiConnect.
async fn execute_requests(
    request_list: &RequestList,
    anki_client: &AnkiConnect,
    http_client: &Client,
    cache: &mut Cache,
    result: &mut SyncResult,
    processed_notes: &[ProcessedNote],
) -> Result<()> {
    // Log the request list (visible with `RUST_LOG=debug`).
    if let Ok(pretty) = serde_json::to_string_pretty(request_list) {
        debug!("Request list:\n{}", pretty);
    }

    // Execute requests sequentially to ensure proper ordering
    // (deck creation before note addition, etc.)

    for request_or_list in &request_list.requests {
        match request_or_list {
            RequestOrRequestList::Single(request) => {
                let note_ids =
                    execute_single_request(request, anki_client, http_client, cache, result)
                        .await?;

                // Update cache for successfully added notes
                if let Some(note_ids) = note_ids {
                    update_cache_with_added_notes(cache, &note_ids, processed_notes, result)
                        .await?;
                }
            }
            RequestOrRequestList::List(nested_list) => {
                Box::pin(execute_requests(
                    nested_list,
                    anki_client,
                    http_client,
                    cache,
                    result,
                    processed_notes,
                ))
                .await?;
            }
        }
    }

    Ok(())
}

/// Execute a single request against AnkiConnect.
async fn execute_single_request(
    request: &serde_json::Value,
    anki_client: &AnkiConnect,
    http_client: &Client,
    _cache: &mut Cache,
    result: &mut SyncResult,
) -> Result<Option<Vec<Option<u64>>>> {
    let response = http_client
        .post(anki_client.url())
        .json(request)
        .send()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to send request: {}", e)))?;

    if !response.status().is_success() {
        return Err(Error::anki_connect(format!(
            "AnkiConnect returned status: {}",
            response.status()
        )));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| Error::anki_connect(format!("Failed to read response: {}", e)))?;

    // Parse the response to check for errors
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| Error::anki_connect(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        if !error.is_null() {
            return Err(Error::anki_connect(format!("AnkiConnect error: {}", error)));
        }
    }

    // Determine the type of request and update result accordingly
    let mut returned_note_ids = None;

    if let Some(action) = request.get("action").and_then(|a| a.as_str()) {
        match action {
            "createDeck" => {
                result.decks_created += 1;
            }
            "addNotes" => {
                // AnkiConnect's `addNotes` returns one slot per submitted note:
                // the new note's ID, or `null` for a note it could not add. The
                // `null`s must be preserved so every ID stays aligned with the
                // note it belongs to.
                if let Some(note_ids) = response_json.get("result").and_then(|r| r.as_array()) {
                    let ids: Vec<Option<u64>> = note_ids.iter().map(|v| v.as_u64()).collect();
                    result.notes_added += ids.iter().flatten().count();
                    returned_note_ids = Some(ids);
                }
            }
            "updateNote" => {
                result.notes_updated += 1;
            }
            _ => {
                // Other actions don't affect our counts
            }
        }
    }

    Ok(returned_note_ids)
}

/// Update the cache with the IDs `addNotes` returned for newly added notes.
///
/// `note_ids` is positionally aligned with the notes submitted in the
/// `addNotes` request: a `None` entry marks a note AnkiConnect declined to add,
/// which is skipped (and surfaced as a warning) rather than cached against a
/// neighbouring note's ID.
async fn update_cache_with_added_notes(
    cache: &mut Cache,
    note_ids: &[Option<u64>],
    processed_notes: &[ProcessedNote],
    result: &mut SyncResult,
) -> Result<()> {
    // The notes submitted in the `addNotes` request, in submission order — the
    // same filter `create_request_list` used to build that request.
    let new_notes: Vec<_> = processed_notes
        .iter()
        .filter(|note| !cache.contains(&note.metadata.label))
        .collect();

    if note_ids.len() != new_notes.len() {
        result.warnings.push(format!(
            "AnkiConnect returned {} note IDs for {} submitted notes; \
             some notes may not have been cached",
            note_ids.len(),
            new_notes.len()
        ));
    }

    for (i, maybe_id) in note_ids.iter().enumerate() {
        let Some(processed_note) = new_notes.get(i) else {
            continue;
        };
        match maybe_id {
            Some(id) => {
                cache
                    .update_from_note(
                        &processed_note.metadata,
                        NoteId(*id),
                        processed_note.field_hashes.clone(),
                    )
                    .await?;
            }
            None => {
                let msg = format!(
                    "Anki could not add note '{}' — skipped",
                    processed_note.metadata.label
                );
                warn!("{}", msg);
                result.warnings.push(msg);
            }
        }
    }

    Ok(())
}
