//! Cache module.
//!
//! This module should provide the following:
//! -

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ankiconnect::{Deck, Field, Model, NoteId, Tag};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Sha256(String);

/// Label of a note, as parsed from a Typst file's metadata.
struct Label(String);

///
struct CacheEntry {
    id: NoteId,
    tags: Vec<Tag>,
    hash: HashMap<Field, Sha256>,
    deck: Deck,
    model: Model,
}
