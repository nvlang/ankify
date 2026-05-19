//! Module dealing with querying Typst files for metadata.

use std::path::Path;

use crate::compile::Format;
use crate::metadata::{
    AnkiConnectChecks, CacheOptions, Checks, CompletedNote, CompletedNoteDataValueWithFormat,
    CompletedTypstAnkifyConfiguration, Note, NoteDataValue, NoteDataValueWithFormat, NoteDefaults,
    TypstAnkifyConfiguration,
};

use crate::error::{Error, Result};
use tokio::process::Command;

/// Function that essentially executes the following shell command:
/// ```sh
/// typst query <file> "<ankify-configuration>"
/// ```
///
/// Should return `TypstAnkifyConfiguration` if successful
pub async fn query_ankify_configuration(
    typst_file: &Path,
    extra_args: Option<&[&str]>,
) -> Result<CompletedTypstAnkifyConfiguration> {
    let values = query_typst_metadata(typst_file, "ankify-configuration", extra_args).await?;

    // A document need not call `configure()` at all — fall back to defaults in
    // that case. If `configure()` is called more than once, the last call
    // reflects the most up-to-date (merged) configuration.
    let config: TypstAnkifyConfiguration = match values.last() {
        Some(config_value) => serde_json::from_value(config_value.clone())
            .map_err(|e| Error::typst(format!("Failed to parse ankify configuration: {}", e)))?,
        None => TypstAnkifyConfiguration {
            ankiconnect_url: None,
            verbose: None,
            setup: None,
            cache: None,
            checks: None,
            defaults: None,
        },
    };

    // Apply defaults to the configuration and convert to completed version
    Ok(apply_configuration_defaults(config))
}

/// Function that essentially executes the following shell command:
/// ```sh
/// typst query <file> "<ankify-note>"
/// ```
///
/// Should return `Vec<Note>` if successful
pub async fn query_ankify_notes(
    typst_file: &Path,
    extra_args: Option<&[&str]>,
) -> Result<Vec<Note>> {
    let values = query_typst_metadata(typst_file, "ankify-note", extra_args).await?;

    let mut notes = Vec::new();
    for value in values {
        let note: Note = serde_json::from_value(value)
            .map_err(|e| Error::typst(format!("Failed to parse ankify note: {}", e)))?;
        notes.push(note);
    }

    Ok(notes)
}

/// Helper function to apply defaults to a single note based on configuration
fn apply_defaults_to_note(note: &mut Note, config: &CompletedTypstAnkifyConfiguration) {
    // Apply string field defaults
    if note.model.is_empty() {
        note.model = config.defaults.model.clone().unwrap_or_default();
    }
    if note.deck.is_empty() {
        note.deck = config.defaults.deck.clone().unwrap_or_default();
    }
    if note.tags.is_empty() {
        note.tags = config.defaults.tags.clone().unwrap_or_default();
    }

    // Apply format default
    if note.format.is_none() {
        note.format = config
            .defaults
            .format
            .clone()
            .or_else(|| Some("svg".to_string()));
    }

    // Apply other field defaults
    if note.other.as_object().is_none_or(|obj| obj.is_empty()) {
        note.other = config.defaults.other.clone().unwrap_or_default();
    }
}

/// Helper function to normalize data fields to WithFormat variant
fn normalize_data_fields(note: &mut Note) {
    // Convert Simple values to WithFormat and ensure all have formats
    for (_key, value) in note.data.iter_mut() {
        match value {
            NoteDataValue::Simple(simple_val) => {
                *value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
                    value: simple_val.clone(),
                    format: note.format.clone(),
                });
            }
            NoteDataValue::WithFormat(with_format) => {
                if with_format.format.is_none() {
                    with_format.format = note.format.clone();
                }
            }
        }
    }
}

