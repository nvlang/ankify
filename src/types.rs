// See the file LICENSE for the full license governing this code.

//! Type definitions for the Ankify library.
//!
//! This module contains all the core data structures used throughout the library,
//! including cards, configuration options, and rendering formats.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a single flashcard with all its metadata.
///
/// This is the core data structure that represents a flashcard extracted from
/// a Typst document. It contains all the information needed to create or update
/// an Anki card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Card {
    /// Unique label for this card (used for tracking updates)
    pub label: String,

    /// The Anki model (note type) to use for this card
    pub model: String,

    /// Field data for the card (e.g., "Front", "Back", etc.)
    pub data: HashMap<String, String>,

    /// The Anki deck to place this card in
    pub deck: String,

    /// Tags to apply to this card
    pub tags: Vec<String>,

    /// Additional metadata to pass through to AnkiConnect
    pub rest: HashMap<String, serde_json::Value>,

    /// Rendering format for this card's content
    pub format: RenderFormat,

    /// Source file where this card was defined
    pub source_file: PathBuf,
}

/// Represents a card after its content has been rendered.
///
/// This is what gets sent to the AnkiConnect API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedCard {
    /// The Anki model (note type) to use for this card
    pub model: String,

    /// Rendered field data for the card
    pub data: HashMap<String, String>,

    /// The Anki deck to place this card in
    pub deck: String,

    /// Tags to apply to this card
    pub tags: Vec<String>,

    /// Additional metadata to pass through to AnkiConnect
    pub rest: HashMap<String, serde_json::Value>,

    /// Original card label for tracking
    pub label: String,

    /// Source file where this card was defined
    pub source_file: PathBuf,

    /// Media files associated with fields (field_name -> file_path)
    pub media_files: HashMap<String, String>,
}

/// Individual rendering format for a field.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldFormat {
    /// Render as SVG image using Typst
    Svg,
    /// Render as PNG image using Typst
    Png,
    /// Render as HTML using Typst
    Html,
    /// Pass content through without rendering
    Plain,
}

impl Default for FieldFormat {
    fn default() -> Self {
        FieldFormat::Svg
    }
}

/// Supported rendering formats for card content.
/// Can be either a single format applied to all fields or field-specific formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RenderFormat {
    /// Single format applied to all fields
    Single(FieldFormat),
    /// Field-specific formats
    PerField(HashMap<String, FieldFormat>),
}

impl Default for RenderFormat {
    fn default() -> Self {
        RenderFormat::Single(FieldFormat::Svg)
    }
}

impl RenderFormat {
    /// Get the format for a specific field
    pub fn get_format_for_field(&self, field_name: &str) -> FieldFormat {
        match self {
            RenderFormat::Single(format) => *format,
            RenderFormat::PerField(formats) => formats.get(field_name).copied().unwrap_or_default(),
        }
    }

    /// Check if any field uses SVG or PNG format (requires media file handling)
    pub fn has_media_fields(&self) -> bool {
        match self {
            RenderFormat::Single(format) => matches!(format, FieldFormat::Svg | FieldFormat::Png),
            RenderFormat::PerField(formats) => formats
                .values()
                .any(|format| matches!(format, FieldFormat::Svg | FieldFormat::Png)),
        }
    }

