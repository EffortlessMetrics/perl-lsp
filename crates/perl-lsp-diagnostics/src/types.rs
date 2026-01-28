//! Diagnostic types for Perl code analysis
//!
//! This module defines the core types used for representing diagnostic messages,
//! severity levels, and related information.

/// Severity level for diagnostics
///
/// Represents the importance and type of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents successful parsing or execution
    Error = 1,
    /// Non-critical issue that should be addressed
    Warning = 2,
    /// Informational message
    Information = 3,
    /// Subtle suggestion for improvement
    Hint = 4,
}

/// A diagnostic message
///
/// Represents an issue found during code analysis with location,
/// severity, and additional context information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Source code range (start, end) where the issue occurs
    pub range: (usize, usize),
    /// Severity level of the diagnostic
    pub severity: DiagnosticSeverity,
    /// Optional diagnostic code for categorization
    pub code: Option<String>,
    /// Human-readable description of the issue
    pub message: String,
    /// Additional context and related information
    pub related_information: Vec<RelatedInformation>,
    /// Tags for categorizing the diagnostic
    pub tags: Vec<DiagnosticTag>,
}

/// Related information for a diagnostic
///
/// Additional context that helps understand or resolve the main diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInformation {
    /// Location in source code for the related information
    pub location: (usize, usize),
    /// Description of the related information
    pub message: String,
}

/// Tags for diagnostics
///
/// Additional metadata about the nature of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    /// Code that is not needed and can be removed
    Unnecessary,
    /// Code that uses deprecated features
    Deprecated,
}