/// Helper function to convert Note to CompletedNote
fn note_to_completed_note(note: Note) -> CompletedNote {
    let default_format = note.format.unwrap_or_else(|| "svg".to_string());

    CompletedNote {
        label: note.label,
        model: note.model,
        deck: note.deck,
        tags: note.tags,
        other: note.other,
        format: default_format.clone(),
        data: note
            .data
            .into_iter()
            .map(|(key, value)| {
                let completed_value = match value {
                    NoteDataValue::Simple(val) => CompletedNoteDataValueWithFormat {
                        value: val,
                        format: default_format.clone(),
                    },
                    NoteDataValue::WithFormat(val_with_format) => {
                        CompletedNoteDataValueWithFormat {
                            value: val_with_format.value,
                            format: val_with_format
                                .format
                                .unwrap_or_else(|| default_format.clone()),
                        }
                    }
                };
                (key, completed_value)
            })
            .collect(),
    }
}

/// Complete ankify notes metadata by applying defaults and converting to CompletedNote
pub fn complete_ankify_notes_metadata(
    mut ankify_notes: Vec<Note>,
    config: &CompletedTypstAnkifyConfiguration,
) -> Vec<CompletedNote> {
    // Apply defaults and normalize each note
    for note in &mut ankify_notes {
        apply_defaults_to_note(note, config);
        normalize_data_fields(note);
    }

    // Convert to CompletedNote
    ankify_notes
        .into_iter()
        .map(note_to_completed_note)
        .collect()
}

/// Determine which formats need to be compiled based on (completed) note metadata.
pub fn determine_required_formats(completed_notes: &[CompletedNote]) -> Result<Vec<Format>> {
    let mut formats = std::collections::HashSet::new();

    for note in completed_notes {
        // Check field-specific formats if data has format specifications
        for value in note.data.values() {
            let format = Format::parse(value.format.as_str())?;
            // Skip plain format since it doesn't need compilation
            if format != Format::Plain {
                formats.insert(format);
            }
        }
    }

    Ok(formats.into_iter().collect())
}

/// Apply defaults to configuration fields and convert to CompletedTypstAnkifyConfiguration
fn apply_configuration_defaults(
    config: TypstAnkifyConfiguration,
) -> CompletedTypstAnkifyConfiguration {
    // Apply defaults for cache
    let cache = config.cache.unwrap_or(CacheOptions {
        enabled: Some(true),
        custom_file: None,
    });
    let completed_cache = CacheOptions {
        enabled: Some(cache.enabled.unwrap_or(true)),
        custom_file: cache.custom_file,
    };

    // Apply defaults for checks
    let checks = config.checks.unwrap_or(Checks {
        typst: Some(true),
        ankiconnect: Some(AnkiConnectChecks {
            model: Some(true),
            deck: Some(true),
            tags: Some(true),
        }),
    });
    let completed_checks = Checks {
        typst: Some(checks.typst.unwrap_or(true)),
        ankiconnect: Some({
            let ankiconnect = checks.ankiconnect.unwrap_or(AnkiConnectChecks {
                model: Some(true),
                deck: Some(true),
                tags: Some(true),
            });
            AnkiConnectChecks {
                model: Some(ankiconnect.model.unwrap_or(true)),
                deck: Some(ankiconnect.deck.unwrap_or(true)),
                tags: Some(ankiconnect.tags.unwrap_or(true)),
            }
        }),
    };

    // Apply defaults for note defaults
    let defaults = config.defaults.unwrap_or(NoteDefaults {
        model: Some("Basic".to_string()),
        deck: Some("Default".to_string()),
        tags: Some(Vec::new()),
        other: None,
        format: Some("svg".to_string()),
        render: None,
    });
    let completed_defaults = NoteDefaults {
        model: Some(defaults.model.unwrap_or("Basic".to_string())),
        deck: Some(defaults.deck.unwrap_or("Default".to_string())),
        tags: Some(defaults.tags.unwrap_or_default()),
        other: defaults.other,
        format: Some(defaults.format.unwrap_or("svg".to_string())),
        render: defaults.render,
    };

    CompletedTypstAnkifyConfiguration {
        ankiconnect_url: config
            .ankiconnect_url
            .unwrap_or("http://127.0.0.1:8765".to_string()),
        verbose: config.verbose.unwrap_or(false),
        setup: config.setup.unwrap_or(serde_json::Value::Null),
        cache: completed_cache,
        checks: completed_checks,
        defaults: completed_defaults,
    }
}

