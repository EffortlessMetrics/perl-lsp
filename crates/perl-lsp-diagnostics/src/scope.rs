//! Scope analyzer issue to diagnostic conversion
//!
//! This module provides functionality for converting scope analyzer issues
//! into diagnostic messages with pragma-aware severity mapping.

use perl_diagnostics_codes::DiagnosticCode;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};

use perl_lsp_diagnostic_types::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation,
};

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
            | IssueKind::UninitializedVariable
            | IssueKind::UndeclaredSubroutine => DiagnosticSeverity::Warning,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticSeverity::Information,
        };

        let code = match issue.kind {
            IssueKind::UndeclaredVariable => DiagnosticCode::UndefinedVariable,
            IssueKind::UnusedVariable => DiagnosticCode::UnusedVariable,
            IssueKind::VariableShadowing => DiagnosticCode::VariableShadowing,
            IssueKind::VariableRedeclaration => DiagnosticCode::VariableRedeclaration,
            IssueKind::DuplicateParameter => DiagnosticCode::DuplicateParameter,
            IssueKind::ParameterShadowsGlobal => DiagnosticCode::ParameterShadowsGlobal,
            IssueKind::UnusedParameter => DiagnosticCode::UnusedParameter,
            IssueKind::UnquotedBareword => DiagnosticCode::UnquotedBareword,
            IssueKind::UninitializedVariable => DiagnosticCode::UninitializedVariable,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticCode::CaptureVarWithoutRegexMatch,
            IssueKind::UndeclaredSubroutine => DiagnosticCode::UndeclaredSubroutine,
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
            IssueKind::CaptureVarWithoutRegexMatch => vec![
                RelatedInformation {
                    location: issue.range,
                    message: "💡 Perform a regex match before using this capture variable: if ($str =~ /(...)/){ ... }".to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message: "ℹ️ Capture variables ($1, $2, etc.) hold the last successful match and may be undef if no match has occurred.".to_string(),
                }
            ],
            IssueKind::UndeclaredSubroutine => vec![
                RelatedInformation {
                    location: issue.range,
                    message:
                        "💡 Check the spelling of the sub name, or define it with `sub NAME { ... }` in the target package."
                            .to_string(),
                },
                RelatedInformation {
                    location: issue.range,
                    message:
                        "ℹ️ Under `use strict 'subs'`, a package-qualified call must resolve to a known subroutine. This diagnostic fires only when the target package is defined in this file and has no inheritance, AUTOLOAD, or dynamic dispatch markers."
                            .to_string(),
                }
            ],
        };

        let suggestion = build_scope_suggestion(&issue);

        diagnostics.push(Diagnostic {
            range: issue.range,
            severity,
            code: Some(code.as_str().to_string()),
            message: build_enhanced_scope_message(&issue),
            related_information: related_info,
            tags: if matches!(issue.kind, IssueKind::UnusedVariable | IssueKind::UnusedParameter) {
                vec![DiagnosticTag::Unnecessary]
            } else {
                Vec::new()
            },
            suggestion,
        });
    }

    diagnostics
}

/// Build an enhanced, more helpful message for a scope issue.
///
/// Augments the analyzer's raw description with the variable name and
/// actionable context so users immediately understand what went wrong.
fn build_enhanced_scope_message(issue: &ScopeIssue) -> String {
    let name = &issue.variable_name;
    match issue.kind {
        IssueKind::UndeclaredVariable => {
            format!(
                "Variable '{}' is used but not declared -- add 'my {}' to declare it in this scope",
                name, name
            )
        }
        IssueKind::UnusedVariable => {
            format!(
                "Variable '{}' is declared but never used -- prefix with '_' or remove it",
                name
            )
        }
        IssueKind::UnusedParameter => {
            format!(
                "Parameter '{}' is never used -- prefix with '_' (e.g., $_{}) to suppress this warning",
                name,
                name.trim_start_matches('$')
            )
        }
        IssueKind::VariableShadowing => {
            format!(
                "Variable '{}' shadows an outer declaration -- consider renaming to avoid confusion",
                name
            )
        }
        IssueKind::VariableRedeclaration => {
            format!(
                "Variable '{}' is declared again in the same scope -- remove the duplicate 'my'",
                name
            )
        }
        IssueKind::UninitializedVariable => {
            format!(
                "Variable '{}' is used before being initialized -- assign a value when declaring it",
                name
            )
        }
        IssueKind::UnquotedBareword => {
            format!(
                "Bareword '{}' is not allowed under 'use strict' -- quote it as '{}' or use it as a subroutine call",
                name, name
            )
        }
        // Fall back to the analyzer's original description for other kinds
        _ => issue.description.clone(),
    }
}

/// Build a short actionable fix suggestion for a scope issue.
fn build_scope_suggestion(issue: &ScopeIssue) -> Option<String> {
    let name = &issue.variable_name;
    match issue.kind {
        IssueKind::UndeclaredVariable => Some(format!("Add 'my {};' before this line", name)),
        IssueKind::UnusedVariable => Some(format!("Prefix as '_{}'", name.trim_start_matches('$'))),
        IssueKind::UnusedParameter => {
            Some(format!("Rename to '$_{}'", name.trim_start_matches('$')))
        }
        IssueKind::VariableRedeclaration => Some("Remove the duplicate 'my' keyword".to_string()),
        IssueKind::UninitializedVariable => Some(format!("Initialize: my {} = ...;", name)),
        IssueKind::UnquotedBareword => {
            Some(format!("Quote as '{}' or use qw({}) for lists", name, name))
        }
        _ => None,
    }
}
