//! Diagnostic de-duplication utilities for Perl LSP crates.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_lsp_diagnostic_types::Diagnostic;

/// De-duplicate diagnostics to avoid reporting the same issue twice.
///
/// This function sorts diagnostics by range, severity, code, and message,
/// then removes exact duplicates (same range, severity, code, and message).
pub fn deduplicate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.sort_by(|a, b| {
        a.range
            .0
            .cmp(&b.range.0)
            .then(a.range.1.cmp(&b.range.1))
            .then(a.severity.cmp(&b.severity))
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });

    diagnostics.dedup_by(|a, b| {
        a.range == b.range && a.severity == b.severity && a.code == b.code && a.message == b.message
    });
}
