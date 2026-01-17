//! Code formatting support using Perl::Tidy for Perl parsing workflow pipeline
//!
//! This module provides integration with perltidy for code formatting of Perl scripts
//! throughout the LSP workflow, ensuring consistent code style and readability for
//! large-scale Perl parsing operations.
//!
//! # LSP Workflow Integration
//!
//! Formatting operations are integrated across LSP workflow stages:
//! - **Extract**: Format Perl scripts during initial processing for consistency
//! - **Normalize**: Apply standardized formatting rules to Perl parsing code
//! - **Thread**: Maintain readable formatting during control flow analysis
//! - **Render**: Ensure consistent output formatting for processed Perl scripts
//! - **Index**: Generate consistently formatted code for indexing and search
//!
//! # Performance Characteristics
//!
//! Optimized for enterprise-scale Perl script formatting:
//! - **large Perl codebase Support**: Efficient handling of large Perl script collections
//! - **Incremental Formatting**: Format only changed code sections for performance
//! - **Graceful Degradation**: Continues operation even when perltidy is unavailable
//! - **Memory Efficient**: Streams large files to minimize memory usage during formatting
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::perltidy::{PerlTidyFormatter, PerlTidyConfig};
//! use perl_parser::formatting::FormattingOptions;
//!
//! // Format Perl parsing script with standard options
//! let script = "sub process_email{my$msg=shift;return$msg;}";
//! let _options = FormattingOptions {
//!     tab_size: 4,
//!     insert_spaces: true,
//!     trim_trailing_whitespace: Some(true),
//!     insert_final_newline: Some(true),
//!     trim_final_newlines: Some(true),
//! };
//! let mut formatter = PerlTidyFormatter::with_os_runtime(PerlTidyConfig::default());
//!
//! match formatter.format(script) {
//!     Ok(formatted_text) => {
//!         // Apply formatting to Perl script
//!         println!("Formatted text: {}", formatted_text);
//!     }
//!     Err(e) => {
//!         // Handle formatting errors gracefully
//!         eprintln!("Formatting failed: {}", e);
//!     }
//! }
//! ```

use crate::convert::{WirePosition, WireRange};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};

/// Text edit for formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatTextEdit {
    /// The range to replace
    pub range: WireRange,
    /// The new text
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// Formatting options
#[derive(Debug, Clone, Deserialize)]
pub struct FormattingOptions {
    /// Size of a tab in spaces
    #[serde(rename = "tabSize")]
    pub tab_size: u32,
    /// Prefer spaces over tabs
    #[serde(rename = "insertSpaces")]
    pub insert_spaces: bool,
    /// Trim trailing whitespace on a line
    #[serde(rename = "trimTrailingWhitespace")]
    pub trim_trailing_whitespace: Option<bool>,
    /// Insert a newline character at the end of the file
    #[serde(rename = "insertFinalNewline")]
    pub insert_final_newline: Option<bool>,
    /// Trim all newlines after the final newline at the end of the file
    #[serde(rename = "trimFinalNewlines")]
    pub trim_final_newlines: Option<bool>,
}

/// Code formatter using perltidy
pub struct CodeFormatter;

