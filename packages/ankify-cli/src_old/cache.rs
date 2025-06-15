// See the file LICENSE for the full license governing this code.

//! Auxiliary file management and caching logic.
//!
//! This module handles the auxiliary JSON file that tracks card metadata
//! and determines what operations need to be performed on Anki cards.

use crate::{types::*, Error, Result};
use std::collections::HashMap;
use std::path::Path;

/// Operations that can be performed on Anki cards.
#[derive(Debug, Clone)]
pub enum Operation {
    /// Create a new card in Anki
    Create { card: Card },
    /// Update an existing card in Anki
    Update { card: Card, anki_id: String },
    /// Delete a card from Anki
    Delete { anki_id: String },
    /// Skip this card (no changes needed)
    Skip { card: Card },
}

/// Cache for managing card metadata and determining required operations.
#[derive(Debug)]
pub struct Cache {
    /// Path to the auxiliary file
    aux_file: std::path::PathBuf,
    /// Current cache entries indexed by label
    entries: HashMap<String, CacheEntry>,
    /// Whether the cache has been modified
    dirty: bool,
}

impl Cache {
    /// Create a new cache instance.
    ///
    /// If the auxiliary file exists, it will be loaded. Otherwise, an empty
    /// cache will be created.
    ///
    /// # Arguments
    ///
    /// * `aux_file` - Path to the auxiliary JSON file
    ///
    /// # Returns
    ///
    /// A new Cache instance.
    pub fn new(aux_file: &Path) -> Result<Self> {
        let mut cache = Self {
            aux_file: aux_file.to_path_buf(),
            entries: HashMap::new(),
            dirty: false,
        };

        // Load existing cache if file exists
        if aux_file.exists() {
            cache.load_from_file()?;
        }

        Ok(cache)
    }

    /// Plan operations based on current cards and cache state.
    ///
    /// This function compares the provided cards against the cache to determine
    /// what operations need to be performed:
    ///
    /// 1. If label doesn't exist in cache, create a new card
    /// 2. If label exists but any field hash differs, update the card
    /// 3. If label exists and all field hashes match, skip (no changes needed)
    /// 4. If cache contains entries not in the current cards, delete them
    ///
    /// # Arguments
    ///
    /// * `cards` - Vector of cards to process
    ///
    /// # Returns
    ///
    /// A vector of operations to perform.
    pub fn plan_operations(&self, cards: &[Card]) -> Result<Vec<Operation>> {
        let mut operations = Vec::new();
        let mut processed_labels = std::collections::HashSet::new();

        for card in cards {
            let card_hashes = card.content_hashes();
            processed_labels.insert(card.label.clone());

            if let Some(entry) = self.entries.get(&card.label) {
                // Label matches - check if any field hash has changed
                let hashes_match = card_hashes.len() == entry.hash.len()
                    && card_hashes.iter().all(|(field, hash)| {
                        entry
                            .hash
                            .get(field)
                            .map_or(false, |entry_hash| entry_hash == hash)
                    });

                if hashes_match {
                    // All hashes match - no changes needed
                    operations.push(Operation::Skip { card: card.clone() });
                } else {
                    // Some hash differs - update needed
                    operations.push(Operation::Update {
                        card: card.clone(),
                        anki_id: entry.id.clone(),
                    });
                }
            } else {
                // Label doesn't exist - create new card
                operations.push(Operation::Create { card: card.clone() });
            }
        }

        // Check for entries that should be deleted (not in current cards)
        for (label, entry) in &self.entries {
            if !processed_labels.contains(label) {
                operations.push(Operation::Delete {
                    anki_id: entry.id.clone(),
                });
                tracing::info!("Card '{}' no longer exists, will be deleted", label);
            }
        }

        Ok(operations)
    }

    /// Record the creation of a new card.
    ///
    /// # Arguments
    ///
    /// * `card` - The card that was created
    /// * `anki_id` - The ID returned by AnkiConnect
    pub fn record_creation(&mut self, card: &Card, anki_id: String) -> Result<()> {
        let entry = CacheEntry {
            label: card.label.clone(),
            id: anki_id,
            hash: card.content_hashes(),
        };

        self.entries.insert(card.label.clone(), entry);
        self.dirty = true;

        tracing::debug!("Recorded creation of card '{}'", card.label);
        Ok(())
    }

    /// Record the update of an existing card.
    ///
    /// # Arguments
    ///
    /// * `card` - The card that was updated
    pub fn record_update(&mut self, card: &Card) -> Result<()> {
        if let Some(entry) = self.entries.get_mut(&card.label) {
            entry.hash = card.content_hashes();
            self.dirty = true;

            tracing::debug!("Recorded update of card '{}'", card.label);
        } else {
            return Err(Error::cache(format!(
                "Cannot update non-existent card '{}'",
                card.label
            )));
        }

        Ok(())
    }

    /// Record the deletion of a card.
    ///
    /// # Arguments
    ///
    /// * `anki_id` - The AnkiConnect ID of the deleted card
    pub fn record_deletion(&mut self, anki_id: String) -> Result<()> {
        // Find and remove the entry with this ID
        let mut to_remove = None;
        for (label, entry) in &self.entries {
            if entry.id == anki_id {
                to_remove = Some(label.clone());
                break;
            }
        }

        if let Some(label) = to_remove {
            self.entries.remove(&label);
            self.dirty = true;
            tracing::debug!("Recorded deletion of card '{}'", label);
        } else {
            tracing::warn!(
                "Attempted to delete non-existent card with ID '{}'",
                anki_id
            );
        }

        Ok(())
    }

