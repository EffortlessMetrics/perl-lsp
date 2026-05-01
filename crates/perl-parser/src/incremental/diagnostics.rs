use lsp_types::Diagnostic;
use std::ops::Range;

/// Result of incremental reparse
#[derive(Debug)]
pub struct ReparseResult {
    pub changed_ranges: Vec<Range<usize>>,
    pub diagnostics: Vec<Diagnostic>,
    pub reparsed_bytes: usize,
}
