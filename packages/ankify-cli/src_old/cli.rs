// See the file LICENSE for the full license governing this code.

//! Command-line interface and argument parsing.

use clap::Parser;
use std::path::PathBuf;
use tracing::Level;

/// Advanced Typst to Anki bridge with caching and templating support.
#[derive(Parser, Debug, Clone)]
#[command(name = "ankify")]
#[command(version)]
#[command(about = "Advanced Typst to Anki bridge with caching and templating support")]
#[command(long_about = None)]
pub struct Cli {
    /// Typst file(s) to process (supports glob patterns like '*.typ')
    #[arg(value_name = "FILES")]
    pub typst_files: Vec<String>,

    /// AnkiConnect URL
    #[arg(long, default_value = "http://localhost:8765")]
    pub ankiconnect_url: String,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to auxiliary file for caching card metadata
    #[arg(long, default_value = "ankify.aux.json")]
    pub aux_file: PathBuf,

    /// Bypass cache and update all cards, even if unchanged
    #[arg(long)]
    pub bypass_cache: bool,

    /// Path to custom render function (Typst file)
    #[arg(long)]
    pub render: Option<PathBuf>,

    /// Watch mode: continuously monitor files for changes
    #[arg(short, long)]
    pub watch: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

impl Cli {
    /// Parse command-line arguments and return a Cli struct.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Initialize logging based on the CLI arguments.
    pub fn init_logging(&self) -> crate::Result<()> {
        let level = match self.log_level.to_lowercase().as_str() {
            "error" => Level::ERROR,
            "warn" | "warning" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => {
                return Err(crate::Error::config(format!(
                    "Invalid log level: {}. Must be one of: error, warn, info, debug, trace",
                    self.log_level
                )));
            }
        };

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(level)
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| crate::Error::config(format!("Failed to initialize logging: {}", e)))?;

        Ok(())
    }

    /// Validate the CLI arguments.
    pub fn validate(&self) -> crate::Result<()> {
        // Ensure at least one file pattern is provided
        if self.typst_files.is_empty() {
            return Err(crate::Error::config(
                "At least one Typst file pattern must be provided".to_string(),
            ));
        }

        // Validate render file exists if provided
        if let Some(render_file) = &self.render {
            if !render_file.exists() {
                return Err(crate::Error::config(format!(
                    "Render file does not exist: {}",
                    render_file.display()
                )));
            }

            if render_file.extension().map_or(true, |ext| ext != "typ") {
                return Err(crate::Error::config(format!(
                    "Render file must have .typ extension: {}",
                    render_file.display()
                )));
            }
        }

        // Validate aux file directory exists
        if let Some(parent) = self.aux_file.parent() {
            if !parent.exists() {
                return Err(crate::Error::config(format!(
                    "Directory for auxiliary file does not exist: {}",
                    parent.display()
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_basic() {
        let args = vec!["ankify", "test.typ"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.typst_files, vec!["test.typ"]);
        assert_eq!(cli.ankiconnect_url, "http://localhost:8765");
        assert!(!cli.verbose);
        assert!(!cli.bypass_cache);
        assert!(!cli.watch);
    }

    #[test]
    fn test_cli_parse_all_options() {
        let args = vec![
            "ankify",
            "*.typ",
            "--ankiconnect-url",
            "http://example.com:8765",
            "--verbose",
            "--aux-file",
            "custom.aux.json",
            "--bypass-cache",
            "--watch",
            "--log-level",
            "debug",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.typst_files, vec!["*.typ"]);
        assert_eq!(cli.ankiconnect_url, "http://example.com:8765");
        assert!(cli.verbose);
        assert_eq!(cli.aux_file, PathBuf::from("custom.aux.json"));
        assert!(cli.bypass_cache);
        assert!(cli.watch);
        assert_eq!(cli.log_level, "debug");
    }

    #[test]
    fn test_cli_validation_no_files() {
        let cli = Cli {
            typst_files: vec![],
            ankiconnect_url: "http://localhost:8765".to_string(),
            verbose: false,
            aux_file: PathBuf::from("ankify.aux.json"),
            bypass_cache: false,
            render: None,
            watch: false,
            log_level: "info".to_string(),
        };

        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_cli_help() {
        // This test ensures the CLI can generate help text without panicking
        let _help = Cli::command().render_help();
    }
}