    /// Save the cache to the auxiliary file if it has been modified.
    pub async fn save(&mut self) -> Result<()> {
        if !self.dirty {
            tracing::debug!(
                "Cache not dirty, skipping save for {}",
                self.aux_file.display()
            );
            return Ok(());
        }

        let entries: Vec<_> = self.entries.values().cloned().collect();
        let json = serde_json::to_string_pretty(&entries)?;

        // Ensure parent directory exists
        if let Some(parent) = self.aux_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write atomically by writing to a temporary file first
        let temp_file = self.aux_file.with_extension("tmp");
        tracing::debug!(
            "Writing aux file to {} (tmp: {})",
            self.aux_file.display(),
            temp_file.display()
        );
        tokio::fs::write(&temp_file, json).await?;
        tokio::fs::rename(&temp_file, &self.aux_file).await?;

        self.dirty = false;
        tracing::debug!("Saved cache to {}", self.aux_file.display());

        Ok(())
    }

    /// Load cache entries from the auxiliary file.
    fn load_from_file(&mut self) -> Result<()> {
        let content = std::fs::read_to_string(&self.aux_file)?;

        // Handle empty files gracefully
        if content.trim().is_empty() {
            tracing::debug!(
                "Cache file {} is empty, starting with empty cache",
                self.aux_file.display()
            );
            return Ok(());
        }

        let entries: Vec<CacheEntry> = serde_json::from_str(&content)?;

        self.entries.clear();
        for entry in entries {
            self.entries.insert(entry.label.clone(), entry);
        }

        tracing::debug!(
            "Loaded {} cache entries from {}",
            self.entries.len(),
            self.aux_file.display()
        );

        Ok(())
    }

    /// Get cache statistics for debugging.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
            dirty: self.dirty,
            file_path: self.aux_file.clone(),
        }
    }

    /// Returns the path to the media directory used by this cache.
    pub fn media_dir(&self) -> std::path::PathBuf {
        self.aux_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("media")
    }
}

/// Cache statistics for debugging and monitoring.
#[derive(Debug)]
pub struct CacheStats {
    /// Total number of entries in the cache
    pub total_entries: usize,
    /// Whether the cache has unsaved changes
    pub dirty: bool,
    /// Path to the auxiliary file
    pub file_path: std::path::PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn create_test_card(label: &str, content: &str) -> Card {
        Card {
            label: label.to_string(),
            model: "Basic".to_string(),
            data: [
                ("Front".to_string(), content.to_string()),
                ("Back".to_string(), "Answer".to_string()),
            ]
            .into(),
            deck: "Test".to_string(),
            tags: vec![],
            rest: HashMap::new(),
            format: RenderFormat::Single(FieldFormat::Svg),
            source_file: std::path::PathBuf::from("test.typ"),
        }
    }

    #[test]
    fn test_cache_new_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Remove the file so cache starts empty
        std::fs::remove_file(temp_path).unwrap();

        let cache = Cache::new(temp_path).unwrap();
        assert_eq!(cache.entries.len(), 0);
        assert!(!cache.dirty);
    }

    #[test]
    fn test_plan_operations_create_new() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Remove the file so cache starts empty
        std::fs::remove_file(temp_path).unwrap();

        let cache = Cache::new(temp_path).unwrap();

        let cards = vec![create_test_card("new-card", "New content")];
        let operations = cache.plan_operations(&cards).unwrap();

        assert_eq!(operations.len(), 1);
        matches!(operations[0], Operation::Create { .. });
    }

    #[test]
    fn test_plan_operations_skip_unchanged() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Remove the file so cache starts empty
        std::fs::remove_file(temp_path).unwrap();

        let mut cache = Cache::new(temp_path).unwrap();

        let card = create_test_card("existing-card", "Content");

        // Add entry to cache
        cache.record_creation(&card, "test-id".to_string()).unwrap();

        let operations = cache.plan_operations(&[card]).unwrap();

        assert_eq!(operations.len(), 1);
        matches!(operations[0], Operation::Skip { .. });
    }

    #[test]
    fn test_plan_operations_update_changed() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Remove the file so cache starts empty
        std::fs::remove_file(temp_path).unwrap();

        let mut cache = Cache::new(temp_path).unwrap();

        let original_card = create_test_card("test-card", "Original content");
        cache
            .record_creation(&original_card, "test-id".to_string())
            .unwrap();

        let updated_card = create_test_card("test-card", "Updated content");
        let operations = cache.plan_operations(&[updated_card]).unwrap();

        assert_eq!(operations.len(), 1);
        if let Operation::Update { anki_id, .. } = &operations[0] {
            assert_eq!(anki_id, "test-id");
        } else {
            panic!("Expected Update operation");
        }
    }

    #[test]
    fn test_plan_operations_delete_removed() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Remove the file so cache starts empty
        std::fs::remove_file(temp_path).unwrap();

        let mut cache = Cache::new(temp_path).unwrap();

        let card = create_test_card("to-be-deleted", "Content");
        cache.record_creation(&card, "test-id".to_string()).unwrap();

        // Plan operations with empty card list
        let operations = cache.plan_operations(&[]).unwrap();

        assert_eq!(operations.len(), 1);
        if let Operation::Delete { anki_id } = &operations[0] {
            assert_eq!(anki_id, "test-id");
        } else {
            panic!("Expected Delete operation");
        }
    }

    #[tokio::test]
    async fn test_cache_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Create cache and add entry
        {
            let mut cache = Cache::new(&temp_path).unwrap();
            let card = create_test_card("test-card", "Content");
            cache.record_creation(&card, "test-id".to_string()).unwrap();
            cache.save().await.unwrap();
        }

        // Load cache in new instance
        {
            let cache = Cache::new(&temp_path).unwrap();
            assert_eq!(cache.entries.len(), 1);
            assert!(cache.entries.contains_key("test-card"));
        }
    }
}
