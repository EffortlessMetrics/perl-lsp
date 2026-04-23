//! Conservative `next`/`last`/`redo LABEL` validation.
//!
//! This lint warns when a loop-control statement targets an explicit label
//! (`next FOO;`, `last BAR;`, `redo BAZ;`) and no matching label symbol
//! exists anywhere in the current file. Bare `next;` / `last;` / `redo;`
//! (no label) are always allowed — they target the innermost enclosing
//! loop at runtime.
//!
//! Like [`check_goto_labels`](super::goto_label::check_goto_labels), the
//! analysis is intentionally file-scoped and forgiving: it only fires when
//! the target label is *not defined anywhere in the file*, which avoids
//! false positives from labels introduced by macros, source filters, or
//! imported code that the parser cannot see.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL410` | Warning | `next`/`last`/`redo LABEL` references a label that is not defined in the file |
//!
//! # Runtime behavior
//!
//! At runtime, Perl reports `Label not found for "next LABEL"` (or
//! equivalent) as a fatal error. Catching it statically lets editors flag
//! the problem before the program is run.

use super::super::internal_types::{Diagnostic, RelatedInformation};
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

use super::super::walker::walk_node;

fn has_label(symbol_table: &SymbolTable, label: &str) -> bool {
    symbol_table
        .symbols
        .get(label)
        .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Label))
}

/// Warn when a `next LABEL`, `last LABEL`, or `redo LABEL` target does not
/// have a matching label symbol in the same file.
pub fn check_loop_control_labels(
    root: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    walk_node(root, &mut |node| {
        let NodeKind::LoopControl { op, label } = &node.kind else {
            return;
        };

        // Bare forms (no label) always target the innermost enclosing loop
        // and are always valid; nothing to check.
        let Some(label_name) = label.as_deref() else {
            return;
        };

        // Defensive: only the three documented ops carry a label.  Ignore
        // any future variants to avoid double-reporting if another lint
        // grows to cover them.
        if !matches!(op.as_str(), "next" | "last" | "redo") {
            return;
        }

        if has_label(symbol_table, label_name) {
            return;
        }

        diagnostics.push(Diagnostic {
            range: (node.location.start, node.location.end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::LoopControlUndefinedLabel.as_str().to_string()),
            message: format!(
                "`{op} {label_name}` references a label that is not defined in this file"
            ),
            related_information: vec![RelatedInformation {
                location: (node.location.start, node.location.end),
                message:
                    "Add a matching `LABEL:` on an enclosing loop, or drop the label to target the innermost loop."
                        .to_string(),
            }],
            tags: Vec::new(),
            suggestion: Some(format!(
                "Define a `{label_name}:` label on the intended enclosing loop, or write `{op};` to target the innermost loop"
            )),
        });
    });
}
