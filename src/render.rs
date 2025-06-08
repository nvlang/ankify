//! Card content rendering system
//!
//! This module provides functionality for rendering card content in different formats
//! including SVG, PNG, HTML, and plain text. It handles Typst compilation and
//! format conversion with proper error handling and resource management.

use crate::error::{Error, Result};
use crate::types::{Card, FieldFormat, RenderedCard};
#[cfg(test)]
use crate::types::RenderFormat;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::{debug, info, instrument, warn};
use base64::{engine::general_purpose, Engine as _};

/// Card content renderer
///
/// Handles rendering of card content from Typst sources to various output formats.
/// Supports caching of rendered content and multiple output formats.
#[derive(Debug)]
pub struct Renderer {
    /// Cache directory for rendered assets
    #[allow(dead_code)]
    cache_dir: PathBuf,
    /// Typst compiler path
    typst_binary: PathBuf,
    /// Temporary directory for rendering operations
    temp_dir: PathBuf,
}

impl Renderer {
    /// Create a new renderer with the specified cache directory
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory to store cached rendered content
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or if the Typst
    /// binary cannot be found in the system PATH.
    #[instrument]
    pub async fn new(cache_dir: impl AsRef<Path> + std::fmt::Debug) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        let typst_binary = Self::find_typst_binary()?;
        let temp_dir = cache_dir.join("temp");

        // Create directories
        fs::create_dir_all(&cache_dir).await?;
        fs::create_dir_all(&temp_dir).await?;

        info!(?cache_dir, ?typst_binary, "Initialized renderer");

