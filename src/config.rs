// See the file LICENSE for the full license governing this code.

//! Configuration management and option merging.
//!
//! This module handles merging configuration from CLI arguments and Typst documents,
//! with proper precedence and validation.

use crate::{cli::Cli, types::*, typst, Error, Result};
use std::path::PathBuf;

/// Complete configuration for the Ankify application.
///
/// This struct contains all configuration options, merged from CLI arguments
/// and Typst document configuration with proper precedence.
#[derive(Debug, Clone)]
pub struct Config {
    /// Typst file patterns to process
    pub typst_files: Vec<String>,

    /// AnkiConnect URL
    pub ankiconnect_url: String,

    /// Enable verbose output
    pub verbose: bool,

    /// Path to auxiliary cache file
    pub aux_file: PathBuf,

    /// Default values for card fields
    pub defaults: CardDefaults,

    /// Bypass cache and update all cards
    pub bypass_cache: bool,

    /// Custom render function path
    pub render: Option<PathBuf>,

    /// Watch mode enabled
    pub watch: bool,

    /// Cache directory for rendered content
    pub cache_dir: PathBuf,

    /// Default render format for cards
    pub render_format: RenderFormat,
}

impl Config {
    /// Create a configuration from CLI arguments and Typst document configuration.
    ///
    /// This method will:
    /// 1. Parse CLI arguments
    /// 2. Extract configuration from the first matching Typst file
    /// 3. Merge them with proper precedence (CLI overrides Typst)
    /// 4. Validate the final configuration
    ///
    /// # Arguments
    ///
    /// * `cli` - Parsed CLI arguments
    ///
    /// # Returns
    ///
    /// A validated configuration object.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No matching Typst files are found
    /// - Typst configuration extraction fails
    /// - The merged configuration is invalid
    pub async fn from_cli(cli: Cli) -> Result<Self> {
        // Extract Typst configuration from the first matching file
        let typst_config = Self::extract_typst_config(&cli.typst_files).await?;

        // Merge CLI and Typst configuration
        let config = Self::merge_configs(cli, typst_config)?;

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Create a default configuration for testing purposes.
    pub fn default_for_testing() -> Self {
        Self {
            typst_files: vec!["*.typ".to_string()],
            ankiconnect_url: "http://localhost:8765".to_string(),
            verbose: false,
            aux_file: PathBuf::from("ankify.aux.json"),
            defaults: CardDefaults::default(),
            bypass_cache: false,
            render: None,
            watch: false,
            cache_dir: PathBuf::from("cache"),
            render_format: RenderFormat::Svg,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate AnkiConnect URL
        if self.ankiconnect_url.is_empty() {
            return Err(Error::config("AnkiConnect URL cannot be empty"));
        }

        if !self.ankiconnect_url.starts_with("http://")
            && !self.ankiconnect_url.starts_with("https://")
        {
            return Err(Error::config(
                "AnkiConnect URL must start with http:// or https://",
            ));
        }

        // Validate render file if provided
        if let Some(render_file) = &self.render {
            if !render_file.exists() {
                return Err(Error::config(format!(
                    "Render file does not exist: {}",
                    render_file.display()
                )));
            }
        }

        // Ensure at least one file pattern is provided
        if self.typst_files.is_empty() {
            return Err(Error::config(
                "At least one Typst file pattern must be provided",
            ));
        }

        Ok(())
    }

    /// Extract configuration from Typst files.
    async fn extract_typst_config(file_patterns: &[String]) -> Result<TypstConfig> {
        use glob::glob;

        // Find the first matching file to extract configuration from
        for pattern in file_patterns {
            for entry in glob(pattern)? {
                let file_path = entry?;
                if file_path.extension().map_or(false, |ext| ext == "typ") {
                    match typst::extract_config(&file_path).await {
                        Ok(config) => return Ok(config),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to extract config from {}: {}",
                                file_path.display(),
                                e
                            );
                            // Continue to next file
                        }
                    }
                }
            }
        }

        // No configuration found, return default
        Ok(TypstConfig::default())
    }

