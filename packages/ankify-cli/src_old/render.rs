//! Card content rendering system
//!
//! This module provides functionality for rendering card content using Typst's
//! state and metadata capabilities. It generates single compilations that
//! produce multiple pages, one for each field of each card.

use crate::error::{Error, Result};
use crate::types::{Card, FieldFormat, RenderedCard};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, instrument};

// Typst library imports
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

// PDF to image conversion for PNG support
use typst_pdf::pdf;
use typst_svg::svg;

// Add chrono traits for date methods
use chrono::{Datelike, Timelike};

/// Simple World implementation for Typst rendering
#[derive(Debug)]
struct SimpleWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    files: HashMap<FileId, Source>,
    main_id: FileId,
}

impl SimpleWorld {
    fn new() -> Self {
        let library = LazyHash::new(Library::default());
        let book = LazyHash::new(FontBook::new());
        let fonts = Vec::new();
        let files = HashMap::new();
        let main_id = FileId::new(None, VirtualPath::new("main.typ"));

        Self {
            library,
            book,
            fonts,
            files,
            main_id,
        }
    }

    fn insert_source(&mut self, path: &str, content: &str) -> FileId {
        let id = FileId::new(None, VirtualPath::new(path));
        let source = Source::new(id, content.to_string());
        self.files.insert(id, source);
        self.main_id = id; // Set as main
        id
    }
}

impl World for SimpleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.get(&id).cloned().ok_or(FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = chrono::Utc::now();
        let offset = offset.unwrap_or(0);
        let adjusted = now + chrono::Duration::hours(offset);

        Datetime::from_ymd_hms(
            adjusted.year(),
            adjusted.month() as u8,
            adjusted.day() as u8,
            adjusted.hour() as u8,
            adjusted.minute() as u8,
            adjusted.second() as u8,
        )
    }
}

/// Card content renderer
///
/// Handles rendering of card content from Typst sources to various output formats.
/// Supports caching of rendered content and multiple output formats.
#[derive(Debug)]
pub struct Renderer {
    /// Cache directory for rendered assets
    #[allow(dead_code)]
    cache_dir: PathBuf,
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
    /// Returns an error if the cache directory cannot be created.
    #[instrument]
    pub async fn new(cache_dir: impl AsRef<Path> + std::fmt::Debug) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        let temp_dir = cache_dir.join("temp");

        // Create directories
        fs::create_dir_all(&cache_dir).await?;
        fs::create_dir_all(&temp_dir).await?;

        info!(?cache_dir, "Initialized renderer");

