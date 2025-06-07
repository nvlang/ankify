// See the file LICENSE for the full license governing this code.

//! Typst integration and card extraction.
//!
//! This module handles communication with Typst to extract card metadata
//! and configuration from Typst documents.

use crate::{types::*, Error, Result};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

/// Extract all cards from a Typst file.
///
/// This function queries the Typst document for card metadata and returns
/// a vector of validated Card objects.
///
/// # Arguments
///
/// * `typst_file` - Path to the Typst file to process
///
/// # Returns
///
/// A vector of Card objects extracted from the document.
///
/// # Errors
///
/// Returns an error if:
/// - The Typst file cannot be read or processed
/// - The Typst query fails
/// - The extracted metadata is invalid
pub async fn extract_cards(typst_file: &Path) -> Result<Vec<Card>> {
    tracing::debug!("Extracting cards from {}", typst_file.display());

    // Query Typst for card metadata
    let raw_cards = query_typst_metadata(typst_file, "anki-card").await?;

    if raw_cards.is_empty() {
        tracing::info!("No cards found in {}", typst_file.display());
        return Ok(Vec::new());
    }

    // Parse and validate cards
    let mut cards = Vec::new();
    for (index, raw_data) in raw_cards.into_iter().enumerate() {
        match parse_card_data(raw_data, typst_file) {
            Ok(card) => {
                validate_card(&card)?;
                cards.push(card);
            }
            Err(e) => {
                tracing::warn!(
                    "Skipping invalid card {} in {}: {}",
                    index + 1,
                    typst_file.display(),
                    e
                );
            }
        }
    }

    tracing::info!(
        "Extracted {} cards from {}",
        cards.len(),
        typst_file.display()
    );
    Ok(cards)
}

/// Extract configuration from a Typst file.
///
/// This function queries the Typst document for configuration metadata.
///
/// # Arguments
///
/// * `typst_file` - Path to the Typst file to process
///
/// # Returns
///
/// A TypstConfig object with the configuration found in the document.
pub async fn extract_config(typst_file: &Path) -> Result<TypstConfig> {
    tracing::debug!("Extracting config from {}", typst_file.display());

    let raw_configs = query_typst_metadata(typst_file, "anki-config").await?;

    if raw_configs.is_empty() {
        return Ok(TypstConfig::default());
    }

    // Use the first configuration found
    let config_data = raw_configs.into_iter().next().unwrap();
    let config: TypstConfig = serde_json::from_value(config_data)
        .map_err(|e| Error::typst(format!("Invalid configuration format: {}", e)))?;

    tracing::debug!(
        "Extracted config from {}: {:?}",
        typst_file.display(),
        config
    );
    Ok(config)
}

