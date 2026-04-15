//! Conservative `next LABEL` / `last LABEL` / `redo LABEL` validation.
//!
//! This lint warns when a loop control statement (`next`, `last`, `redo`)
//! references a label that is not defined anywhere in the current file.
//! Loop control statements without labels are not validated — they implicitly
//! target the innermost enclosing loop, which is always present at runtime if
//! the code parses successfully.
//!
//! The check is intentionally file-scoped (not block-scoped) for parity with
//! the `goto LABEL` lint (`PL409`). A reference is accepted as long as some
//! `LABEL:` appears in the file; tighter scope-aware validation can layer on
//! top later without changing the diagnostic code.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL410` | Warning | `next`/`last`/`redo LABEL` references a label not defined in the file |

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

/// Warn when a `next`/`last`/`redo LABEL` target does not have a matching label
/// definition somewhere in the same file.
pub fn check_loop_control_labels(
    root: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_node(root, &mut |node| {
        let NodeKind::LoopControl { op, label: Some(label) } = &node.kind else {
            return;
        };

        if has_label(symbol_table, label) {
            return;
        }

        diagnostics.push(Diagnostic {
            range: (node.location.start, node.location.end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::LoopControlUndefinedLabel.as_str().to_string()),
            message: format!("Loop control label '{label}' is not defined in this file"),
            related_information: vec![RelatedInformation {
                location: (node.location.start, node.location.end),
                message: format!(
                    "`{op} {label}` requires a `{label}:` label on an enclosing loop. \
                    Perl raises a fatal error at runtime when the label is not in scope."
                ),
            }],
            tags: Vec::new(),
            suggestion: Some(format!(
                "Add a `{label}:` prefix to the enclosing loop or remove the label from `{op}`"
            )),
        });
    });
}
