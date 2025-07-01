//! This module is responsible for generating temporary Typst files that will
//! then be compiled by the `compile` module.
//!
//! The idea is essentially this: given `"source.typ"` (the hypothetical Typst
//! file being processed) and Typst plugin version `"0.1.0"`, this module should
//! generate the following temporary Typst file in a temporary `.ankify` directory:
//!
//! ```typst
//! #import "../source.typ"
//! #import "@preview/ankify:0.1.0": __ankify-configuration, __ankify-notes
//! #hide([#source])
//! #set page(width: 105mm, height: auto, margin: 5mm)
//!
//! #show: body => {
//!   context {
//!     let config = __ankify-configuration.final()
//!     if config.setup != none {
//!       (config.setup)(body)
//!     }
//!   }
//! }
//!
//! #context {
//!   let notes = __ankify-notes.final()
//!   let notes-len = notes.len()
//!   let current-note-index = 0
//!   for note in notes {
//!     let sorted-data = note.data.pairs().sorted()
//!     let fields-len = sorted-data.len()
//!     let current-field-index = 0
//!     for (field, value) in sorted-data {
//!       let field-content = none
//!       if (type(value) == dictionary and "value" in value) {
//!         field-content = value.value
//!       } else if (type(value) == content or type(value) == str) {
//!         field-content = value
//!       } else {
//!         panic("Invalid type for note data field", field)
//!       }
//!       (note.render)(note: note, field: field, field-content: field-content)
//!       if current-note-index != notes-len - 1 or current-field-index != fields-len - 1 {
//!           pagebreak(weak: false)
//!       }
//!       current-field-index += 1
//!     }
//!     current-note-index += 1
//!   }
//! }
//! ```
//!
//! Note that importing the source file from the parent directory will require
//! the compilation to have a `--root` flag set to the parent directory of the
//! source file. Furthermore, note that the `source` part in `#hide([#source])`
//! comes from the `source.typ` file name, so it will also have to be different
//! depending on the source file name.

use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const PLUGIN_VERSION: &str = "0.1.0";

/// Configuration for generating temporary Typst files.
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    /// The path to the source Typst file.
    pub source_file: PathBuf,
    /// The output directory for temporary files (optional, defaults to .ankify in source directory).
    pub output_dir: Option<PathBuf>,
}

/// Generate a temporary Typst file for rendering note fields.
///
/// This function creates a temporary Typst file that imports the source file
/// and uses the ankify plugin to extract and render note fields.
///
/// # Arguments
///
/// * `config` - Configuration for file generation
///
/// # Returns
///
/// Returns the path to the generated temporary file.
///
/// # Errors
///
/// Returns an error if:
/// - The source file doesn't exist
/// - The output directory cannot be created
/// - The temporary file cannot be written
pub fn generate_temp_file(config: &GenerateConfig) -> Result<PathBuf> {
    // Validate source file exists
    if !config.source_file.exists() {
        return Err(Error::custom(format!(
            "Source file does not exist: {}",
            config.source_file.display()
        )));
    }

    // Determine output directory
    let output_dir = match &config.output_dir {
        Some(dir) => dir.clone(),
        None => {
            let parent = config
                .source_file
                .parent()
                .ok_or_else(|| Error::custom("Source file has no parent directory".to_string()))?;
            parent.join(".ankify")
        }
    };

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).map_err(|e| {
            Error::custom(format!(
                "Failed to create output directory {}: {}",
                output_dir.display(),
                e
            ))
        })?;
    }

    // Generate the source file stem for the import
    let source_stem = config
        .source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::custom("Invalid source file name".to_string()))?;

    // Generate the relative path from output directory to source file
    let relative_source_path = get_relative_path(&output_dir, &config.source_file)?;

    // Generate temporary file path
    let temp_file_path = output_dir.join(format!("{}_render.typ", source_stem));

    // Generate the Typst content
    let typst_content = generate_typst_content(&relative_source_path)?;

    // Write the temporary file
    fs::write(&temp_file_path, typst_content).map_err(|e| {
        Error::custom(format!(
            "Failed to write temporary file {}: {}",
            temp_file_path.display(),
            e
        ))
    })?;

    Ok(temp_file_path)
}

