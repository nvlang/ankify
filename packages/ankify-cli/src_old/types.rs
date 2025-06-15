// See the file LICENSE for the full license governing this code.

//! Type definitions for the Ankify library.
//!
//! This module contains all the core data structures used throughout the library,
//! including cards, configuration options, and rendering formats.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents content for a card field with its hash and optional format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Content {
    /// The actual content string
    pub content: String,
    /// Hash of the content for change detection
    pub hash: String,
    /// Optional format override for this specific field
    pub format: Option<FieldFormat>,
}

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
    pub data: HashMap<String, Content>,

    /// The Anki deck to place this card in
    pub deck: String,

    /// Tags to apply to this card
    pub tags: Vec<String>,

    /// Additional metadata to pass through to AnkiConnect
    pub rest: HashMap<String, serde_json::Value>,

    /// Optional global rendering format for this card's content
    pub format: Option<FieldFormat>,

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
    /// Pass content through without rendering
    Plain,
}

impl Default for FieldFormat {
    fn default() -> Self {
        FieldFormat::Svg
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
    pub data: Option<HashMap<String, Content>>,

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

    /// Content hashes for change detection (field_name -> hash)
    pub hash: HashMap<String, String>,
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
    pub format: Option<FieldFormat>,
}

impl Card {
    /// Calculate content hashes for each field for change detection.
    ///
    /// Returns a map of field names to their content hashes.
    pub fn content_hashes(&self) -> HashMap<String, String> {
        self.data
            .iter()
            .map(|(field_name, content)| (field_name.clone(), content.hash.clone()))
            .collect()
    }

    /// Get the effective format for a specific field.
    ///
    /// First checks the field's individual format, then falls back to the card's
    /// global format, then to the default format.
    pub fn get_field_format(&self, field_name: &str) -> FieldFormat {
        self.data
            .get(field_name)
            .and_then(|content| content.format)
            .or(self.format)
            .unwrap_or_default()
    }

    /// Check if any field uses SVG or PNG format (requires media file handling)
    pub fn has_media_fields(&self) -> bool {
        self.data.iter().any(|(field_name, _content)| {
            let format = self.get_field_format(field_name);
            matches!(format, FieldFormat::Svg | FieldFormat::Png)
        })
    }

    /// Get all fields that use SVG or PNG format
    pub fn get_media_fields(&self) -> Vec<String> {
        self.data
            .iter()
            .filter_map(|(field_name, _)| {
                let format = self.get_field_format(field_name);
                if matches!(format, FieldFormat::Svg | FieldFormat::Png) {
                    Some(field_name.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl From<Card> for RenderedCard {
    fn from(card: Card) -> Self {
        // Convert Content to String (extracting just the content field)
        let data = card
            .data
            .iter()
            .map(|(k, v)| (k.clone(), v.content.clone()))
            .collect();

        Self {
            model: card.model,
            data,
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

    fn create_test_content(content: &str, format: Option<FieldFormat>) -> Content {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        Content {
            content: content.to_string(),
            hash,
            format,
        }
    }

    #[test]
    fn test_card_content_hashes() {
        let mut data = HashMap::new();
        data.insert("Front".to_string(), create_test_content("Question", None));
        data.insert(
            "Back".to_string(),
            create_test_content("Answer", Some(FieldFormat::Plain)),
        );

        let card = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data,
            deck: "Test".to_string(),
            tags: vec!["tag1".to_string()],
            rest: HashMap::new(),
            format: Some(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        };

        let hashes = card.content_hashes();
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains_key("Front"));
        assert!(hashes.contains_key("Back"));
    }

    #[test]
    fn test_card_get_field_format() {
        let mut data = HashMap::new();
        data.insert("Front".to_string(), create_test_content("Question", None));
        data.insert(
            "Back".to_string(),
            create_test_content("Answer", Some(FieldFormat::Plain)),
        );

        let card = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data,
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: Some(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        };

        // Field with specific format should use that format
        assert_eq!(card.get_field_format("Back"), FieldFormat::Plain);

        // Field without specific format should use card's global format
        assert_eq!(card.get_field_format("Front"), FieldFormat::Svg);

        // Non-existent field should use default
        assert_eq!(card.get_field_format("NonExistent"), FieldFormat::Svg);
    }

    #[test]
    fn test_card_has_media_fields() {
        let mut data = HashMap::new();
        data.insert(
            "Front".to_string(),
            create_test_content("Question", Some(FieldFormat::Svg)),
        );
        data.insert(
            "Back".to_string(),
            create_test_content("Answer", Some(FieldFormat::Plain)),
        );

        let card = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data,
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: None,
            source_file: PathBuf::from("test.typ"),
        };

        assert!(card.has_media_fields());

        let media_fields = card.get_media_fields();
        assert_eq!(media_fields.len(), 1);
        assert!(media_fields.contains(&"Front".to_string()));
    }

    #[test]
    fn test_card_to_rendered_card() {
        let mut data = HashMap::new();
        data.insert("Front".to_string(), create_test_content("Question", None));
        data.insert("Back".to_string(), create_test_content("Answer", None));

        let card = Card {
            label: "test-card".to_string(),
            model: "Basic".to_string(),
            data,
            deck: "Test".to_string(),
            tags: vec!["tag1".to_string()],
            rest: HashMap::new(),
            format: Some(FieldFormat::Svg),
            source_file: PathBuf::from("test.typ"),
        };

        let rendered: RenderedCard = card.into();
        assert_eq!(rendered.model, "Basic");
        assert_eq!(rendered.data.get("Front").unwrap(), "Question");
        assert_eq!(rendered.data.get("Back").unwrap(), "Answer");
        assert_eq!(rendered.deck, "Test");
        assert_eq!(rendered.tags, vec!["tag1"]);
    }
}
