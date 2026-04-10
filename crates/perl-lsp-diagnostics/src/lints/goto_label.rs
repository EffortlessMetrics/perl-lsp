//! Conservative `goto LABEL` validation.
//!
//! This lint only warns when a `goto` target is a plain identifier and no
//! matching label symbol exists anywhere in the current file. Dynamic goto
//! forms (`goto &sub`, `goto $expr`, etc.) are intentionally ignored.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL409` | Warning | `goto LABEL` references a label that is not defined in the file |

use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use perl_parser_core::ast::{Node, NodeKind};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

use super::super::walker::walk_node;

fn has_label(symbol_table: &SymbolTable, label: &str) -> bool {
    symbol_table
        .symbols
        .get(label)
        .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Label))
}

fn goto_target_is_plain_label(target: &Node) -> Option<&str> {
    match &target.kind {
        NodeKind::Identifier { name } => Some(name.as_str()),
        _ => None,
    }
}

/// Warn when a `goto LABEL` target does not have a matching label symbol.
pub fn check_goto_labels(
    root: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_node(root, &mut |node| {
        let NodeKind::Goto { target } = &node.kind else {
            return;
        };

        let Some(label) = goto_target_is_plain_label(target) else {
            return;
        };

        if has_label(symbol_table, label) {
            return;
        }

        diagnostics.push(Diagnostic {
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
        });
    });
}
