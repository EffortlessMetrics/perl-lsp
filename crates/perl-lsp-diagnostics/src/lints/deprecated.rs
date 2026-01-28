//! Deprecated syntax lint checks
//!
//! This module provides functionality for detecting deprecated Perl syntax
//! and generating appropriate diagnostic warnings.

use perl_parser_core::ast::{Node, NodeKind};

use super::super::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};
use super::super::walker::walk_node;

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
                if name == "defined" {
                    if let Some(arg) = args.first() {
                        if let NodeKind::Variable { sigil, name } = &arg.kind {
                            if sigil == "@" || sigil == "%" {
                                let type_name = if sigil == "@" { "array" } else { "hash" };
                                diagnostics.push(Diagnostic {
                                    range: (n.location.start, n.location.end),
                                    severity: DiagnosticSeverity::Warning,
                                    code: Some("deprecated-defined".to_string()),
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
                                });
                            }
                        }
                    }
                }
            }

            // Check for deprecated $[ variable
            NodeKind::Variable { sigil, name } => {
                if sigil == "$" && name == "[" {
                    diagnostics.push(Diagnostic {
                        range: (n.location.start, n.location.start + 2),
                        severity: DiagnosticSeverity::Warning,
                        code: Some("deprecated-array-base".to_string()),
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
                    });
                }
            }

            _ => {}
        }
    });
}
