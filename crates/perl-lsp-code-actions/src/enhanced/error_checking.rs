//! Error checking code actions

use crate::ide::lsp_compat::code_actions::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind};

/// Add error checking to file operations
pub fn add_error_checking(node: &Node, source: &str) -> Option<CodeAction> {
    if let NodeKind::FunctionCall { name, args: _ } = &node.kind {
        let func_name = name.as_str();

        // Check for file operations without error checking
        if matches!(
            func_name,
            "open" | "close" | "print" | "printf" | "write" | "read" | "seek" | "truncate"
        ) {
            // Check if already has error checking
            if !has_error_checking_nearby(source, node.location.end) {
                let expr_text = &source[node.location.start..node.location.end];

                return Some(CodeAction {
                    title: format!("Add error checking to '{}'", func_name),
                    kind: CodeActionKind::RefactorRewrite,
                    diagnostics: Vec::new(),
                    edit: CodeActionEdit {
                        changes: vec![TextEdit {
                            location: node.location,
                            new_text: format!(
                                "{} or die \"Failed to {}: $!\"",
                                expr_text, func_name
                            ),
                        }],
                    },
                    is_preferred: false,
                });
            }
        }
    }

    None
}

/// Check if there's error checking nearby
pub fn has_error_checking_nearby(source: &str, pos: usize) -> bool {
    // Check next 50 characters for "or", "||", "die", "warn"
    let check_text = &source[pos..std::cmp::min(pos + 50, source.len())];
    check_text.contains(" or ")
        || check_text.contains(" || ")
        || check_text.contains("die")
        || check_text.contains("warn")
}
