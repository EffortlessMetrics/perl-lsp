//! Scope analyzer issue to diagnostic conversion
//!
//! This module provides functionality for converting scope analyzer issues
//! into diagnostic messages with pragma-aware severity mapping.

use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};

use super::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};

/// Convert scope analyzer issues to diagnostics
///
/// This function processes scope analyzer issues and converts them into
/// appropriate diagnostics with severity levels, codes, and helpful related
/// information based on the issue type.
pub fn scope_issues_to_diagnostics(issues: Vec<ScopeIssue>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for issue in issues {
        let severity = match issue.kind {
            IssueKind::UndeclaredVariable
            | IssueKind::VariableRedeclaration
            | IssueKind::DuplicateParameter
            | IssueKind::UnquotedBareword => DiagnosticSeverity::Error,
            IssueKind::VariableShadowing
            | IssueKind::UnusedVariable
            | IssueKind::ParameterShadowsGlobal
            | IssueKind::UnusedParameter
            | IssueKind::UninitializedVariable => DiagnosticSeverity::Warning,
        };

        let code = match issue.kind {
            IssueKind::UndeclaredVariable => "undeclared-variable",
            IssueKind::UnusedVariable => "unused-variable",
            IssueKind::VariableShadowing => "variable-shadowing",
            IssueKind::VariableRedeclaration => "variable-redeclaration",
            IssueKind::DuplicateParameter => "duplicate-parameter",
            IssueKind::ParameterShadowsGlobal => "parameter-shadows-global",
            IssueKind::UnusedParameter => "unused-parameter",
            IssueKind::UnquotedBareword => "unquoted-bareword",
            IssueKind::UninitializedVariable => "uninitialized-variable",
        };

        // Build helpful related information based on issue type
        let related_info = match issue.kind {
            IssueKind::UndeclaredVariable => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Declare the variable with 'my', 'our', 'local', or 'state'".to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message: "ℹ️ Under 'use strict', all variables must be declared before use. Use 'my' for lexical scope or 'our' for package variables.".to_string(),
                }
            ],
            IssueKind::UnusedVariable => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Remove the unused variable or prefix with '_' to indicate it's intentionally unused".to_string(),
                }
            ],
            IssueKind::UnusedParameter => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Remove the unused parameter or prefix with '_' (e.g., $_unused) to indicate it's intentionally unused".to_string(),
                }
            ],
            IssueKind::VariableShadowing => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Rename this variable or use the outer scope variable instead".to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message: "ℹ️ Variable shadowing can make code harder to understand and may hide bugs.".to_string(),
                }
            ],
            IssueKind::VariableRedeclaration => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Remove the duplicate 'my' declaration - just assign to the existing variable".to_string(),
                }
            ],
            IssueKind::DuplicateParameter => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Remove the duplicate parameter or use a different name".to_string(),
                }
            ],
            IssueKind::ParameterShadowsGlobal => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Rename the parameter to avoid shadowing the global variable".to_string(),
                }
            ],
            IssueKind::UninitializedVariable => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Initialize the variable when declaring it: my $var = value;".to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message: "ℹ️ Using uninitialized variables may cause warnings and unexpected behavior.".to_string(),
                }
            ],
            IssueKind::UnquotedBareword => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Quote the bareword as a string: 'word' or \"word\"".to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message: "ℹ️ Under 'use strict', barewords are not allowed unless they're subroutine calls or hash keys.".to_string(),
                }
            ],
        };

        diagnostics.push(Diagnostic {
            range: issue.range,
            severity,
            code: Some(code.to_string()),
            message: issue.description.clone(),
            related_information: related_info,
            tags: if matches!(issue.kind, IssueKind::UnusedVariable | IssueKind::UnusedParameter) {
                vec![DiagnosticTag::Unnecessary]
            } else {
                Vec::new()
            },
        });
    }

    diagnostics
}
