//! Diagnostic deduplication
//!
//! This module provides functionality for removing duplicate diagnostics
//! to avoid reporting the same issue multiple times.

use super::types::Diagnostic;

/// De-duplicate diagnostics to avoid reporting the same issue twice
///
/// This function sorts diagnostics by range, severity, code, and message,
/// then removes exact duplicates (same range, severity, code, and message).
#[allow(dead_code)]
pub fn deduplicate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    // Sort by range, severity, code, and message
    diagnostics.sort_by(|a, b| {
        a.range
            .0
            .cmp(&b.range.0)
            .then(a.range.1.cmp(&b.range.1))
            .then(a.severity.cmp(&b.severity))
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });

    // Remove only exact duplicates (same range, severity, code, and message)
    diagnostics.dedup_by(|a, b| {
        a.range == b.range && a.severity == b.severity && a.code == b.code && a.message == b.message
    });
}
