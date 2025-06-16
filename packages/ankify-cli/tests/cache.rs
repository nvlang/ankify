use ankify::ankiconnect::{Deck, Field, Model, NoteId, Tag};
use ankify::cache::{Cache, CacheEntry, Label, Sha256};
use ankify::metadata::{Note, NoteDataValue, NoteDataValueWithFormat};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

#[test]
fn test_sha256_creation() {
    let hash = Sha256::new("abc123".to_string());
    assert_eq!(hash.as_str(), "abc123");
}

#[test]
fn test_sha256_from_text() {
    let hash = Sha256::from_text("hello world");
    // This should be the actual SHA-256 hash of "hello world"
    assert_eq!(
        hash.as_str(),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn test_sha256_from_bytes() {
    let data = b"hello world";
    let hash = Sha256::from_bytes(data);
    assert_eq!(
        hash.as_str(),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn test_label_creation() {
    let label = Label::new("test-label".to_string());
    assert_eq!(label.as_str(), "test-label");

    let label_from_str: Label = "test-label".into();
    assert_eq!(label_from_str.as_str(), "test-label");

    let label_from_string: Label = "test-label".to_string().into();
    assert_eq!(label_from_string.as_str(), "test-label");
}

#[test]
fn test_cache_entry_creation() {
    let label = Label::new("test".to_string());
    let id = NoteId(12345);
    let tags = vec![Tag::new("tag1".to_string())];
    let mut hash = HashMap::new();
    hash.insert(
        Field::new("Front".to_string()),
        Sha256::new("hash1".to_string()),
    );
    let deck = Deck::new("Default".to_string());
    let model = Model::new("Basic".to_string());

    let entry = CacheEntry {
        label,
        id,
        tags,
        hash,
        deck,
        model,
    };
    assert_eq!(entry.label.as_str(), "test");
    assert_eq!(
        entry
            .get_field_hash(&Field::new("Front".to_string()))
            .unwrap()
            .as_str(),
        "hash1"
    );
}

#[test]
fn test_cache_entry_has_same_content() {
    let label = Label::new("test".to_string());
    let id = NoteId(12345);
    let tags = vec![];
    let mut hash = HashMap::new();
    hash.insert(
        Field::new("Front".to_string()),
        Sha256::new("hash1".to_string()),
    );
    hash.insert(
        Field::new("Back".to_string()),
        Sha256::new("hash2".to_string()),
    );
    let deck = Deck::new("Default".to_string());
    let model = Model::new("Basic".to_string());

    let entry = CacheEntry {
        label,
        id,
        tags,
        hash,
        deck,
        model,
    };

    // Same content
    let mut field_hashes = HashMap::new();
    field_hashes.insert(
        Field::new("Front".to_string()),
        Sha256::new("hash1".to_string()),
    );
    field_hashes.insert(
        Field::new("Back".to_string()),
        Sha256::new("hash2".to_string()),
    );
    assert!(entry.has_same_content(&field_hashes));

    // Different content
    field_hashes.insert(
        Field::new("Front".to_string()),
        Sha256::new("different_hash".to_string()),
    );
    assert!(!entry.has_same_content(&field_hashes));

    // Missing field
    field_hashes.remove(&Field::new("Back".to_string()));
    assert!(!entry.has_same_content(&field_hashes));

    // Extra field
    field_hashes.insert(
        Field::new("Back".to_string()),
        Sha256::new("hash2".to_string()),
    );
    field_hashes.insert(
        Field::new("Extra".to_string()),
        Sha256::new("extra_hash".to_string()),
    );
    assert!(!entry.has_same_content(&field_hashes));
}

#[test]
fn test_cache_basic_operations() {
    let mut cache = Cache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);

    let entry = create_test_cache_entry("test", 123);
    cache.insert("test".to_string(), entry);

    assert!(!cache.is_empty());
    assert_eq!(cache.len(), 1);
    assert!(cache.contains("test"));
    assert!(cache.get("test").is_some());

    let removed = cache.remove("test");
    assert!(removed.is_some());
    assert!(cache.is_empty());
}

#[test]
fn test_cache_labels_and_entries() {
    let mut cache = Cache::new();
    cache.insert("label1".to_string(), create_test_cache_entry("label1", 1));
    cache.insert("label2".to_string(), create_test_cache_entry("label2", 2));

    let labels = cache.labels();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&&"label1".to_string()));
    assert!(labels.contains(&&"label2".to_string()));

    let entries = cache.entries();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_cache_has_note_changed() {
    let mut cache = Cache::new();
    let entry = create_test_cache_entry("test", 123);
    cache.insert("test".to_string(), entry);

    // Same content - should not have changed
    let mut field_hashes = HashMap::new();
    field_hashes.insert(
        Field::new("Front".to_string()),
        Sha256::new("hash1".to_string()),
    );
    assert!(!cache.has_note_changed("test", &field_hashes));

    // Different content - should have changed
    field_hashes.insert(
        Field::new("Front".to_string()),
        Sha256::new("different".to_string()),
    );
    assert!(cache.has_note_changed("test", &field_hashes));

    // Note not in cache - should have changed
    assert!(cache.has_note_changed("nonexistent", &field_hashes));
}

#[test]
fn test_cache_get_note_id() {
    let mut cache = Cache::new();
    let entry = create_test_cache_entry("test", 123);
    cache.insert("test".to_string(), entry);

    assert_eq!(cache.get_note_id("test"), Some(NoteId(123)));
    assert_eq!(cache.get_note_id("nonexistent"), None);
}

#[test]
fn test_cache_clear() {
    let mut cache = Cache::new();
    cache.insert("test1".to_string(), create_test_cache_entry("test1", 1));
    cache.insert("test2".to_string(), create_test_cache_entry("test2", 2));

    assert_eq!(cache.len(), 2);
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn test_cache_get_mut() {
    let mut cache = Cache::new();
    cache.insert("test".to_string(), create_test_cache_entry("test", 123));

    {
        let entry = cache.get_mut("test").unwrap();
        entry.set_field_hash(
            Field::new("NewField".to_string()),
            Sha256::new("newhash".to_string()),
        );
    }

    let entry = cache.get("test").unwrap();
    assert!(entry
        .get_field_hash(&Field::new("NewField".to_string()))
        .is_some());
}

#[tokio::test]
async fn test_cache_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let cache_file = temp_dir.path().join("test_cache.json");

    // Test saving and loading
    let mut cache = Cache::with_file(&cache_file);
    cache.insert("test".to_string(), create_test_cache_entry("test", 123));

    cache.save().await.unwrap();
    assert!(cache_file.exists());

    // Load from file
    let loaded_cache = Cache::load_from_file(&cache_file).await.unwrap();
    assert_eq!(loaded_cache.len(), 1);
    assert!(loaded_cache.contains("test"));
}

#[tokio::test]
async fn test_cache_load_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let cache_file = temp_dir.path().join("nonexistent.json");

    let cache = Cache::load_from_file(&cache_file).await.unwrap();
    assert!(cache.is_empty());
}

#[tokio::test]
async fn test_cache_save_without_file_path() {
    let cache = Cache::new();
    let result = cache.save().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No cache file path specified"));
}

#[tokio::test]
async fn test_cache_save_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let cache_file = temp_dir.path().join("subdir").join("cache.json");

    let mut cache = Cache::with_file(&cache_file);
    cache.insert("test".to_string(), create_test_cache_entry("test", 123));

    cache.save().await.unwrap();
    assert!(cache_file.exists());
}

