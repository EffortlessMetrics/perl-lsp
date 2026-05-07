//! Deprecated syntax lint checks
//!
//! This module provides functionality for detecting deprecated Perl syntax
//! and generating appropriate diagnostic warnings.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `deprecated-defined` | Warning | `defined(@array)` or `defined(%hash)` is deprecated |
//! | `deprecated-array-base` | Warning | `$[` variable is deprecated since Perl 5.12 |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::{Diagnostic, DiagnosticTag, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Check for deprecated syntax
///
/// This function walks the AST looking for deprecated Perl syntax such as:
/// - `defined @array` or `defined %hash`
/// - Use of `$[` variable
pub fn check_deprecated_syntax(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    walk_node(node, &mut |n| {
        match &n.kind {
            // Check for deprecated 'defined @array' or 'defined %hash'
            NodeKind::FunctionCall { name, args } => {
                if name == "defined"
                    && let Some(arg) = args.first()
                    && let NodeKind::Variable { sigil, name } = &arg.kind
                    && (sigil == "@" || sigil == "%")
                {
                    let type_name = if sigil == "@" { "array" } else { "hash" };
                    diagnostics.push(Diagnostic {
                        range: (n.location.start, n.location.end),
                        severity: DiagnosticSeverity::Warning,
                        code: Some(DiagnosticCode::DeprecatedDefined.as_str().to_string()),
                        message: format!(
                            "Use of 'defined {}{}' is deprecated",
                            sigil, name
                        ),
                        related_information: vec![
                            RelatedInformation {
                                location: (arg.location.start, arg.location.end),
                                message: format!("💡 Use 'if ({}{})'  or 'if ({}{}[0])' instead", sigil, name, sigil, name),
                            },
                            RelatedInformation {
                                location: (n.location.start, n.location.end),
                                message: format!("ℹ️ Testing definedness of {} is deprecated because it was rarely useful and often wrong. Empty {}s are false in boolean context.", type_name, type_name),
                            }
                        ],
                        tags: vec![DiagnosticTag::Deprecated],
                        suggestion: Some(format!("Replace with 'if ({}{})'", sigil, name)),
                    });
                }
            }

            // Check for deprecated $[ variable
            NodeKind::Variable { sigil, name } => {
                if sigil == "$" && name == "[" {
                    diagnostics.push(Diagnostic {
                        range: (n.location.start, n.location.start + 2),
                        severity: DiagnosticSeverity::Warning,
                        code: Some(DiagnosticCode::DeprecatedArrayBase.as_str().to_string()),
                        message: "Use of '$[' is deprecated and will be removed".to_string(),
                        related_information: vec![
                            RelatedInformation {
                                location: (n.location.start, n.location.start + 2),
                                message: "💡 Remove usage of '$[' - arrays always start at index 0".to_string(),
                            },
                            RelatedInformation {
                                location: (n.location.start, n.location.start + 2),
                                message: "ℹ️ The $[ variable was used to change the base index of arrays, but this feature has been deprecated since Perl 5.12 and will be removed in future versions.".to_string(),
                            }
                        ],
                        tags: vec![DiagnosticTag::Deprecated],
                        suggestion: Some("Remove '$[' -- arrays always start at index 0".to_string()),
                    });
                }
            }

            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn deprecated_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_deprecated_syntax(&ast, &mut diags);
        diags
    }

    fn has_code(diags: &[Diagnostic], code: &str) -> bool {
        diags.iter().any(|d| d.code.as_deref() == Some(code))
    }

    // --- defined(@array) / defined(%hash) ---

    #[test]
    fn defined_array_is_flagged() {
        let diags = deprecated_diags("my @arr = (1,2); if (defined @arr) { }");
        assert!(
            has_code(&diags, "PL500"),
            "defined @arr should be flagged as PL500: {diags:?}"
        );
    }

    #[test]
    fn defined_hash_is_flagged() {
        let diags = deprecated_diags("my %h = (a => 1); if (defined %h) { }");
        assert!(
            has_code(&diags, "PL500"),
            "defined %h should be flagged as PL500: {diags:?}"
        );
    }

    #[test]
    fn defined_scalar_is_not_flagged() {
        let diags = deprecated_diags("my $x; if (defined $x) { }");
        assert!(
            !has_code(&diags, "PL500"),
            "defined $x should NOT be flagged as PL500: {diags:?}"
        );
    }

    #[test]
    fn defined_string_literal_not_flagged() {
        let diags = deprecated_diags(r#"if (defined "hello") { }"#);
        assert!(
            !has_code(&diags, "PL500"),
            "defined on string literal should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn defined_array_diagnostic_has_deprecated_tag() {
        let diags = deprecated_diags("my @arr = (); defined @arr;");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL500"));
        assert!(diag.is_some(), "expected PL500 diagnostic");
        let diag = diag.unwrap();
        assert!(
            diag.tags.contains(&DiagnosticTag::Deprecated),
            "PL500 should carry the Deprecated tag"
        );
    }

    #[test]
    fn defined_hash_diagnostic_message_mentions_hash() {
        let diags = deprecated_diags("my %h = (); defined %h;");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL500")).unwrap();
        assert!(
            diag.message.contains("%h"),
            "message should mention the hash variable: {}", diag.message
        );
    }

    #[test]
    fn defined_array_diagnostic_suggestion_present() {
        let diags = deprecated_diags("my @a = (); defined @a;");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL500")).unwrap();
        assert!(
            diag.suggestion.is_some(),
            "PL500 should carry a suggestion"
        );
    }

    // --- $[ deprecated array base ---

    #[test]
    fn array_base_variable_is_flagged() {
        let diags = deprecated_diags("my $base = $[;");
        assert!(
            has_code(&diags, "PL501"),
            "use of $[ should be flagged as PL501: {diags:?}"
        );
    }

    #[test]
    fn array_base_assignment_is_flagged() {
        let diags = deprecated_diags("$[ = 1;");
        assert!(
            has_code(&diags, "PL501"),
            "assignment to $[ should be flagged as PL501: {diags:?}"
        );
    }

    #[test]
    fn array_base_diagnostic_has_deprecated_tag() {
        let diags = deprecated_diags("my $x = $[;");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL501")).unwrap();
        assert!(
            diag.tags.contains(&DiagnosticTag::Deprecated),
            "PL501 should carry the Deprecated tag"
        );
    }

    #[test]
    fn normal_array_index_not_flagged() {
        let diags = deprecated_diags("my @a = (1,2,3); my $x = $a[0];");
        assert!(
            !has_code(&diags, "PL501"),
            "$a[0] should not be flagged as PL501: {diags:?}"
        );
    }

    #[test]
    fn clean_code_no_deprecated_diagnostics() {
        let diags = deprecated_diags(
            "use strict;\nuse warnings;\nmy @arr = (1,2);\nif (@arr) { print 'ok'; }\n",
        );
        assert!(
            !has_code(&diags, "PL500") && !has_code(&diags, "PL501"),
            "clean code should produce no deprecated diagnostics: {diags:?}"
        );
    }
}