/// Generate the Typst content for the temporary file.
fn generate_typst_content(relative_source_path: &str) -> Result<String> {
    // Validate inputs
    if relative_source_path.is_empty() {
        return Err(Error::custom(
            "Relative source path cannot be empty".to_string(),
        ));
    }

    let content = format!(
        r#"#import "{relative_source_path}" as __ankify-source-file
#import "@preview/ankify:{PLUGIN_VERSION}": __ankify-configuration, __ankify-notes
#hide([#__ankify-source-file])
#set page(width: 105mm, height: auto, margin: 5mm)

#show: body => {{
  context {{
    let config = __ankify-configuration.final()
    if config.setup != none {{
      (config.setup)(body)
    }}
  }}
}}

#context {{
  (__ankify-configuration.final().setup)()
  let notes = __ankify-notes.final()
  let notes-len = notes.len()
  let current-note-index = 0
  for note in notes {{
    let sorted-data = note.data.pairs().sorted()
    let fields-len = sorted-data.len()
    let current-field-index = 0
    for (field, value) in sorted-data {{
      let field-content = none
      if (type(value) == dictionary and "value" in value) {{
        field-content = value.value
      }} else if (type(value) == content or type(value) == str) {{
        field-content = value
      }} else {{
        panic("Invalid type for note data field", field)
      }}
      (note.render)(note: note, field: field, field-content: field-content)
      if current-note-index != notes-len - 1 or current-field-index != fields-len - 1 {{
          pagebreak(weak: false)
      }}
      current-field-index += 1
    }}
    current-note-index += 1
  }}
}}
"#
    );

    // If we're in a test environment, we want to replace the import statement for ankify
    // with a local path to the plugin.
    let content = if std::env::var("ANKIFY_USE_LOCAL_IMPORTS").is_ok() {
        content.replace(
            &format!("@preview/ankify:{}", PLUGIN_VERSION),
            "ankify-typst/lib.typ",
        )
    } else {
        content
    };

    Ok(content)
}

/// Get the relative path from one directory to another file.
fn get_relative_path(from_dir: &Path, to_file: &Path) -> Result<String> {
    // Convert to absolute paths to handle edge cases
    let from_abs = from_dir.canonicalize().map_err(|e| {
        Error::custom(format!(
            "Failed to canonicalize from directory {}: {}",
            from_dir.display(),
            e
        ))
    })?;

    let to_abs = to_file.canonicalize().map_err(|e| {
        Error::custom(format!(
            "Failed to canonicalize to file {}: {}",
            to_file.display(),
            e
        ))
    })?;

    // Calculate relative path
    let relative = pathdiff::diff_paths(&to_abs, &from_abs)
        .ok_or_else(|| Error::custom("Failed to calculate relative path".to_string()))?;

    // Convert to string and normalize path separators
    let relative_str = relative
        .to_str()
        .ok_or_else(|| Error::custom("Relative path contains invalid UTF-8".to_string()))?
        .replace('\\', "/"); // Normalize to forward slashes for Typst

    Ok(relative_str)
}

/// Clean up temporary files in the given directory.
///
/// This function removes all files matching the pattern `*_render.typ` in the
/// specified directory.
///
/// # Arguments
///
/// * `dir` - The directory to clean up
///
/// # Errors
///
/// Returns an error if the directory cannot be read or files cannot be removed.
pub fn cleanup_temp_files(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(()); // Nothing to clean up
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| Error::custom(format!("Failed to read directory {}: {}", dir.display(), e)))?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::custom(format!(
                "Failed to read directory entry in {}: {}",
                dir.display(),
                e
            ))
        })?;

        let path = entry.path();
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.ends_with("_render.typ") {
                fs::remove_file(&path).map_err(|e| {
                    Error::custom(format!(
                        "Failed to remove temporary file {}: {}",
                        path.display(),
                        e
                    ))
                })?;
            }
        }
    }

    Ok(())
}