    /// Get all fields that use SVG or PNG format
    pub fn get_media_fields(&self, field_names: &[String]) -> Vec<String> {
        match self {
            RenderFormat::Single(format) => {
                if matches!(format, FieldFormat::Svg | FieldFormat::Png) {
                    field_names.to_vec()
                } else {
                    Vec::new()
                }
            }
            RenderFormat::PerField(formats) => formats
                .iter()
                .filter_map(|(field, format)| {
                    if matches!(format, FieldFormat::Svg | FieldFormat::Png) {
                        Some(field.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

/// Default values that can be applied to cards.
///
/// These are used to fill in missing values in card objects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardDefaults {
    /// Default model/note type
    pub model: Option<String>,

    /// Default deck
    pub deck: Option<String>,

    /// Default tags
    pub tags: Option<Vec<String>>,

    /// Default field data
    pub data: Option<HashMap<String, String>>,

    /// Default additional metadata
    pub rest: Option<HashMap<String, serde_json::Value>>,
}

impl Default for CardDefaults {
    fn default() -> Self {
        Self {
            model: Some("Basic".to_string()),
            deck: Some("Default".to_string()),
            tags: Some(Vec::new()),
            data: Some(HashMap::new()),
            rest: Some(HashMap::new()),
        }
    }
}

/// Configuration extracted from Typst documents.
///
/// This represents configuration options that can be set within Typst documents
/// using the `configure` function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypstConfig {
    /// AnkiConnect URL
    pub ankiconnect_url: Option<String>,

    /// Verbose output flag
    pub verbose: Option<bool>,

    /// Auxiliary file path
    pub aux_file: Option<PathBuf>,

    /// Default values for cards
    pub defaults: Option<CardDefaults>,

    /// Custom render function path
    pub render: Option<PathBuf>,
}

impl Default for TypstConfig {
    fn default() -> Self {
        Self {
            ankiconnect_url: None,
            verbose: None,
            aux_file: None,
            defaults: None,
            render: None,
        }
    }
}

/// An entry in the auxiliary cache file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEntry {
    /// Card label
    pub label: String,

    /// AnkiConnect card ID
    pub id: String,

    /// Content hash for change detection
    pub hash: String,

    /// Timestamp of last update
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Raw card data as extracted from Typst metadata.
///
/// This is the intermediate format before cards are processed and validated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCard {
    /// Card label
    pub label: String,

    /// Model/note type
    pub model: Option<String>,

    /// Field data
    pub data: Option<HashMap<String, String>>,

    /// Deck name
    pub deck: Option<String>,

    /// Tags
    pub tags: Option<Vec<String>>,

    /// Additional metadata
    pub rest: Option<HashMap<String, serde_json::Value>>,

    /// Rendering format
    pub format: Option<RenderFormat>,
}

impl Card {
    /// Calculate a hash of this card's content for change detection.
    ///
    /// The hash includes all content that affects the rendered card but excludes
    /// the label (which is used for identification) and source file (which doesn't
    /// affect the card content).
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Include model
        hasher.update(self.model.as_bytes());

        // Include data fields in a deterministic order
        let mut data_items: Vec<_> = self.data.iter().collect();
        data_items.sort_by_key(|(k, _)| *k);
        for (key, value) in data_items {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }

        // Include deck
        hasher.update(self.deck.as_bytes());

        // Include tags in sorted order
        let mut tags = self.tags.clone();
        tags.sort();
        for tag in tags {
            hasher.update(tag.as_bytes());
        }

        // Include rest metadata
        if let Ok(rest_json) = serde_json::to_string(&self.rest) {
            hasher.update(rest_json.as_bytes());
        }

        // Include format
        if let Ok(format_json) = serde_json::to_string(&self.format) {
            hasher.update(format_json.as_bytes());
        }

        hex::encode(hasher.finalize())
    }
}

impl From<Card> for RenderedCard {
    fn from(card: Card) -> Self {
        Self {
            model: card.model,
            data: card.data,
            deck: card.deck,
            tags: card.tags,
            rest: card.rest,
            label: card.label,
            source_file: card.source_file,
            media_files: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_content_hash_consistency() {
        let card1 = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data: [
                ("Front".to_string(), "Question".to_string()),
                ("Back".to_string(), "Answer".to_string()),
            ]
            .into(),
            deck: "Test".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        };

        let card2 = Card {
            label: "different-label".to_string(), // Different label shouldn't affect hash
            model: "Basic".to_string(),
            data: [
                ("Back".to_string(), "Answer".to_string()),
                ("Front".to_string(), "Question".to_string()),
            ]
            .into(), // Different order shouldn't affect hash
            deck: "Test".to_string(),
            tags: vec!["tag2".to_string(), "tag1".to_string()], // Different order shouldn't affect hash
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("different.typ"), // Different source file shouldn't affect hash
        };

        assert_eq!(card1.content_hash(), card2.content_hash());
    }

    #[test]
    fn test_card_content_hash_changes() {
        let mut card = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data: [("Front".to_string(), "Question".to_string())].into(),
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        };

        let original_hash = card.content_hash();

        // Changing content should change hash
        card.data.insert("Back".to_string(), "Answer".to_string());
        assert_ne!(card.content_hash(), original_hash);
    }
}
