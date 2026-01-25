//! Common mistakes lint checks
//!
//! This module provides functionality for detecting common mistakes in Perl code
//! such as assignment in conditions and comparing with undef.

use perl_parser_core::ast::{Node, NodeKind};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

use super::super::types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use super::super::walker::walk_node;

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
                        code: Some("numeric-undef".to_string()),
                        message: format!("Using '{}' with potentially undefined value", op),
                        related_information: vec![RelatedInformation {
                            location: (n.location.start, n.location.end),
                            message: "Consider using 'defined' check or '//' operator".to_string(),
                        }],
                        tags: Vec::new(),
                    });
                }
            }

            _ => {}
        }
    });
}

/// Check for assignment in condition (common mistake)
fn check_assignment_in_condition(condition: &Node, diagnostics: &mut Vec<Diagnostic>) {
    match &condition.kind {
        NodeKind::Binary { op, .. } if op == "=" => {
            diagnostics.push(Diagnostic {
                range: (condition.location.start, condition.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some("assignment-in-condition".to_string()),
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
            });
        }
        NodeKind::Assignment { .. } => {
            diagnostics.push(Diagnostic {
                range: (condition.location.start, condition.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some("assignment-in-condition".to_string()),
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
