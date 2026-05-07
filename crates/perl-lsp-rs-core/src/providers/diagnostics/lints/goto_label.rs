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
