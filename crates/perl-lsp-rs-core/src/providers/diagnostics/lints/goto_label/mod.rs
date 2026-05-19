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

mod diagnostic;
mod labels;
mod target;

use super::super::internal_types::Diagnostic;
use perl_parser_core::ast::{Node, NodeKind};
use perl_semantic_analyzer::symbol::SymbolTable;

use super::super::walker::walk_node;

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

        let Some(label) = target::plain_label_name(target) else {
            return;
        };

        if labels::has_label(symbol_table, label) {
            return;
        }

        diagnostics.push(diagnostic::undefined_label(target, label));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_semantic_analyzer::analysis::symbol::SymbolExtractor;
    use perl_tdd_support::{must, must_some};

    fn goto_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let symbol_table = SymbolExtractor::new_with_source(source).extract(&ast);
        let mut diags = Vec::new();
        check_goto_labels(&ast, &symbol_table, &mut diags);
        diags
    }

    fn has_pl409(diags: &[Diagnostic]) -> bool {
        diags.iter().any(|d| d.code.as_deref() == Some("PL409"))
    }

    #[test]
    fn goto_undefined_label_is_flagged() {
        let diags = goto_diags("goto MISSING;");
        assert!(has_pl409(&diags), "goto to undefined label should be flagged as PL409: {diags:?}");
    }

    #[test]
    fn goto_defined_label_not_flagged() {
        let diags = goto_diags("goto FOUND;\nFOUND: my $x = 1;");
        assert!(!has_pl409(&diags), "goto to a defined label should not be flagged: {diags:?}");
    }

    #[test]
    fn goto_sub_reference_not_flagged() {
        let diags = goto_diags("sub foo { }; goto &foo;");
        assert!(!has_pl409(&diags), "goto &sub should not be flagged as PL409: {diags:?}");
    }

    #[test]
    fn goto_variable_not_flagged() {
        let diags = goto_diags("my $target = 'LABEL'; goto $target;");
        assert!(!has_pl409(&diags), "goto $var should not be flagged as PL409: {diags:?}");
    }

    #[test]
    fn diagnostic_message_names_the_label() {
        let diags = goto_diags("goto NOWHERE;");
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL409")));
        assert!(
            diag.message.contains("NOWHERE"),
            "PL409 message should name the missing label: {}",
            diag.message
        );
    }

    #[test]
    fn diagnostic_has_suggestion() {
        let diags = goto_diags("goto PHANTOM;");
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL409")));
        assert!(diag.suggestion.is_some(), "PL409 should carry a suggestion");
    }

    #[test]
    fn no_goto_no_diagnostic() {
        let diags = goto_diags("my $x = 1; print $x;");
        assert!(!has_pl409(&diags), "code without goto should not trigger PL409: {diags:?}");
    }
}
