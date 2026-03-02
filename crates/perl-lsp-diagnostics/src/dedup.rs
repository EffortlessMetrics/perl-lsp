//! Diagnostic deduplication
//!
//! This module provides functionality for removing duplicate diagnostics
//! to avoid reporting the same issue multiple times.

use super::types::Diagnostic;
use perl_lsp_dedup::sort_and_dedup_by;

/// De-duplicate diagnostics to avoid reporting the same issue twice
///
/// This function sorts diagnostics by range, severity, code, and message,
/// then removes exact duplicates (same range, severity, code, and message).
#[allow(dead_code)]
pub fn deduplicate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    sort_and_dedup_by(
        diagnostics,
        |a, b| {
            a.range
                .0
                .cmp(&b.range.0)
                .then(a.range.1.cmp(&b.range.1))
                .then(a.severity.cmp(&b.severity))
                .then(a.code.cmp(&b.code))
                .then(a.message.cmp(&b.message))
        },
        |a, b| {
            a.range == b.range
                && a.severity == b.severity
                && a.code == b.code
                && a.message == b.message
        },
    );
}
