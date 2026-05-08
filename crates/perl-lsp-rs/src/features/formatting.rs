//! Formatting compatibility shim for LSP
//!
//! This module provides a `CodeFormatter` wrapper that uses the default OS subprocess runtime
//! for backward compatibility with code that expects `CodeFormatter::new()`.

use crate::convert::WireRange;
pub use perl_lsp_rs_core::providers::formatting::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingError,
    FormattingOptions, FormattingProvider, PerlTidyConfig,
};
use perl_lsp_rs_core::tooling::OsSubprocessRuntime;
use perl_lsp_rs_core::tooling::perltidy::FormatterMode;

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
        Self { inner: FormattingProvider::new(OsSubprocessRuntime::with_timeout(10)) }
    }

    /// Create a new code formatter with perltidy configuration
    pub fn with_config(config: PerlTidyConfig) -> Self {
        Self::with_config_and_mode(config, FormatterMode::Native)
    }

    /// Create a new code formatter with perltidy configuration and explicit engine mode.
    pub fn with_config_and_mode(config: PerlTidyConfig, mode: FormatterMode) -> Self {
        let timeout = config.timeout_secs;
        Self {
            inner: FormattingProvider::new(OsSubprocessRuntime::with_timeout(timeout))
                .with_perltidy_config(config)
                .with_formatter_mode(mode),
        }
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
