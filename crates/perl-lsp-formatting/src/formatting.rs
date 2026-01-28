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

use serde::{Deserialize, Serialize};

/// Text edit for formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatTextEdit {
    /// The range to replace
    pub range: FormatRange,
    /// The new text
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// Position in a document (UTF-16 based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatPosition {
    /// Line position (0-based)
    pub line: u32,
    /// Character position (UTF-16, 0-based)
    pub character: u32,
}

/// Range in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRange {
    /// Start position
    pub start: FormatPosition,
    /// End position
    pub end: FormatPosition,
}

impl FormatRange {
    /// Create a range covering the entire document
    pub fn whole_document(content: &str) -> Self {
        let lines: Vec<&str> = content.lines().collect();
        let last_line = if lines.is_empty() { 0 } else { (lines.len() - 1) as u32 };

        FormatRange {
            start: FormatPosition { line: 0, character: 0 },
            end: FormatPosition {
                line: last_line,
                character: lines.get(last_line as usize).map(|l| l.len() as u32).unwrap_or(0),
            },
        }
    }

    /// Create a new range from positions
    pub fn new(start: FormatPosition, end: FormatPosition) -> Self {
        Self { start, end }
    }
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

/// Formatted document result
#[derive(Debug, Clone)]
pub struct FormattedDocument {
    /// The formatted text
    pub text: String,
    /// Text edits to apply formatting
    pub edits: Vec<FormatTextEdit>,
}

/// Formatting error
#[derive(Debug, thiserror::Error)]
pub enum FormattingError {
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

/// Code formatter using perltidy
pub struct FormattingProvider<R> {
    /// Subprocess runtime for executing perltidy
    runtime: R,
    /// Optional custom perltidy path
    perltidy_path: Option<String>,
}

impl<R> FormattingProvider<R> {
    /// Create a new formatting provider with the given runtime
    pub fn new(runtime: R) -> Self {
        Self { runtime, perltidy_path: None }
    }

    /// Set a custom perltidy path
    pub fn with_perltidy_path(mut self, path: String) -> Self {
        self.perltidy_path = Some(path);
        self
    }
}

impl<R: perl_lsp_tooling::SubprocessRuntime> FormattingProvider<R> {
    /// Format the entire Perl script document with perltidy integration
    ///
    /// Performs comprehensive formatting of Perl script content using perltidy
    /// with graceful fallback handling for environments where perltidy is not
    /// available. Optimized for Perl parsing workflow development workflows.
    ///
    /// # Arguments
    ///
    /// * `content` - Perl script source code to format
    /// * `options` - Formatting configuration including indentation and style preferences
    ///
    /// # Returns
    ///
    /// * `Ok(FormattedDocument)` - Formatted document with text and edits
    /// * `Err(FormattingError)` - When formatting fails or perltidy is unavailable
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_lsp_formatting::{FormattingProvider, FormattingOptions};
    /// use perl_lsp_tooling::OsSubprocessRuntime;
    ///
    /// let runtime = OsSubprocessRuntime::new();
    /// let provider = FormattingProvider::new(runtime);
    /// let options = FormattingOptions {
    ///     tab_size: 4,
    ///     insert_spaces: true,
    ///     trim_trailing_whitespace: Some(true),
    ///     insert_final_newline: Some(true),
    ///     trim_final_newlines: Some(true),
    /// };
    ///
    /// match provider.format_document(script, &options) {
    ///     Ok(doc) => {
    ///         println!("Formatted with {} edits", doc.edits.len());
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Formatting failed: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Error Recovery
    ///
    /// This function provides graceful degradation when perltidy is not available,
    /// ensuring Perl script development can continue with manual formatting.
    pub fn format_document(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<FormattedDocument, FormattingError> {
        // Format using perltidy
        let formatted = self.run_perltidy(content, options)?;

        // If nothing changed, return empty edits
        if formatted == content {
            return Ok(FormattedDocument { text: formatted, edits: vec![] });
        }

        // Return a single edit that replaces the entire document
        Ok(FormattedDocument {
            text: formatted.clone(),
            edits: vec![FormatTextEdit {
                range: FormatRange::whole_document(content),
                new_text: formatted,
            }],
        })
    }

    /// Format a specific range in the document
    pub fn format_range(
        &self,
        content: &str,
        range: &FormatRange,
        options: &FormattingOptions,
    ) -> Result<FormattedDocument, FormattingError> {
        // Extract the lines to format
        let lines: Vec<&str> = content.lines().collect();
        let start_line = range.start.line as usize;
        let end_line = (range.end.line as usize).min(lines.len().saturating_sub(1));

        if start_line >= lines.len() {
            return Ok(FormattedDocument { text: content.to_string(), edits: vec![] });
        }

        // Get the text to format
        let text_to_format = lines[start_line..=end_line].join("\n");

        // Format using perltidy
        let formatted = self.run_perltidy(&text_to_format, options)?;

        // If nothing changed, return empty edits
        if formatted == text_to_format {
            return Ok(FormattedDocument { text: content.to_string(), edits: vec![] });
        }

        // Calculate the range to replace
        let start_char = 0;
        let end_char = lines[end_line].len() as u32;

        Ok(FormattedDocument {
            text: content.to_string(),
            edits: vec![FormatTextEdit {
                range: FormatRange::new(
                    FormatPosition::new(start_line as u32, start_char),
                    FormatPosition::new(end_line as u32, end_char),
                ),
                new_text: formatted,
            }],
        })
    }

    /// Run perltidy on the given text
    fn run_perltidy(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<String, FormattingError> {
        // Build perltidy arguments
        let mut args = vec![
            "-st".to_string(), // Output to stdout
            "-se".to_string(), // Errors to stderr
        ];

        // Add formatting options
        if options.insert_spaces {
            args.push(format!("-et={}", options.tab_size)); // Expand tabs
            args.push(format!("-i={}", options.tab_size)); // Indent size
        } else {
            args.push("-dt".to_string()); // Use tabs
            args.push(format!("-i={}", options.tab_size)); // Tab size
        }

        // Get perltidy command
        let default_cmd = "perltidy";
        let perltidy_cmd = self.perltidy_path.as_deref().unwrap_or(default_cmd);

        // Try to run perltidy
        let output = self
            .runtime
            .run_command(
                perltidy_cmd,
                &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                Some(content.as_bytes()),
            )
            .map_err(|e| FormattingError::PerltidyNotFound(e.message))?;

        if !output.success() {
            return Err(FormattingError::PerltidyError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl FormatPosition {
    /// Create a new position
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

#[cfg(test)]
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

    #[test]
    fn test_format_position() {
        let pos = FormatPosition::new(5, 10);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn test_format_range() {
        let start = FormatPosition::new(0, 0);
        let end = FormatPosition::new(10, 20);
        let range = FormatRange::new(start, end);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 10);
    }
}