        Ok(Self {
            cache_dir,
            typst_binary,
            temp_dir,
        })
    }

    /// Render a card field to the specified format
    ///
    /// # Arguments
    ///
    /// * `card` - The card to render
    /// * `field_name` - The specific field to render (e.g., "Front", "Back", "Text")
    /// * `format` - The target format for rendering
    /// * `render_function_path` - Optional path to custom Typst render function
    ///
    /// # Returns
    ///
    /// Returns the rendered content as a string, or path to temp file for SVG/PNG formats.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails, the format is unsupported, or I/O operations fail.
    #[instrument(skip(self))]
    pub async fn render_field(
        &self,
        card: &Card,
        field_name: &str,
        format: FieldFormat,
        render_function_path: Option<&Path>,
    ) -> Result<String> {
        debug!(label = %card.label, field = field_name, ?format, "Rendering card field");

        match format {
            FieldFormat::Svg => {
                self.render_field_to_svg(card, field_name, render_function_path)
                    .await
            }
            FieldFormat::Png => {
                self.render_field_to_png(card, field_name, render_function_path)
                    .await
            }
            FieldFormat::Html => {
                self.render_field_to_html(card, field_name, render_function_path)
                    .await
            }
            FieldFormat::Plain => self.render_field_to_plain(card, field_name),
        }
    }

    /// Render a complete card with all its fields
    ///
    /// # Arguments
    ///
    /// * `card` - The card to render
    /// * `render_function_path` - Optional path to custom Typst render function
    ///
    /// # Returns
    ///
    /// Returns a `RenderedCard` containing all rendered field content.
    #[instrument(skip(self))]
    pub async fn render_card(
        &self,
        card: &Card,
        render_function_path: Option<&Path>,
    ) -> Result<RenderedCard> {
        debug!(label = %card.label, "Rendering complete card");

        let mut rendered_data = HashMap::new();

        // Render each field in the card data
        for (field_name, _) in &card.data {
            let field_format = card.format.get_format_for_field(field_name);
            let rendered_content = self
                .render_field(card, field_name, field_format, render_function_path)
                .await?;
            rendered_data.insert(field_name.clone(), rendered_content);
        }

        Ok(RenderedCard {
            model: card.model.clone(),
            data: rendered_data,
            deck: card.deck.clone(),
            tags: card.tags.clone(),
            rest: card.rest.clone(),
            label: card.label.clone(),
            source_file: card.source_file.clone(),
            media_files: HashMap::new(), // Will be populated by AnkiConnect integration
        })
    }

    /// Render multiple cards efficiently
    ///
    /// # Arguments
    ///
    /// * `cards` - Iterator of cards to render
    /// * `render_function_path` - Optional path to custom Typst render function
    ///
    /// # Returns
    ///
    /// Returns a vector of rendered cards in the same order as the input.
    #[instrument(skip(self, cards))]
    pub async fn render_cards_batch<'a, I>(
        &self,
        cards: I,
        render_function_path: Option<&Path>,
    ) -> Result<Vec<RenderedCard>>
    where
        I: Iterator<Item = &'a Card>,
    {
        let cards: Vec<_> = cards.collect();
        debug!(count = cards.len(), "Batch rendering cards");

        let mut results = Vec::with_capacity(cards.len());
        for card in cards {
            results.push(self.render_card(card, render_function_path).await?);
        }

        Ok(results)
    }

    /// Render a card field to SVG format
    async fn render_field_to_svg(
        &self,
        card: &Card,
        field_name: &str,
        render_function_path: Option<&Path>,
    ) -> Result<String> {
        if render_function_path.is_none() {
            // Default behavior: return the field content as-is
            return Ok(card.data.get(field_name).unwrap_or(&String::new()).clone());
        }

        let temp_file = self
            .create_temp_typst_file(card, field_name, render_function_path.unwrap())
            .await?;
        
        // Create output file with unique name based on card label and field
        let output_filename = format!("{}-{}.svg", card.label, field_name);
        let output_file = self.temp_dir.join(output_filename);

        let output = Command::new(&self.typst_binary)
            .args(["compile", "--format", "svg"])
            .arg(&temp_file)
            .arg(&output_file)
            .output()
            .map_err(|e| Error::render(format!("Failed to execute typst command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::RenderError(format!(
                "SVG compilation failed: {}",
                stderr
            )));
        }

        // Clean up temporary typst file but keep the SVG output file
        self.cleanup_temp_files(&[temp_file]).await?;

        // Read the SVG file and encode as base64
        let svg_content = fs::read(&output_file).await
            .map_err(|e| Error::render(format!("Failed to read SVG file: {}", e)))?;
        let base64_content = general_purpose::STANDARD.encode(&svg_content);

        // Clean up the SVG file
        self.cleanup_temp_files(&[output_file]).await?;

        // Return the base64 encoded content
        Ok(base64_content)
    }

    /// Render a card field to PNG format
    async fn render_field_to_png(
        &self,
        card: &Card,
        field_name: &str,
        render_function_path: Option<&Path>,
    ) -> Result<String> {
        if render_function_path.is_none() {
            // Default behavior: return the field content as-is
            return Ok(card.data.get(field_name).unwrap_or(&String::new()).clone());
        }

        let temp_file = self
            .create_temp_typst_file(card, field_name, render_function_path.unwrap())
            .await?;
        
        // Create output file with unique name based on card label and field
        let output_filename = format!("{}-{}.png", card.label, field_name);
        let output_file = self.temp_dir.join(output_filename);

        let output = Command::new(&self.typst_binary)
            .args(["compile", "--format", "png", "--ppi", "300"])
            .arg(&temp_file)
            .arg(&output_file)
            .output()
            .map_err(|e| Error::render(format!("Failed to execute typst command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::RenderError(format!(
                "PNG compilation failed: {}",
                stderr
            )));
        }

        // Clean up temporary typst file but keep the PNG output file
        self.cleanup_temp_files(&[temp_file]).await?;

        // Read the PNG file and encode as base64
        let png_content = fs::read(&output_file).await
            .map_err(|e| Error::render(format!("Failed to read PNG file: {}", e)))?;
        let base64_content = general_purpose::STANDARD.encode(&png_content);

        // Clean up the PNG file
        self.cleanup_temp_files(&[output_file]).await?;

        // Return the base64 encoded content
        Ok(base64_content)
    }

    /// Render a card field to HTML format
    async fn render_field_to_html(
        &self,
        card: &Card,
        field_name: &str,
        render_function_path: Option<&Path>,
    ) -> Result<String> {
        // First render to SVG, then embed in HTML
        let svg_content = self
            .render_field_to_svg(card, field_name, render_function_path)
            .await?;

        Ok(format!(
            r#"<div class="ankify-card-field" data-field="{}">{}</div>"#,
            field_name, svg_content
        ))
    }

    /// Render a card field to plain text format
    fn render_field_to_plain(&self, card: &Card, field_name: &str) -> Result<String> {
        Ok(card.data.get(field_name).unwrap_or(&String::new()).clone())
    }

    /// Create a temporary Typst file for rendering with custom render function
    async fn create_temp_typst_file(
        &self,
        card: &Card,
        field_name: &str,
        render_function_path: &Path,
    ) -> Result<PathBuf> {
        let temp_file = self
            .temp_dir
            .join(format!("card_{}_{}.typ", card.label, field_name));

        // Create Typst document that imports and calls the custom render function
        let content = format!(
            r#"
#import "{}": render

// Create card object matching the spec
#let cardObj = (
    label: "{}",
    model: "{}",
    data: {},
    deck: "{}",
    tags: {},
    rest: {},
    format: "{}",
    source_file: "{}"
)

// Call the render function with card object and field name
#render(cardObj, "{}")
"#,
            render_function_path.display(),
            card.label,
            card.model,
            serde_json::to_string(&card.data).unwrap_or_else(|_| "()".to_string()),
            card.deck,
            serde_json::to_string(&card.tags).unwrap_or_else(|_| "()".to_string()),
            serde_json::to_string(&card.rest).unwrap_or_else(|_| "()".to_string()),
            serde_json::to_string(&card.format).unwrap_or_else(|_| "\"svg\"".to_string()),
            card.source_file.display(),
            field_name
        );

        fs::write(&temp_file, content).await?;
        Ok(temp_file)
    }

    /// Clean up temporary files
    async fn cleanup_temp_files(&self, files: &[PathBuf]) -> Result<()> {
        for file in files {
            if file.exists() {
                if let Err(e) = fs::remove_file(file).await {
                    warn!(?file, error = %e, "Failed to cleanup temp file");
                }
            }
        }
        Ok(())
    }

    /// Find the Typst binary in the system PATH
    fn find_typst_binary() -> Result<PathBuf> {
        if let Ok(output) = std::process::Command::new("which").arg("typst").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }

        // Fallback to common locations
        let common_paths = [
            "/usr/local/bin/typst",
            "/opt/homebrew/bin/typst",
            "/usr/bin/typst",
        ];

        for path in &common_paths {
            let typst_path = PathBuf::from(path);
            if typst_path.exists() {
                return Ok(typst_path);
            }
        }

        Err(Error::MissingDependency(
            "Typst binary not found in PATH or common locations".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_renderer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let result = Renderer::new(temp_dir.path()).await;

        // This might fail if Typst is not installed, which is fine for testing
        match result {
            Ok(renderer) => {
                assert!(renderer.cache_dir.exists());
                assert!(renderer.temp_dir.exists());
            }
            Err(Error::MissingDependency(_)) => {
                // Expected if Typst is not installed
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_render_field_to_plain() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer {
            cache_dir: temp_dir.path().to_path_buf(),
            typst_binary: PathBuf::from("typst"), // Won't be used for plain text
            temp_dir: temp_dir.path().join("temp"),
        };

        let mut card_data = HashMap::new();
        card_data.insert("Front".to_string(), "What is 2+2?".to_string());
        card_data.insert("Back".to_string(), "4".to_string());

        let card = Card {
            label: "test-1".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Math".to_string(),
            tags: vec!["arithmetic".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Plain),
            source_file: PathBuf::from("test.typ"),
        };

        let front_rendered = renderer.render_field_to_plain(&card, "Front").unwrap();
        let back_rendered = renderer.render_field_to_plain(&card, "Back").unwrap();

        assert_eq!(front_rendered, "What is 2+2?");
        assert_eq!(back_rendered, "4");
    }

    #[test]
    fn test_find_typst_binary_fallback() {
        // This test checks that we handle the case when Typst is not found
        match Renderer::find_typst_binary() {
            Ok(path) => {
                assert!(path.exists() || path.to_string_lossy().contains("typst"));
            }
            Err(Error::MissingDependency(_)) => {
                // Expected when Typst is not installed
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_render_field_all_formats() {
        let temp_dir = TempDir::new().unwrap();

        // Create a renderer without Typst dependency for testing fallbacks
        let renderer = Renderer {
            cache_dir: temp_dir.path().to_path_buf(),
            typst_binary: PathBuf::from("typst"), // Won't be used for plain text
            temp_dir: temp_dir.path().join("temp"),
        };

        let mut card_data = HashMap::new();
        card_data.insert("Test".to_string(), "Content".to_string());

        let card = Card {
            label: "format-test".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Test".to_string(),
            tags: vec!["test".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Plain),
            source_file: PathBuf::from("test.typ"),
        };

        // Test all formats without render function (should use fallbacks)
        let plain_result = renderer
            .render_field(&card, "Test", FieldFormat::Plain, None)
            .await;
        assert!(plain_result.is_ok());
        assert_eq!(plain_result.unwrap(), "Content");

        let svg_result = renderer
            .render_field(&card, "Test", FieldFormat::Svg, None)
            .await;
        assert!(svg_result.is_ok());
        assert_eq!(svg_result.unwrap(), "Content");

        let png_result = renderer
            .render_field(&card, "Test", FieldFormat::Png, None)
            .await;
        assert!(png_result.is_ok());
        assert_eq!(png_result.unwrap(), "Content");

        let html_result = renderer
            .render_field(&card, "Test", FieldFormat::Html, None)
            .await;
        assert!(html_result.is_ok());
        let html = html_result.unwrap();
        assert!(html.contains("Content"));
        assert!(html.contains("ankify-card-field"));
        assert!(html.contains("data-field=\"Test\""));
    }

    #[test]
    fn test_render_field_missing_field() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer {
            cache_dir: temp_dir.path().to_path_buf(),
            typst_binary: PathBuf::from("typst"),
            temp_dir: temp_dir.path().join("temp"),
        };

        let card = Card {
            label: "test".to_string(),
            model: "Basic".to_string(),
            data: HashMap::new(), // Empty data
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Plain),
            source_file: PathBuf::from("test.typ"),
        };

        let result = renderer.render_field_to_plain(&card, "NonExistent");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ""); // Should return empty string for missing fields
    }

    #[tokio::test]
    async fn test_create_temp_typst_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("temp"))
            .await
            .unwrap();

        let renderer = Renderer {
            cache_dir: temp_dir.path().to_path_buf(),
            typst_binary: PathBuf::from("typst"),
            temp_dir: temp_dir.path().join("temp"),
        };

        let mut card_data = HashMap::new();
        card_data.insert("Front".to_string(), "Question".to_string());
        card_data.insert("Back".to_string(), "Answer".to_string());

        let mut rest_data = HashMap::new();
        rest_data.insert(
            "extra".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        let card = Card {
            label: "temp-test".to_string(),
            model: "BasicAndReversed".to_string(),
            data: card_data,
            deck: "Test Deck".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            rest: rest_data,
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("source.typ"),
        };

        let render_path = temp_dir.path().join("render.typ");
        fs::write(&render_path, "// Mock render function")
            .await
            .unwrap();

        let result = renderer
            .create_temp_typst_file(&card, "Front", &render_path)
            .await;
        assert!(result.is_ok());

        let temp_file = result.unwrap();
        assert!(temp_file.exists());

        let content = fs::read_to_string(&temp_file).await.unwrap();
        assert!(content.contains("temp-test"));
        assert!(content.contains("BasicAndReversed"));
        assert!(content.contains("Test Deck"));
        assert!(content.contains("Front"));
        assert!(content.contains("svg"));
    }

    #[tokio::test]
    async fn test_cleanup_temp_files() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer {
            cache_dir: temp_dir.path().to_path_buf(),
            typst_binary: PathBuf::from("typst"),
            temp_dir: temp_dir.path().join("temp"),
        };

        // Create temporary files
        let temp_files = vec![
            temp_dir.path().join("temp1.txt"),
            temp_dir.path().join("temp2.txt"),
            temp_dir.path().join("nonexistent.txt"), // This one doesn't exist
        ];

        // Create the first two files
        fs::write(&temp_files[0], "temp1").await.unwrap();
        fs::write(&temp_files[1], "temp2").await.unwrap();

        assert!(temp_files[0].exists());
        assert!(temp_files[1].exists());
        assert!(!temp_files[2].exists());

        // Test cleanup
        let result = renderer.cleanup_temp_files(&temp_files).await;
        assert!(result.is_ok());

        // Files should be deleted (or at least attempted)
        // Note: The cleanup function logs warnings for failed deletions but doesn't fail
    }
}
