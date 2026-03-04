//! Diagnostic deduplication utilities.

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

#[cfg(test)]
mod tests {
    use super::deduplicate_diagnostics;
    use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};

    #[test]
    fn removes_only_exact_duplicates() {
        let mut diagnostics = vec![
            Diagnostic {
                range: (4, 6),
                severity: DiagnosticSeverity::Warning,
                code: Some("W1".to_string()),
                message: "warning".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (4, 6),
                severity: DiagnosticSeverity::Warning,
                code: Some("W1".to_string()),
                message: "warning".to_string(),
                related_information: vec![],
                tags: vec![],
            },
            Diagnostic {
                range: (4, 6),
                severity: DiagnosticSeverity::Warning,
                code: Some("W2".to_string()),
                message: "warning".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        deduplicate_diagnostics(&mut diagnostics);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code.as_deref(), Some("W1"));
        assert_eq!(diagnostics[1].code.as_deref(), Some("W2"));
    }
}
