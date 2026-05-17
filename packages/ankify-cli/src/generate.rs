//! This module is responsible for generating temporary Typst files that will
//! then be compiled by the `compile` module.
//!
//! The generated file imports the user's source document and the `ankify`
//! Typst package, then renders every note field — one per page — so that the
//! `compile` module can turn each page into an image.
//!
//! The crucial invariant is *page ordering*: page `N` must contain field `N`
//! (counting notes in document order, and fields within a note in alphabetical
//! order). The `compile` module relies on this 1:1 mapping.
//!
//! To uphold that invariant, the note fields are rendered *first* (pages
//! `1..=N`), and only afterwards is the source document itself re-run — hidden,
//! on trailing pages that the CLI ignores. Re-running the source is what makes
//! the `note()`/`configure()` calls register their state; reading `.final()`
//! before those trailing pages still observes every update, because `.final()`
//! always yields the end-of-document value regardless of where it is read.
//!
//! Rendering the source last (instead of first, hidden, as an earlier version
//! did) matters because a real lecture document produces an unpredictable
//! number of pages — putting it first would shift every field's page number.
//!
//! The generated file looks like this:
//!
//! ```typst
//! #import "../source.typ" as __ankify-source-file
//! #import "@preview/ankify:0.1.0": __ankify-configuration, __ankify-notes
//!
//! #set page(width: 105mm, height: auto, margin: 5mm)
//!
//! #context {
//!   let config = __ankify-configuration.final()
//!   let notes = __ankify-notes.final()
//!   // ... flatten fields, then render one per page ...
//! }
//!
//! #pagebreak(weak: false)
//! #set page(width: 210mm, height: 297mm, margin: 20mm)
//! #hide([#__ankify-source-file])
//! ```
//!
//! Note that importing the source file from the parent directory requires the
//! compilation to use a `--root` flag covering both the temp file and the
//! source. The `__ankify-source-file` import path is relative to the temp
//! file's location, so it depends on the source file's name and directory.

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
///
/// The template uses `<<SOURCE>>` and `<<VERSION>>` placeholders rather than
/// `format!` interpolation so that the (brace-heavy) Typst code can be written
/// literally, without escaping every `{` and `}`.
fn generate_typst_content(relative_source_path: &str) -> Result<String> {
    // Validate inputs
    if relative_source_path.is_empty() {
        return Err(Error::custom(
            "Relative source path cannot be empty".to_string(),
        ));
    }

    const TEMPLATE: &str = r#"#import "<<SOURCE>>" as __ankify-source-file
#import "@preview/ankify:<<VERSION>>": __ankify-configuration, __ankify-notes

// Default geometry for the rendered card images. The user's `setup` function
// (if any) is applied below and may override this.
#set page(width: 105mm, height: auto, margin: 5mm)

#context {
  let config = __ankify-configuration.final()
  let notes = __ankify-notes.final()

  let raw-setup = config.at("setup", default: none)
  let setup = if raw-setup == none { (body) => body } else { raw-setup }

  // Flatten every note's fields into a single ordered list. Notes keep their
  // document order; fields within a note are sorted by name. The CLI walks
  // fields in this exact same order, so list index I corresponds to page I+1.
  let items = ()
  for note in notes {
    for (field, value) in note.data.pairs().sorted() {
      let field-content = if type(value) == dictionary and "value" in value {
        value.value
      } else if type(value) in (content, str) {
        value
      } else {
        panic("Invalid type for note data field: " + field)
      }
      items.push((note: note, field: field, content: field-content))
    }
  }

  // Render one field per page, sizing each page snugly to its content so the
  // resulting card image has no superfluous whitespace. Content wider than
  // `max-width` wraps instead of producing an arbitrarily wide image; the whole
  // card is then enlarged by the configured `scale` factor.
  let max-width = 14cm
  let card-margin = 5mm
  let card-scale = config.at("scale", default: 1.5) * 100%
  setup({
    for (i, item) in items.enumerate() {
      let body = (item.note.render)(
        note: item.note,
        field: item.field,
        field-content: item.content,
      )
      let w = calc.min(measure(body).width, max-width)
      let card = scale(card-scale, reflow: true, block(width: w, body))
      set page(width: w * card-scale + 2 * card-margin, height: auto, margin: card-margin)
      card
      if i != items.len() - 1 {
        pagebreak(weak: false)
      }
    }
  })
}

// Re-run the source document so that its `note()`/`configure()` calls register
// their state. It is rendered hidden, on trailing pages that the CLI ignores
// (the CLI only consumes the first N pages, one per note field).
#pagebreak(weak: false)
#set page(width: 210mm, height: 297mm, margin: 20mm)
#hide([#__ankify-source-file])
"#;

    let content = TEMPLATE
        .replace("<<SOURCE>>", relative_source_path)
        .replace("<<VERSION>>", PLUGIN_VERSION);

    // In tests the `ankify` Typst package is not published to the registry, so
    // it is installed into a local package directory (pointed at by the
    // TYPST_PACKAGE_PATH env var) and the import is rewritten from the
    // `@preview` namespace to `@local`.
    let content = if std::env::var("ANKIFY_USE_LOCAL_IMPORTS").is_ok() {
        content.replace(
            &format!("@preview/ankify:{}", PLUGIN_VERSION),
            &format!("@local/ankify:{}", PLUGIN_VERSION),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The note fields must be rendered *before* the source document so that
    /// output page N maps to note field N. Rendering the source first would
    /// shift every field's page number by the (unpredictable) source page
    /// count — the exact bug this ordering guards against.
    #[test]
    fn template_renders_fields_before_source() {
        let content = generate_typst_content("../notes.typ").unwrap();
        let fields_pos = content
            .find("__ankify-notes.final()")
            .expect("field rendering should be present");
        let source_pos = content
            .find("#hide([#__ankify-source-file])")
            .expect("source rendering should be present");
        assert!(
            fields_pos < source_pos,
            "note fields must be rendered before the source document"
        );
    }

    #[test]
    fn template_substitutes_all_placeholders() {
        let content = generate_typst_content("../notes.typ").unwrap();
        assert!(content.contains("../notes.typ"));
        assert!(!content.contains("<<SOURCE>>"));
        assert!(!content.contains("<<VERSION>>"));
    }

    #[test]
    fn empty_source_path_is_rejected() {
        assert!(generate_typst_content("").is_err());
    }
}