        Ok(Self {
            cache_dir,
            temp_dir,
        })
    }

    /// Render cards from a source file using the state-based approach
    ///
    /// This generates a single Typst compilation that produces multiple pages,
    /// one for each field of each card.
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path to the source Typst file containing cards
    /// * `cards` - Cards to render (used for determining field order)
    /// * `format` - Output format (SVG or PNG)
    ///
    /// # Returns
    ///
    /// Returns a vector of rendered cards with media files populated.
    #[instrument(skip(self, cards))]
    pub async fn render_cards_from_source(
        &self,
        source_file: &Path,
        cards: &[Card],
        format: FieldFormat,
    ) -> Result<Vec<RenderedCard>> {
        debug!(
            ?source_file,
            ?format,
            card_count = cards.len(),
            "Rendering cards from source"
        );

        match format {
            FieldFormat::Plain => {
                // For plain format, just return the content as-is without rendering
                self.render_cards_plain(cards).await
            }
            FieldFormat::Svg | FieldFormat::Png => {
                self.render_cards_with_typst(source_file, cards, format)
                    .await
            }
        }
    }

    /// Render cards in plain format (no Typst compilation)
    async fn render_cards_plain(&self, cards: &[Card]) -> Result<Vec<RenderedCard>> {
        let mut results = Vec::new();

        for card in cards {
            let mut rendered_data = HashMap::new();

            // Convert Content objects back to plain strings
            for (field_name, content) in &card.data {
                rendered_data.insert(field_name.clone(), content.content.clone());
            }

            results.push(RenderedCard {
                model: card.model.clone(),
                data: rendered_data,
                deck: card.deck.clone(),
                tags: card.tags.clone(),
                rest: card.rest.clone(),
                label: card.label.clone(),
                source_file: card.source_file.clone(),
                media_files: HashMap::new(),
            });
        }

        Ok(results)
    }

    /// Render cards using Typst compilation with state-based approach
    async fn render_cards_with_typst(
        &self,
        source_file: &Path,
        cards: &[Card],
        format: FieldFormat,
    ) -> Result<Vec<RenderedCard>> {
        // Create the render script
        let render_script = self.create_render_script(source_file)?;

        // Compile with Typst
        let document = self.compile_render_script(&render_script).await?;

        // Export based on format
        let rendered_pages = match format {
            FieldFormat::Svg => self.export_as_svg(&document).await?,
            FieldFormat::Png => self.export_as_png(&document).await?,
            FieldFormat::Plain => unreachable!("Plain format handled above"),
        };

        // Map rendered pages back to cards and fields
        self.map_pages_to_cards(cards, rendered_pages, format).await
    }

    /// Create the Typst render script that imports the source and renders all fields
    fn create_render_script(&self, source_file: &Path) -> Result<String> {
        let source_path = source_file.to_string_lossy();

        Ok(format!(
            r#"
#import "{source_path}"
#import "{source_path}": ankify-cards
#hide([#{source_path}])
#import "{source_path}": ankify-render
#set page(height: auto)
#context(
    for c in ankify-cards.final() {{
        for (field, value) in c.data {{
            ankify-render(c, field)
            pagebreak(weak: false)
        }}
    }}
)
"#,
            source_path = source_path
        ))
    }

    /// Compile the render script using Typst
    async fn compile_render_script(&self, script_content: &str) -> Result<PagedDocument> {
        let mut world = SimpleWorld::new();
        world.insert_source("render.typ", script_content);

        let result = typst::compile::<PagedDocument>(&world);
        match result.output {
            Ok(document) => Ok(document),
            Err(errors) => Err(Error::render(format!(
                "Typst compilation failed: {:?}",
                errors
            ))),
        }
    }

    /// Export document as SVG pages
    async fn export_as_svg(&self, document: &PagedDocument) -> Result<Vec<String>> {
        let mut pages = Vec::new();

        for page in &document.pages {
            let svg_data = svg(page);

            // Encode as base64
            let base64_data = general_purpose::STANDARD.encode(svg_data.as_bytes());
            pages.push(base64_data);
        }

        Ok(pages)
    }

    /// Export document as PNG pages
    async fn export_as_png(&self, document: &PagedDocument) -> Result<Vec<String>> {
        // First export to PDF, then convert pages to PNG
        let pdf_result = pdf(document, &Default::default());

        let pdf_data =
            pdf_result.map_err(|e| Error::render(format!("PDF export failed: {:?}", e)))?;

        // For now, we'll just return the PDF as base64 for each page
        // In a real implementation, you'd convert each PDF page to PNG
        let base64_pdf = general_purpose::STANDARD.encode(&pdf_data);

        // Return same PDF data for each page (placeholder)
        // TODO: Implement proper PDF to PNG conversion
        let mut pages = Vec::new();
        for _ in 0..document.pages.len() {
            pages.push(base64_pdf.clone());
        }

        Ok(pages)
    }

    /// Map rendered pages back to their corresponding cards and fields
    async fn map_pages_to_cards(
        &self,
        cards: &[Card],
        rendered_pages: Vec<String>,
        format: FieldFormat,
    ) -> Result<Vec<RenderedCard>> {
        let mut results = Vec::new();
        let mut page_idx = 0;

        for card in cards {
            let mut rendered_data = HashMap::new();
            let mut media_files = HashMap::new();

            // Process each field for this card
            for (field_name, content) in &card.data {
                let field_format = content
                    .format
                    .unwrap_or(card.format.unwrap_or(FieldFormat::Plain));

                if field_format == FieldFormat::Plain {
                    // Plain format: use content as-is
                    rendered_data.insert(field_name.clone(), content.content.clone());
                } else if page_idx < rendered_pages.len() {
                    // Use rendered page content
                    let timestamp = chrono::Utc::now().timestamp();
                    let filename = format!(
                        "{}-{}-{}.{}",
                        card.label,
                        field_name,
                        timestamp,
                        match format {
                            FieldFormat::Svg => "svg",
                            FieldFormat::Png => "png",
                            FieldFormat::Plain => unreachable!(),
                        }
                    );

                    // Store the base64 data as media file
                    media_files.insert(field_name.clone(), rendered_pages[page_idx].clone());

                    // For the field data, we'll use a reference to the media file
                    rendered_data.insert(field_name.clone(), format!("[media:{}]", filename));

                    page_idx += 1;
                } else {
                    return Err(Error::render(format!(
                        "Not enough rendered pages for card {} field {}",
                        card.label, field_name
                    )));
                }
            }

            results.push(RenderedCard {
                model: card.model.clone(),
                data: rendered_data,
                deck: card.deck.clone(),
                tags: card.tags.clone(),
                rest: card.rest.clone(),
                label: card.label.clone(),
                source_file: card.source_file.clone(),
                media_files,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Content;
    use crate::utils;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_renderer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let result = Renderer::new(temp_dir.path()).await;

        // Should always succeed since we're using the Typst library directly
        assert!(result.is_ok());
        let renderer = result.unwrap();
        assert!(renderer.cache_dir.exists());
        assert!(renderer.temp_dir.exists());
    }

    #[tokio::test]
    async fn test_render_cards_plain() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        let mut card_data = HashMap::new();
        card_data.insert(
            "Front".to_string(),
            Content {
                content: "What is 2+2?".to_string(),
                hash: utils::hash_string("What is 2+2?"),
                format: None,
            },
        );
        card_data.insert(
            "Back".to_string(),
            Content {
                content: "4".to_string(),
                hash: utils::hash_string("4"),
                format: None,
            },
        );

        let card = Card {
            label: "test-1".to_string(),
            model: "Basic".to_string(),
            data: card_data,
            deck: "Math".to_string(),
            tags: vec!["arithmetic".to_string()],
            rest: HashMap::new(),
            format: Some(FieldFormat::Plain),
            source_file: PathBuf::from("test.typ"),
        };

        let cards = vec![card];
        let results = renderer
            .render_cards_from_source(&PathBuf::from("test.typ"), &cards, FieldFormat::Plain)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let rendered = &results[0];
        assert_eq!(rendered.data["Front"], "What is 2+2?");
        assert_eq!(rendered.data["Back"], "4");
        assert_eq!(rendered.model, "Basic");
        assert_eq!(rendered.deck, "Math");
    }

    #[tokio::test]
    async fn test_create_render_script() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        let source_file = PathBuf::from("/path/to/source.typ");
        let script = renderer.create_render_script(&source_file).unwrap();

        // Should contain the expected template structure
        assert!(script.contains("#import \"/path/to/source.typ\""));
        assert!(script.contains("ankify-cards"));
        assert!(script.contains("ankify-render"));
        assert!(script.contains("pagebreak(weak: false)"));
        assert!(script.contains("for c in ankify-cards.final()"));
    }

    #[tokio::test]
    async fn test_render_script_compilation() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        // Simple Typst content that should compile
        let simple_script = "Hello, world!";

        let result = renderer.compile_render_script(simple_script).await;

        // Should succeed for basic content
        assert!(result.is_ok());
        let document = result.unwrap();
        assert!(!document.pages.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_cards_plain_rendering() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        let cards = vec![
            Card {
                label: "card1".to_string(),
                model: "Basic".to_string(),
                data: {
                    let mut data = HashMap::new();
                    data.insert(
                        "Front".to_string(),
                        Content {
                            content: "Q1".to_string(),
                            hash: utils::hash_string("Q1"),
                            format: None,
                        },
                    );
                    data.insert(
                        "Back".to_string(),
                        Content {
                            content: "A1".to_string(),
                            hash: utils::hash_string("A1"),
                            format: None,
                        },
                    );
                    data
                },
                deck: "Test".to_string(),
                tags: vec![],
                rest: HashMap::new(),
                format: Some(FieldFormat::Plain),
                source_file: PathBuf::from("test.typ"),
            },
            Card {
                label: "card2".to_string(),
                model: "Basic".to_string(),
                data: {
                    let mut data = HashMap::new();
                    data.insert(
                        "Front".to_string(),
                        Content {
                            content: "Q2".to_string(),
                            hash: utils::hash_string("Q2"),
                            format: None,
                        },
                    );
                    data.insert(
                        "Back".to_string(),
                        Content {
                            content: "A2".to_string(),
                            hash: utils::hash_string("A2"),
                            format: None,
                        },
                    );
                    data
                },
                deck: "Test".to_string(),
                tags: vec![],
                rest: HashMap::new(),
                format: Some(FieldFormat::Plain),
                source_file: PathBuf::from("test.typ"),
            },
        ];

        let results = renderer
            .render_cards_from_source(&PathBuf::from("test.typ"), &cards, FieldFormat::Plain)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].label, "card1");
        assert_eq!(results[1].label, "card2");
        assert_eq!(results[0].data["Front"], "Q1");
        assert_eq!(results[1].data["Front"], "Q2");
    }

    #[tokio::test]
    async fn test_svg_export() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        // Create a simple document
        let mut world = SimpleWorld::new();
        world.insert_source("main.typ", "Hello, SVG!");

        let document = typst::compile::<PagedDocument>(&world).output.unwrap();
        let svg_pages = renderer.export_as_svg(&document).await.unwrap();

        assert_eq!(svg_pages.len(), 1);

        // Should be valid base64
        let decoded = general_purpose::STANDARD.decode(&svg_pages[0]);
        assert!(decoded.is_ok());

        // Decoded content should be SVG-like
        let svg_content = String::from_utf8(decoded.unwrap()).unwrap();
        assert!(svg_content.contains("<svg"));
    }

    #[tokio::test]
    async fn test_png_export() {
        let temp_dir = TempDir::new().unwrap();
        let renderer = Renderer::new(temp_dir.path()).await.unwrap();

        // Create a simple document
        let mut world = SimpleWorld::new();
        world.insert_source("main.typ", "Hello, PNG!");

        let document = typst::compile::<PagedDocument>(&world).output.unwrap();
        let png_pages = renderer.export_as_png(&document).await.unwrap();

        assert_eq!(png_pages.len(), 1);

        // Should be valid base64
        let decoded = general_purpose::STANDARD.decode(&png_pages[0]);
        assert!(decoded.is_ok());

        // For now, this returns PDF data, so we just check it's valid base64
        // In a real implementation, this would be PNG data
    }
}