impl CodeFormatter {
    /// Create a new code formatter for Perl script processing
    ///
    /// Constructs a formatter instance capable of formatting Perl Perl scripts
    /// according to best practices and coding standards within LSP workflow
    /// development workflows.
    ///
    /// # Returns
    ///
    /// A configured formatter ready for Perl script formatting operations
    /// with perltidy integration and graceful fallback handling.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::CodeFormatter;
    ///
    /// let formatter = CodeFormatter::new();
    /// // Formatter ready for Perl script formatting
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Format the entire Perl script document with perltidy integration
    ///
    /// Performs comprehensive formatting of Perl script content using perltidy
    /// with graceful fallback handling for environments where perltidy is not
    /// available. Optimized for Perl parsing workflow development workflows.
    ///
    /// # Arguments
    ///
    /// * `content` - Email script source code to format
    /// * `options` - Formatting configuration including indentation and style preferences
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<FormatTextEdit>)` - Text edits to apply formatted changes
    /// * `Err(FormatError)` - When formatting fails or perltidy is unavailable
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::{CodeFormatter, FormattingOptions};
    ///
    /// let formatter = CodeFormatter::new();
    /// let script = "my$email_count=scalar(@emails);print$email_count;";
    /// let options = FormattingOptions {
    ///     tab_size: 4,
    ///     insert_spaces: true,
    ///     trim_trailing_whitespace: Some(true),
    ///     insert_final_newline: Some(true),
    ///     trim_final_newlines: Some(true),
    /// };
    ///
    /// match formatter.format_document(script, &options) {
    ///     Ok(edits) => {
    ///         // Apply formatting edits for cleaner Perl script
    ///         println!("Formatted with {} edits", edits.len());
    ///     }
    ///     Err(e) => {
    ///         // Handle formatting errors gracefully
    ///         eprintln!("Formatting failed: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Error Recovery
    ///
    /// This function provides graceful degradation when perltidy is not available,
    /// ensuring Perl script development can continue with manual formatting.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn format_document(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormatError> {
        // Format using perltidy
        let formatted = self.run_perltidy(content, options)?;

        // If nothing changed, return empty edits
        if formatted == content {
            return Ok(vec![]);
        }

        // Return a single edit that replaces the entire document
        // Use whole_document to correctly compute UTF-16 end position
        Ok(vec![FormatTextEdit { range: WireRange::whole_document(content), new_text: formatted }])
    }

    /// Format a specific range in the document
    #[cfg(not(target_arch = "wasm32"))]
    pub fn format_range(
        &self,
        content: &str,
        range: &WireRange,
        options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormatError> {
        // Extract the lines to format
        let lines: Vec<&str> = content.lines().collect();
        let start_line = range.start.line as usize;
        let end_line = (range.end.line as usize).min(lines.len().saturating_sub(1));

        if start_line >= lines.len() {
            return Ok(vec![]);
        }

        // Get the text to format
        let text_to_format = lines[start_line..=end_line].join("\n");

        // Format using perltidy
        let formatted = self.run_perltidy(&text_to_format, options)?;

        // If nothing changed, return empty edits
        if formatted == text_to_format {
            return Ok(vec![]);
        }

        // Calculate the range to replace
        // Note: We always format whole lines, so start_char is 0 and
        // end_char is the UTF-16 length of the last line (not byte length!)
        let start_char = 0;
        let end_char =
            crate::util::byte_to_utf16_col(lines[end_line], lines[end_line].len()) as u32;

        Ok(vec![FormatTextEdit {
            range: WireRange {
                start: WirePosition::new(start_line as u32, start_char),
                end: WirePosition::new(end_line as u32, end_char),
            },
            new_text: formatted,
        }])
    }

    /// Find perltidy command in various locations
    #[cfg(not(target_arch = "wasm32"))]
    fn find_perltidy_command(&self) -> String {
        // First try the PATH
        if self.command_exists("perltidy") {
            return "perltidy".to_string();
        }

        // Common locations to check
        let common_paths = [
            "/usr/bin/perltidy",
            "/usr/local/bin/perltidy",
            "/opt/local/bin/perltidy",          // MacPorts
            "/usr/local/opt/perl/bin/perltidy", // Homebrew
        ];

        for path in &common_paths {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }

        // Check perlbrew
        if let Ok(home) = std::env::var("HOME") {
            let perlbrew_path = format!("{}/.perlbrew/perls/current/bin/perltidy", home);
            if Path::new(&perlbrew_path).exists() {
                return perlbrew_path;
            }
        }

        // Default to perltidy and let it fail with helpful error
        "perltidy".to_string()
    }

    /// Check if a command exists
    #[cfg(not(target_arch = "wasm32"))]
    fn command_exists(&self, cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Find .perltidyrc file in the workspace
    #[cfg(not(target_arch = "wasm32"))]
    fn find_perltidyrc(&self, starting_path: Option<&Path>) -> Option<PathBuf> {
        let start = starting_path.unwrap_or(Path::new("."));
        let mut current = start;

        loop {
            let perltidyrc = current.join(".perltidyrc");
            if perltidyrc.exists() {
                return Some(perltidyrc);
            }

            // Check parent directory
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        // Check home directory
        if let Ok(home) = std::env::var("HOME") {
            let home_perltidyrc = Path::new(&home).join(".perltidyrc");
            if home_perltidyrc.exists() {
                return Some(home_perltidyrc);
            }
        }

        None
    }

    /// Run perltidy on the given text
    #[cfg(not(target_arch = "wasm32"))]
    fn run_perltidy(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<String, FormatError> {
        self.run_perltidy_with_config(content, options, None)
    }

    /// Run perltidy with optional config file
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_perltidy_with_config(
        &self,
        content: &str,
        options: &FormattingOptions,
        workspace_path: Option<&Path>,
    ) -> Result<String, FormatError> {
        // Build perltidy arguments
        let mut args = vec![
            "-st".to_string(), // Output to stdout
            "-se".to_string(), // Errors to stderr
        ];

        // Check for .perltidyrc file
        if let Some(config_path) = self.find_perltidyrc(workspace_path) {
            eprintln!("Using .perltidyrc from: {:?}", config_path);
            args.push(format!("-pro={}", config_path.display()));
        } else {
            args.push("-npro".to_string()); // Don't read .perltidyrc
        }

        // Add formatting options
        if options.insert_spaces {
            args.push(format!("-et={}", options.tab_size)); // Expand tabs
            args.push(format!("-i={}", options.tab_size)); // Indent size
        } else {
            args.push("-dt".to_string()); // Use tabs
            args.push(format!("-i={}", options.tab_size)); // Tab size
        }

        // Try to find perltidy in various locations
        let perltidy_cmd = self.find_perltidy_command();

        // Try to run perltidy
        let mut child = Command::new(&perltidy_cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| FormatError::PerltidyNotFound(e.to_string()))?;

        // Write input
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes()).map_err(|e| FormatError::IoError(e.to_string()))?;
        }

        // Get output
        let output = child.wait_with_output().map_err(|e| FormatError::IoError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FormatError::PerltidyError(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Format the entire document (unsupported on wasm32).
    #[cfg(target_arch = "wasm32")]
    pub fn format_document(
        &self,
        _content: &str,
        _options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormatError> {
        Err(perltidy_unavailable())
    }

    /// Format a document range (unsupported on wasm32).
    #[cfg(target_arch = "wasm32")]
    pub fn format_range(
        &self,
        _content: &str,
        _range: &WireRange,
        _options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormatError> {
        Err(perltidy_unavailable())
    }

    /// Run perltidy with optional config (unsupported on wasm32).
    #[cfg(target_arch = "wasm32")]
    pub fn run_perltidy_with_config(
        &self,
        _content: &str,
        _options: &FormattingOptions,
        _workspace_path: Option<&Path>,
    ) -> Result<String, FormatError> {
        Err(perltidy_unavailable())
    }
}

#[cfg(target_arch = "wasm32")]
fn perltidy_unavailable() -> FormatError {
    FormatError::PerltidyNotFound("perltidy is not available on wasm32 targets".to_string())
}

/// Formatting error
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error(
        "perltidy not found: {0}\n\nTo install perltidy:\n  - CPAN: cpan Perl::Tidy\n  - Debian/Ubuntu: apt-get install perltidy\n  - RedHat/Fedora: yum install perltidy\n  - macOS: brew install perltidy\n  - Windows: cpan Perl::Tidy"
    )]
    /// perltidy executable not found on system PATH
    PerltidyNotFound(String),

    /// Error occurred during perltidy execution
    #[error("perltidy error: {0}")]
    PerltidyError(String),

    /// I/O error during file operations
    #[error("IO error: {0}")]
    IoError(String),
}

impl Default for CodeFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_options() {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        assert_eq!(options.tab_size, 4);
        assert!(options.insert_spaces);
    }

    // Integration test (requires perltidy to be installed)
    #[test]
    fn test_format_simple_code() {
        // Self-skip if perltidy is not available
        if std::process::Command::new("perltidy").arg("--version").output().is_err() {
            eprintln!("SKIP: perltidy not installed, skipping formatting integration test");
            return;
        }

        let formatter = CodeFormatter::new();
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        };

        let code = "sub test{my$x=1;print$x;}";
        let result = formatter.format_document(code, &options);

        assert!(result.is_ok());
        let edits = result.unwrap();
        assert!(!edits.is_empty());

        // The formatted code should have proper spacing
        let formatted = &edits[0].new_text;
        assert!(formatted.contains("sub test"));
        assert!(formatted.contains("my $x"));
    }
}
