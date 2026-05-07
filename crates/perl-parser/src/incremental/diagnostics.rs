use lsp_types::Diagnostic;
use std::ops::Range;

/// Result of incremental reparse
#[derive(Debug)]
#[non_exhaustive]
pub struct ReparseResult {
    pub changed_ranges: Vec<Range<usize>>,
    pub diagnostics: Vec<Diagnostic>,
    pub reparsed_bytes: usize,
    pub reused_tokens: usize,
    pub token_count: usize,
}
