//! Formatting compatibility shim for LSP
//!
//! This module provides a `CodeFormatter` wrapper that uses the default OS subprocess runtime
//! for backward compatibility with code that expects `CodeFormatter::new()`.

use crate::convert::WireRange;
pub use perl_lsp_formatting::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingError,
    FormattingOptions, FormattingProvider,
};
use perl_lsp_tooling::OsSubprocessRuntime;

/// Code formatter using the OS subprocess runtime
///
/// This is a compatibility wrapper that provides a `new()` method with no arguments
/// for code that expects the old `CodeFormatter` API.
pub struct CodeFormatter {
    inner: FormattingProvider<OsSubprocessRuntime>,
}

impl CodeFormatter {
    /// Create a new code formatter with the default OS subprocess runtime
    pub fn new() -> Self {
        Self { inner: FormattingProvider::new(OsSubprocessRuntime::new()) }
    }

    /// Configure the formatter with a perltidy configuration file path
    pub fn with_config_path(mut self, path: String) -> Self {
        self.inner = self.inner.with_config_path(path);
        self
    }

    /// Format an entire document, returning just the edits for backwards compatibility
    pub fn format_document(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormattingError> {
        let doc = self.inner.format_document(content, options)?;
        Ok(doc.edits)
    }

    /// Format a specific range, returning just the edits for backwards compatibility
    ///
    /// Accepts WireRange (from perl-position-tracking) and converts it to FormatRange.
    pub fn format_range(
        &self,
        content: &str,
        range: &WireRange,
        options: &FormattingOptions,
    ) -> Result<Vec<FormatTextEdit>, FormattingError> {
        // Convert WireRange to FormatRange
        let format_range = FormatRange {
            start: FormatPosition { line: range.start.line, character: range.start.character },
            end: FormatPosition { line: range.end.line, character: range.end.character },
        };
        let doc = self.inner.format_range(content, &format_range, options)?;
        Ok(doc.edits)
    }
}

impl Default for CodeFormatter {
    fn default() -> Self {
        Self::new()
    }
}
