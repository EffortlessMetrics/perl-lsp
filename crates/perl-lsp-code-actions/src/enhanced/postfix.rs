//! Postfix conversion code actions

use crate::ide::lsp_compat::code_actions::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind};

/// Convert if statement to postfix form
pub fn convert_to_postfix(node: &Node, source: &str) -> Option<CodeAction> {
    if let NodeKind::If { condition, then_branch, elsif_branches, else_branch } = &node.kind {
        // Only convert simple if statements with no elsif/else
        if elsif_branches.is_empty() && else_branch.is_none() {
            if let NodeKind::Block { statements } = &then_branch.kind {
                if statements.len() == 1 {
                    let stmt = &statements[0];
                    let stmt_text = &source[stmt.location.start..stmt.location.end];
                    let cond_text = &source[condition.location.start..condition.location.end];

                    // Check if statement is simple enough for postfix
                    if !stmt_text.contains('\n') && stmt_text.len() < 80 {
                        return Some(CodeAction {
                            title: "Convert to postfix if".to_string(),
                            kind: CodeActionKind::RefactorRewrite,
                            diagnostics: Vec::new(),
                            edit: CodeActionEdit {
                                changes: vec![TextEdit {
                                    location: node.location,
                                    new_text: format!("{} if {}", stmt_text, cond_text),
                                }],
                            },
                            is_preferred: false,
                        });
                    }
                }
            }
        }
    }

    // Similarly for while, until
    // Note: Unless is not a separate node type in this AST

    None
}
