//! Common mistakes lint checks
//!
//! This module provides functionality for detecting common mistakes in Perl code
//! such as assignment in conditions and comparing with undef.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `assignment-in-condition` | Warning | `=` in `if`/`while` condition (likely meant `==`) |
//! | `numeric-undef` | Warning | `==`/`!=` with potentially undefined value |
//! | `PL400` | Information | Bareword filehandle in `open` call |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

use super::super::internal_types::{Diagnostic, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Check for common mistakes
///
/// This function walks the AST looking for common mistakes such as:
/// - Assignment in condition (should be comparison)
/// - Using == or != with potentially undefined values
pub fn check_common_mistakes(
    node: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_node(node, &mut |n| {
        match &n.kind {
            // Check for assignment in condition
            NodeKind::If { condition, .. } | NodeKind::While { condition, .. } => {
                check_assignment_in_condition(condition, diagnostics);
            }

            // Check for == or != with undef
            NodeKind::Binary { op, left, right } => {
                if (op == "==" || op == "!=")
                    && (might_be_undef(left, symbol_table) || might_be_undef(right, symbol_table))
                {
                    diagnostics.push(Diagnostic {
                        range: (n.location.start, n.location.end),
                        severity: DiagnosticSeverity::Warning,
                        code: Some(DiagnosticCode::NumericComparisonWithUndef.as_str().to_string()),
                        message: format!("Using '{}' with potentially undefined value -- use 'defined()' to check first", op),
                        related_information: vec![RelatedInformation {
                            location: (n.location.start, n.location.end),
                            message: "Consider using 'defined' check or '//' operator".to_string(),
                        }],
                        tags: Vec::new(),
                        suggestion: Some("Guard with 'defined($var)' or use the '//' (defined-or) operator".to_string()),
                    });
                }
            }
            NodeKind::FunctionCall { name, args } => {
                check_bareword_filehandle(name, args, n, diagnostics);
            }

            _ => {}
        }
    });
}

fn check_bareword_filehandle(
    function_name: &str,
    args: &[Node],
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if function_name != "open" {
        return;
    }

    let open_args: &[Node] = if args.len() == 1 {
        if let NodeKind::ArrayLiteral { elements } = &args[0].kind {
            elements
        } else {
            args
        }
    } else {
        args
    };

    if open_args.len() < 2 {
        return;
    }

    let NodeKind::Identifier { name } = &open_args[0].kind else {
        return;
    };

    if matches!(
        name.as_str(),
        "STDIN" | "STDOUT" | "STDERR" | "ARGV" | "ARGVOUT" | "DATA"
    ) {
        return;
    }

    diagnostics.push(Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Information,
        code: Some(DiagnosticCode::BarewordFilehandle.as_str().to_string()),
        message: "Use lexical filehandles instead of bareword filehandles".to_string(),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "Bareword filehandles are global and can lead to accidental reuse across scopes"
                .to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some("Use lexical filehandle: open(my $fh, ... )".to_string()),
    });
}

/// Check for assignment in condition (common mistake)
fn check_assignment_in_condition(condition: &Node, diagnostics: &mut Vec<Diagnostic>) {
    match &condition.kind {
        NodeKind::Binary { op, .. } if op == "=" => {
            diagnostics.push(Diagnostic {
                range: (condition.location.start, condition.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::AssignmentInCondition.as_str().to_string()),
                message: "Assignment in condition - did you mean '=='?".to_string(),
                related_information: vec![
                    RelatedInformation {
                        location: (condition.location.start, condition.location.end),
                        message: "💡 Use '==' for comparison or 'eq' for string comparison".to_string(),
                    },
                    RelatedInformation {
                        location: (condition.location.start, condition.location.end),
                        message: "ℹ️ Assignment (=) in conditions is usually a mistake. If intentional, wrap in parentheses: if (($x = value))".to_string(),
                    }
                ],
                tags: Vec::new(),
                suggestion: Some("Replace '=' with '==' for numeric comparison or 'eq' for string comparison".to_string()),
            });
        }
        NodeKind::Assignment { .. } => {
            diagnostics.push(Diagnostic {
                range: (condition.location.start, condition.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::AssignmentInCondition.as_str().to_string()),
                message: "Assignment in condition - did you mean '=='?".to_string(),
                related_information: vec![
                    RelatedInformation {
                        location: (condition.location.start, condition.location.end),
                        message: "💡 Use '==' for comparison or 'eq' for string comparison".to_string(),
                    },
                    RelatedInformation {
                        location: (condition.location.start, condition.location.end),
                        message: "ℹ️ Assignment in conditions is usually a mistake. If intentional, wrap in parentheses: if (($x = value))".to_string(),
                    }
                ],
                tags: Vec::new(),
                suggestion: Some("Replace '=' with '==' for numeric comparison or 'eq' for string comparison".to_string()),
            });
        }
        _ => {}
    }
}

/// Check if a node might evaluate to undef
fn might_be_undef(node: &Node, symbol_table: &SymbolTable) -> bool {
    match &node.kind {
        NodeKind::Variable { name, .. } => {
            // If variable is not defined in scope, it might be undef
            symbol_table.find_symbol(name, 0, SymbolKind::scalar()).is_empty()
        }
        NodeKind::Undef => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::parser::Parser;
    use perl_semantic_analyzer::analysis::symbol::SymbolExtractor;
    use perl_tdd_support::must;

    fn common_mistakes_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let symbol_table = SymbolExtractor::new_with_source(source).extract(&ast);
        let mut diagnostics = Vec::new();
        check_common_mistakes(&ast, &symbol_table, &mut diagnostics);
        diagnostics
    }

    #[test]
    fn bareword_filehandle_open_is_flagged() {
        let diags = common_mistakes_diags(r#"open(FH, "<", "file.txt");"#);
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL400")),
            "bareword filehandle should be flagged as PL400: {diags:?}"
        );
    }

    #[test]
    fn lexical_filehandle_open_is_not_flagged() {
        let diags = common_mistakes_diags(r#"open(my $fh, "<", "file.txt");"#);
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL400")),
            "lexical filehandle open should not be flagged as PL400: {diags:?}"
        );
    }

    #[test]
    fn std_handles_are_not_flagged() {
        let diags = common_mistakes_diags(r#"open(STDOUT, ">", "out.txt");"#);
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL400")),
            "STDOUT handle should not be flagged as PL400: {diags:?}"
        );
    }
}
