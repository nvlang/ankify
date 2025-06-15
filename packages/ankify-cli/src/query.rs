//! Module dealing with querying Typst files for metadata.

use std::path::Path;

use crate::metadata::{Note, TypstAnkifyConfiguration};

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
) -> Result<Option<TypstAnkifyConfiguration>> {
    let values =
        query_typst_metadata_with_options(typst_file, "ankify-configuration", extra_args).await?;

    if values.is_empty() {
        return Ok(None);
    }

    // Take the first configuration found
    let config_value = &values[0];
    let config: TypstAnkifyConfiguration = serde_json::from_value(config_value.clone())
        .map_err(|e| Error::typst(format!("Failed to parse ankify configuration: {}", e)))?;

    Ok(Some(config))
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
    let values = query_typst_metadata_with_options(typst_file, "ankify-note", extra_args).await?;

    let mut notes = Vec::new();
    for value in values {
        let note: Note = serde_json::from_value(value)
            .map_err(|e| Error::typst(format!("Failed to parse ankify note: {}", e)))?;
        notes.push(note);
    }

    Ok(notes)
}

/// Query Typst for metadata with a specific label.
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
/// [{"ankiconnect-url":"http://localhost:8765","verbose":true,"defaults":{},"render":"ankify-render","cache":{"enabled":true,"custom-file":null},"checks":{"typst":{"data":true,"format":true},"ankiconnect":{"model":true,"deck":true,"tags":true}}}]
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
async fn query_typst_metadata(typst_file: &Path, label: &str) -> Result<Vec<serde_json::Value>> {
    query_typst_metadata_with_options(typst_file, label, None).await
}

/// Query Typst for metadata with a specific label and additional CLI options.
/// This allows tests to pass custom flags like --root.
async fn query_typst_metadata_with_options(
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
