use perl_diagnostics::codes::{DiagnosticCode, DiagnosticSeverity};
use perl_parser_core::ast::Node;

use super::super::super::internal_types::{Diagnostic, RelatedInformation};

/// Build the PL408 diagnostic for a duplicate hash key occurrence.
pub(super) fn duplicate_key_diagnostic(
    key: &Node,
    key_text: &str,
    first_occurrence: (usize, usize),
) -> Diagnostic {
    let (first_start, first_end) = first_occurrence;

    Diagnostic {
        range: (key.location.start, key.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::DuplicateHashKey.as_str().to_string()),
        message: format!("Duplicate hash key '{key_text}' -- only the last value will be used"),
        related_information: vec![RelatedInformation {
            location: (first_start, first_end),
            message: format!("Key '{key_text}' first defined here"),
        }],
        tags: Vec::new(),
        suggestion: Some(format!("Remove the earlier '{key_text}' entry or rename this key")),
    }
}