#[tokio::test]
async fn test_cache_load_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let cache_file = temp_dir.path().join("invalid.json");
    fs::write(&cache_file, "invalid json content")
        .await
        .unwrap();

    let result = Cache::load_from_file(&cache_file).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse cache file"));
}

#[tokio::test]
async fn test_cache_update_from_note() {
    let mut cache = Cache::new();
    let note = create_test_note();
    let mut field_hashes = HashMap::new();
    field_hashes.insert(
        Field::new("Front".to_string()),
        Sha256::new("test_hash".to_string()),
    );

    cache
        .update_from_note(&note, NoteId(12345), field_hashes.clone())
        .await
        .unwrap();

    assert!(cache.contains(&note.label));
    let entry = cache.get(&note.label).unwrap();
    assert_eq!(entry.id, NoteId(12345));
    assert_eq!(entry.deck.as_str(), "TestDeck");
    assert_eq!(entry.model.as_str(), "TestModel");
    assert_eq!(entry.tags.len(), 1);
    assert_eq!(entry.tags[0].as_str(), "tag1");
}

#[tokio::test]
async fn test_hash_note_data_value_simple() {
    let cache = Cache::new();
    let data_value = NoteDataValue::Simple("test content".to_string());
    let hash = cache.hash_note_data_value(&data_value, None).await.unwrap();

    // Should be the SHA-256 of "test content"
    let expected = Sha256::from_text("test content");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_with_format_plain() {
    let cache = Cache::new();
    let data_value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
        value: "test content".to_string(),
        format: Some("plain".to_string()),
    });
    let hash = cache.hash_note_data_value(&data_value, None).await.unwrap();

    let expected = Sha256::from_text("test content");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_with_format_no_format() {
    let cache = Cache::new();
    let data_value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
        value: "test content".to_string(),
        format: None,
    });
    let hash = cache.hash_note_data_value(&data_value, None).await.unwrap();

    let expected = Sha256::from_text("test content");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_media_file_exists() {
    let temp_dir = TempDir::new().unwrap();
    let media_file = temp_dir.path().join("test.png");
    fs::write(&media_file, b"fake png data").await.unwrap();

    let cache = Cache::new();
    let data_value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
        value: "test.png".to_string(),
        format: Some("png".to_string()),
    });

    let hash = cache
        .hash_note_data_value(&data_value, Some(temp_dir.path()))
        .await
        .unwrap();
    let expected = Sha256::from_bytes(b"fake png data");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_media_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();

    let cache = Cache::new();
    let data_value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
        value: "nonexistent.png".to_string(),
        format: Some("png".to_string()),
    });

    let hash = cache
        .hash_note_data_value(&data_value, Some(temp_dir.path()))
        .await
        .unwrap();
    let expected = Sha256::from_text("nonexistent.png");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_media_no_media_dir() {
    let cache = Cache::new();
    let data_value = NoteDataValue::WithFormat(NoteDataValueWithFormat {
        value: "test.svg".to_string(),
        format: Some("svg".to_string()),
    });

    let hash = cache.hash_note_data_value(&data_value, None).await.unwrap();
    let expected = Sha256::from_text("test.svg");
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_hash_note_data_value_complex() {
    let cache = Cache::new();
    let complex_value = serde_json::json!({"key": "value", "number": 42});
    let data_value = NoteDataValue::Complex(complex_value.clone());

    let hash = cache.hash_note_data_value(&data_value, None).await.unwrap();
    let expected_json = serde_json::to_string(&complex_value).unwrap();
    let expected = Sha256::from_text(&expected_json);
    assert_eq!(hash.as_str(), expected.as_str());
}

#[tokio::test]
async fn test_create_field_hashes_from_note_data() {
    let cache = Cache::new();
    let mut note_data = HashMap::new();
    note_data.insert(
        "Front".to_string(),
        NoteDataValue::Simple("front content".to_string()),
    );
    note_data.insert(
        "Back".to_string(),
        NoteDataValue::Simple("back content".to_string()),
    );

    let field_hashes = cache
        .create_field_hashes_from_note_data(&note_data, None)
        .await
        .unwrap();

    assert_eq!(field_hashes.len(), 2);
    assert!(field_hashes.contains_key(&Field::new("Front".to_string())));
    assert!(field_hashes.contains_key(&Field::new("Back".to_string())));
}

#[tokio::test]
async fn test_create_field_hashes_empty_note_data() {
    let cache = Cache::new();
    let note_data = HashMap::new();

    let field_hashes = cache
        .create_field_hashes_from_note_data(&note_data, None)
        .await
        .unwrap();

    assert!(field_hashes.is_empty());
}

#[test]
fn test_cache_entry_set_field_hash() {
    let mut entry = create_test_cache_entry("test", 123);
    let new_field = Field::new("NewField".to_string());
    let new_hash = Sha256::new("newhash".to_string());

    entry.set_field_hash(new_field.clone(), new_hash.clone());
    assert_eq!(entry.get_field_hash(&new_field), Some(&new_hash));
}

#[test]
fn test_cache_default() {
    let cache = Cache::default();
    assert!(cache.is_empty());
}

// Helper functions for tests
fn create_test_cache_entry(label: &str, id: u64) -> CacheEntry {
    let label = Label::new(label.to_string());
    let id = NoteId(id);
    let tags = vec![];
    let mut hash = HashMap::new();
    hash.insert(
        Field::new("Front".to_string()),
        Sha256::new("hash1".to_string()),
    );
    let deck = Deck::new("Default".to_string());
    let model = Model::new("Basic".to_string());

    CacheEntry {
        label,
        id,
        tags,
        hash,
        deck,
        model,
    }
}

fn create_test_note() -> Note {
    let mut data = HashMap::new();
    data.insert(
        "Front".to_string(),
        NoteDataValue::Simple("test front".to_string()),
    );

    Note {
        label: "test-note".to_string(),
        model: "TestModel".to_string(),
        data,
        deck: "TestDeck".to_string(),
        tags: vec!["tag1".to_string()],
        other: serde_json::Value::Null,
        format: Some("plain".to_string()),
    }
}
