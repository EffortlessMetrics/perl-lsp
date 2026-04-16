//! Internal diagnostic types for perl-lsp-diagnostics.
//!
//! These types are the working types used by this crate's linting machinery.
//! The canonical public API types (`DiagnosticCode`, `DiagnosticSeverity`, `DiagnosticTag`)
//! are re-exported from `perl-diagnostics::codes::`.

use perl_diagnostics::codes::DiagnosticSeverity;

/// Tags for diagnostics (internal alias for the canonical type from codes::).
pub use perl_diagnostics::codes::DiagnosticTag;

/// A diagnostic message (internal working type).
///
/// This is the rich internal type used by the linting machinery.
/// It has string-based codes for compatibility with the diagnostic pipeline.
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
    /// Optional short suggestion for how to fix the issue.
    pub suggestion: Option<String>,
}

/// Related information for a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    /// Location in source code for the related information.
    pub location: (usize, usize),
    /// Description of the related information.
    pub message: String,
}
