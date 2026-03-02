//! Shared diagnostic types and helpers for Perl LSP crates.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents successful parsing or execution.
    Error = 1,
    /// Non-critical issue that should be addressed.
    Warning = 2,
    /// Informational message.
    Information = 3,
    /// Subtle suggestion for improvement.
    Hint = 4,
}

/// A diagnostic message with location and context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Source code range (start, end) where the issue occurs.
    pub range: (usize, usize),
    /// Severity level of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Optional diagnostic code for categorization.
    pub code: Option<String>,
    /// Human-readable description of the issue.
    pub message: String,
    /// Additional context and related information.
    pub related_information: Vec<RelatedInformation>,
    /// Tags for categorizing the diagnostic.
    pub tags: Vec<DiagnosticTag>,
}

/// Related information for a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    /// Location in source code for the related information.
    pub location: (usize, usize),
    /// Description of the related information.
    pub message: String,
}

/// Tags for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    /// Code that is not needed and can be removed.
    Unnecessary,
    /// Code that uses deprecated features.
    Deprecated,
}

/// De-duplicate diagnostics to avoid reporting the same issue twice.
///
/// Sorts diagnostics by range, severity, code, and message,
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
    use super::{Diagnostic, DiagnosticSeverity, deduplicate_diagnostics};

    #[test]
    fn removes_exact_duplicates_only() {
        let mut diagnostics = vec![
            Diagnostic {
                range: (1, 2),
                severity: DiagnosticSeverity::Warning,
                code: Some("same".to_string()),
                message: "duplicate".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            },
            Diagnostic {
                range: (1, 2),
                severity: DiagnosticSeverity::Warning,
                code: Some("same".to_string()),
                message: "duplicate".to_string(),
                related_information: vec![],
                tags: vec![],
            },
            Diagnostic {
                range: (1, 2),
                severity: DiagnosticSeverity::Warning,
                code: Some("same".to_string()),
                message: "distinct".to_string(),
                related_information: Vec::new(),
                tags: Vec::new(),
            },
        ];

        deduplicate_diagnostics(&mut diagnostics);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].message, "distinct");
        assert_eq!(diagnostics[1].message, "duplicate");
    }
}
