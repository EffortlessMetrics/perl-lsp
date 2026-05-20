//! Diagnostic construction for undefined `goto LABEL` targets.

use crate::providers::diagnostics::internal_types::{Diagnostic, RelatedInformation};
use perl_diagnostics::codes::{DiagnosticCode, DiagnosticSeverity};
use perl_parser_core::ast::Node;

pub(crate) fn undefined_label(target: &Node, label: &str) -> Diagnostic {
    Diagnostic {
        range: (target.location.start, target.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::GotoUndefinedLabel.as_str().to_string()),
        message: format!("Goto label '{label}' is not defined in this file"),
        related_information: vec![RelatedInformation {
            location: (target.location.start, target.location.end),
            message: "Define the label or use a dynamic goto form only when the target is known at runtime.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(format!("Add a '{label}:' label or remove the goto")),
    }
}