    /// Merge CLI and Typst configuration with proper precedence.
    fn merge_configs(cli: Cli, typst_config: TypstConfig) -> Result<Self> {
        // CLI arguments take precedence over Typst configuration
        let ankiconnect_url = if cli.ankiconnect_url != "http://localhost:8765" {
            // CLI value was explicitly set
            cli.ankiconnect_url
        } else {
            // Use Typst value if available, otherwise CLI default
            typst_config.ankiconnect_url.unwrap_or(cli.ankiconnect_url)
        };

        let verbose = cli.verbose || typst_config.verbose.unwrap_or(false);

        let aux_file = if cli.aux_file != PathBuf::from("ankify.aux.json") {
            // CLI value was explicitly set
            cli.aux_file
        } else {
            // Use Typst value if available, otherwise CLI default
            typst_config.aux_file.unwrap_or(cli.aux_file)
        };

        let defaults = typst_config.defaults.unwrap_or_else(CardDefaults::default);

        let render = cli.render.or(typst_config.render);

        Ok(Self {
            typst_files: cli.typst_files,
            ankiconnect_url,
            verbose,
            aux_file,
            defaults,
            bypass_cache: cli.bypass_cache,
            render,
            watch: cli.watch,
            cache_dir: PathBuf::from("cache"),
            render_format: RenderFormat::Svg,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::default_for_testing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_url() {
        let mut config = Config::default();
        config.ankiconnect_url = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_url() {
        let mut config = Config::default();
        config.ankiconnect_url = "invalid-url".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_no_files() {
        let mut config = Config::default();
        config.typst_files.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_merge_configs_cli_precedence() {
        let cli = Cli {
            typst_files: vec!["*.typ".to_string()],
            ankiconnect_url: "http://cli-url:8765".to_string(),
            verbose: true,
            aux_file: PathBuf::from("cli.aux.json"),
            bypass_cache: true,
            render: Some(PathBuf::from("cli-render.typ")),
            watch: true,
            log_level: "info".to_string(),
        };

        let typst_config = TypstConfig {
            ankiconnect_url: Some("http://typst-url:8765".to_string()),
            verbose: Some(false),
            aux_file: Some(PathBuf::from("typst.aux.json")),
            defaults: Some(CardDefaults::default()),
            render: Some(PathBuf::from("typst-render.typ")),
        };

        let config = Config::merge_configs(cli, typst_config).unwrap();

        // CLI values should take precedence
        assert_eq!(config.ankiconnect_url, "http://cli-url:8765");
        assert!(config.verbose); // CLI true overrides Typst false
        assert_eq!(config.aux_file, PathBuf::from("cli.aux.json"));
        assert_eq!(config.render, Some(PathBuf::from("cli-render.typ")));
    }

    #[test]
    fn test_merge_configs_typst_fallback() {
        let cli = Cli {
            typst_files: vec!["*.typ".to_string()],
            ankiconnect_url: "http://localhost:8765".to_string(), // Default value
            verbose: false,
            aux_file: PathBuf::from("ankify.aux.json"), // Default value
            bypass_cache: false,
            render: None,
            watch: false,
            log_level: "info".to_string(),
        };

        let typst_config = TypstConfig {
            ankiconnect_url: Some("http://typst-url:8765".to_string()),
            verbose: Some(true),
            aux_file: Some(PathBuf::from("typst.aux.json")),
            defaults: Some(CardDefaults::default()),
            render: Some(PathBuf::from("typst-render.typ")),
        };

        let config = Config::merge_configs(cli, typst_config).unwrap();

        // Typst values should be used when CLI has defaults
        assert_eq!(config.ankiconnect_url, "http://typst-url:8765");
        assert!(config.verbose);
        assert_eq!(config.aux_file, PathBuf::from("typst.aux.json"));
        assert_eq!(config.render, Some(PathBuf::from("typst-render.typ")));
    }
}
