// See the file LICENSE for the full license governing this code.

//! Error types and result aliases for the Ankify library.

/// The main error type for the Ankify library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O errors (file operations, network requests, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Typst execution errors
    #[error("Typst error: {0}")]
    Typst(String),

    /// AnkiConnect API errors
    #[error("AnkiConnect error: {0}")]
    AnkiConnect(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Cache/auxiliary file errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// Rendering errors
    #[error("Rendering error: {0}")]
    Render(String),

    /// Card validation errors
    #[error("Card validation error: {0}")]
    CardValidation(String),

    /// File system errors
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Render error (alias for compatibility)
    #[error("Render error: {0}")]
    RenderError(String),

    /// Missing dependency error
    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    /// Generic error with custom message
    #[error("{0}")]
    Custom(String),
}

/// Convenience Result type alias for the Ankify library.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new custom error with the given message.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Error::Custom(message.into())
    }

    /// Create a new Typst error with the given message.
    pub fn typst<S: Into<String>>(message: S) -> Self {
        Error::Typst(message.into())
    }

    /// Create a new AnkiConnect error with the given message.
    pub fn anki_connect<S: Into<String>>(message: S) -> Self {
        Error::AnkiConnect(message.into())
    }

    /// Create a new configuration error with the given message.
    pub fn config<S: Into<String>>(message: S) -> Self {
        Error::Config(message.into())
    }

    /// Create a new cache error with the given message.
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Error::Cache(message.into())
    }

    /// Create a new rendering error with the given message.
    pub fn render<S: Into<String>>(message: S) -> Self {
        Error::Render(message.into())
    }

    /// Create a new card validation error with the given message.
    pub fn card_validation<S: Into<String>>(message: S) -> Self {
        Error::CardValidation(message.into())
    }
}