/// Query Typst for metadata with a specific label and additional CLI options.
/// This allows tests to pass custom flags like --root.
///
/// # Examples
///
/// ```sh
/// typst query example.typ "<ankify-configuration>" --field value
/// ```
///
/// might return this:
///
/// ```json
/// [{"ankiconnect-url":"http://127.0.0.1:8765","verbose":true,"defaults":{},"render":"ankify-render","cache":{"enabled":true,"custom-file":null},"checks":{"typst":{"data":true,"format":true},"ankiconnect":{"model":true,"deck":true,"tags":true}}}]
/// ```
///
/// Meanwhile,
///
/// ```sh
/// typst query example.typ "<ankify-note>" --field value
/// ```
///
/// might return this:
///
/// ```json
/// [{"label":"pythagoras-theorem","data":{"Front":"What is the Pythagorean theorem?","Back":{"func":"sequence","children":[{"func":"text","text":"test"},{"func":"space"},{"func":"equation","block":false,"body":{"func":"sequence","children":[{"func":"attach","base":{"func":"op","text":{"func":"text","text":"lim"},"limits":true},"b":{"func":"sequence","children":[{"func":"symbol","text":"n"},{"func":"space"},{"func":"symbol","text":"→"},{"func":"space"},{"func":"symbol","text":"∞"}]}},{"func":"space"},{"func":"frac","num":{"func":"symbol","text":"n"},"denom":{"func":"text","text":"2"}}]}},{"func":"space"},{"func":"symbol","text":"…"},{"func":"space"},{"func":"rect","height":"0% + 100pt","fill":"rgb(\"#0074d9\")"}]}},"model":"Basic","tags":["str"],"deck":"Ankify-Test","other":"dictionary","format":"png"},{"label":"quadratic-formula","data":{"Front":{"func":"text","text":"What is the quadratic formula?"},"Back":{"func":"sequence","children":[{"func":"text","text":"For"},{"func":"space"},{"func":"equation","block":false,"body":{"func":"sequence","children":[{"func":"symbol","text":"a"},{"func":"space"},{"func":"attach","base":{"func":"symbol","text":"x"},"t":{"func":"text","text":"2"}},{"func":"space"},{"func":"symbol","text":"+"},{"func":"space"},{"func":"symbol","text":"b"},{"func":"space"},{"func":"symbol","text":"x"},{"func":"space"},{"func":"symbol","text":"+"},{"func":"space"},{"func":"symbol","text":"c"},{"func":"space"},{"func":"symbol","text":"="},{"func":"space"},{"func":"text","text":"0"}]}},{"func":"text","text":":"},{"func":"space"},{"func":"equation","block":true,"body":{"func":"sequence","children":[{"func":"symbol","text":"x"},{"func":"space"},{"func":"symbol","text":"="},{"func":"space"},{"func":"frac","num":{"func":"text","text":"1"},"denom":{"func":"text","text":"2"}}]}}]}},"model":"Basic","tags":["str"],"deck":"Ankify-Test","other":"dictionary","format":"svg"}]
/// ```
pub async fn query_typst_metadata(
    typst_file: &Path,
    label: &str,
    extra_args: Option<&[&str]>,
) -> Result<Vec<serde_json::Value>> {
    // Always use the selector in the form <label> (e.g., <anki-card>)
    let selector = if label.starts_with('<') && label.ends_with('>') {
        label.to_string()
    } else {
        format!("<{}>", label)
    };

    // Build command arguments - start with basic query args
    let mut args = vec!["query"];

    // Add extra arguments first (they might include --root)
    if let Some(extra_args) = extra_args {
        args.extend(extra_args);
    }

    // Add the file and selector
    args.push(
        typst_file
            .to_str()
            .ok_or_else(|| Error::typst("Invalid file path encoding".to_string()))?,
    );
    args.push(&selector);
    args.push("--field");
    args.push("value");

    let output = Command::new("typst")
        .args(&args)
        .output()
        .await
        .map_err(|e| Error::typst(format!("Failed to execute typst command: {}", e)))?;

    if !output.status.success() {
        let _stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::typst(format!("Typst query failed: {}", _stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Parse JSON output
    let values: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| Error::typst(format!("Failed to parse Typst query output: {}", e)))?;

    Ok(values)
}