/// Query Typst for metadata with a specific label.
async fn query_typst_metadata(typst_file: &Path, label: &str) -> Result<Vec<serde_json::Value>> {
    let output = Command::new("typst")
        .args(&[
            "query",
            typst_file
                .to_str()
                .ok_or_else(|| Error::typst("Invalid file path encoding".to_string()))?,
            &format!("<{}>", label),
            "--field",
            "value",
        ])
        .output()
        .await
        .map_err(|e| Error::typst(format!("Failed to execute typst command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::typst(format!("Typst query failed: {}", stderr)));
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

/// Parse raw card data from Typst metadata into a Card object.
fn parse_card_data(raw_data: serde_json::Value, source_file: &Path) -> Result<Card> {
    let raw_card: RawCard = serde_json::from_value(raw_data)
        .map_err(|e| Error::card_validation(format!("Invalid card format: {}", e)))?;

    // Apply defaults where necessary
    let card = Card {
        label: raw_card.label,
        model: raw_card.model.unwrap_or_else(|| "Basic".to_string()),
        data: raw_card.data.unwrap_or_else(HashMap::new),
        deck: raw_card.deck.unwrap_or_else(|| "Default".to_string()),
        tags: raw_card.tags.unwrap_or_else(Vec::new),
        rest: raw_card.rest.unwrap_or_else(HashMap::new),
        format: raw_card.format.unwrap_or_default(),
        source_file: source_file.to_path_buf(),
    };

    Ok(card)
}

/// Validate a card object.
fn validate_card(card: &Card) -> Result<()> {
    // Validate label
    if card.label.is_empty() {
        return Err(Error::card_validation("Card label cannot be empty"));
    }

    // Validate model
    if card.model.is_empty() {
        return Err(Error::card_validation("Card model cannot be empty"));
    }

    // Validate deck
    if card.deck.is_empty() {
        return Err(Error::card_validation("Card deck cannot be empty"));
    }

    // Validate that required fields are present
    if card.data.is_empty() {
        return Err(Error::card_validation("Card must have at least one field"));
    }

    // Check for common required fields based on model
    match card.model.as_str() {
        "Basic" => {
            if !card.data.contains_key("Front") || !card.data.contains_key("Back") {
                return Err(Error::card_validation(
                    "Basic model requires 'Front' and 'Back' fields",
                ));
            }
        }
        "Cloze" => {
            if !card.data.contains_key("Text") {
                return Err(Error::card_validation("Cloze model requires 'Text' field"));
            }
        }
        _ => {
            // For other models, we can't validate specific fields
            tracing::debug!("Unknown model '{}', skipping field validation", card.model);
        }
    }

    Ok(())
}

/// Apply default values to a card.
pub fn apply_defaults(card: &mut Card, defaults: &CardDefaults) {
    // Apply default model if not set
    if card.model.is_empty() {
        if let Some(default_model) = &defaults.model {
            card.model = default_model.clone();
        }
    }

    // Apply default deck if not set
    if card.deck.is_empty() {
        if let Some(default_deck) = &defaults.deck {
            card.deck = default_deck.clone();
        }
    }

    // Apply default tags if none set
    if card.tags.is_empty() {
        if let Some(default_tags) = &defaults.tags {
            card.tags = default_tags.clone();
        }
    }

    // Apply default field data for missing fields
    if let Some(default_data) = &defaults.data {
        for (key, value) in default_data {
            card.data
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }

    // Apply default rest metadata for missing keys
    if let Some(default_rest) = &defaults.rest {
        for (key, value) in default_rest {
            card.rest
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_card_data_basic() {
        let raw_data = serde_json::json!({
            "label": "test-card",
            "model": "Basic",
            "data": {
                "Front": "Question",
                "Back": "Answer"
            },
            "deck": "Test",
            "tags": ["tag1", "tag2"],
            "format": "svg"
        });

        let source_file = Path::new("test.typ");
        let card = parse_card_data(raw_data, source_file).unwrap();

        assert_eq!(card.label, "test-card");
        assert_eq!(card.model, "Basic");
        assert_eq!(card.data.get("Front"), Some(&"Question".to_string()));
        assert_eq!(card.data.get("Back"), Some(&"Answer".to_string()));
        assert_eq!(card.deck, "Test");
        assert_eq!(card.tags, vec!["tag1", "tag2"]);
        assert_eq!(card.format, RenderFormat::Svg);
        assert_eq!(card.source_file, source_file);
    }

    #[test]
    fn test_parse_card_data_with_defaults() {
        let raw_data = serde_json::json!({
            "label": "minimal-card",
            "data": {
                "Front": "Question"
            }
        });

        let source_file = Path::new("test.typ");
        let card = parse_card_data(raw_data, source_file).unwrap();

        assert_eq!(card.label, "minimal-card");
        assert_eq!(card.model, "Basic"); // Default
        assert_eq!(card.deck, "Default"); // Default
        assert!(card.tags.is_empty()); // Default
        assert_eq!(card.format, RenderFormat::Svg); // Default
    }

    #[test]
    fn test_validate_card_basic_valid() {
        let card = Card {
            label: "test".to_string(),
            model: "Basic".to_string(),
            data: [
                ("Front".to_string(), "Q".to_string()),
                ("Back".to_string(), "A".to_string()),
            ]
            .into(),
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: Path::new("test.typ").to_path_buf(),
        };

        assert!(validate_card(&card).is_ok());
    }

    #[test]
    fn test_validate_card_empty_label() {
        let card = Card {
            label: "".to_string(),
            model: "Basic".to_string(),
            data: [("Front".to_string(), "Q".to_string())].into(),
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: Path::new("test.typ").to_path_buf(),
        };

        assert!(validate_card(&card).is_err());
    }

    #[test]
    fn test_validate_card_basic_missing_fields() {
        let card = Card {
            label: "test".to_string(),
            model: "Basic".to_string(),
            data: [("Front".to_string(), "Q".to_string())].into(), // Missing Back
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: Path::new("test.typ").to_path_buf(),
        };

        assert!(validate_card(&card).is_err());
    }

    #[test]
    fn test_apply_defaults() {
        let mut card = Card {
            label: "test".to_string(),
            model: "".to_string(), // Will be filled
            data: [("Front".to_string(), "Q".to_string())].into(),
            deck: "".to_string(), // Will be filled
            tags: vec![],         // Will be filled
            rest: HashMap::new(),
            format: RenderFormat::Svg,
            source_file: Path::new("test.typ").to_path_buf(),
        };

        let defaults = CardDefaults {
            model: Some("Basic".to_string()),
            deck: Some("DefaultDeck".to_string()),
            tags: Some(vec!["default".to_string()]),
            data: Some([("Back".to_string(), "DefaultBack".to_string())].into()),
            rest: None,
        };

        apply_defaults(&mut card, &defaults);

        assert_eq!(card.model, "Basic");
        assert_eq!(card.deck, "DefaultDeck");
        assert_eq!(card.tags, vec!["default"]);
        assert_eq!(card.data.get("Back"), Some(&"DefaultBack".to_string()));
        assert_eq!(card.data.get("Front"), Some(&"Q".to_string())); // Original value preserved
    }
}
