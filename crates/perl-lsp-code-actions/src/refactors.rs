//! Refactoring actions for code transformations
//!
//! Provides automated refactoring operations for improving code structure.

use super::ast_utils::{find_function_insert_position, find_node_at_range, find_statement_start};
use super::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::{Node, NodeKind, SourceLocation};

/// Get refactoring actions for a selection
pub fn get_refactoring_actions(source: &str, ast: &Node, range: (usize, usize)) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Use the enhanced provider for better refactorings
    let enhanced_provider = super::enhanced::EnhancedCodeActionsProvider::new(source.to_string());
    actions.extend(enhanced_provider.get_enhanced_refactoring_actions(ast, range));

    // Keep basic refactorings as fallback
    if let Some(node) = find_node_at_range(ast, range) {
        match &node.kind {
            // Extract variable (basic version, enhanced version is better)
            NodeKind::FunctionCall { .. } | NodeKind::Binary { .. } if actions.is_empty() => {
                actions.push(CodeAction {
                    title: "Extract to variable".to_string(),
                    kind: CodeActionKind::RefactorExtract,
                    diagnostics: Vec::new(),
                    edit: extract_variable(source, node, range),
                    is_preferred: false,
                });
            }

            // Extract function (basic version)
            NodeKind::Block { .. } if actions.is_empty() => {
                actions.push(CodeAction {
                    title: "Extract to function".to_string(),
                    kind: CodeActionKind::RefactorExtract,
                    diagnostics: Vec::new(),
                    edit: extract_function(source, node, range),
                    is_preferred: false,
                });
            }

            _ => {}
        }
    }

    actions
}

/// Extract expression to variable
fn extract_variable(source: &str, node: &Node, _range: (usize, usize)) -> CodeActionEdit {
    let expr_text = &source[node.location.start..node.location.end];
    let var_name = "$extracted_var";

    // Find statement start
    let stmt_start = find_statement_start(source, node.location.start);

    CodeActionEdit {
        changes: vec![
            // Insert variable declaration
            TextEdit {
                location: SourceLocation { start: stmt_start, end: stmt_start },
                new_text: format!("my {} = {};\n", var_name, expr_text),
            },
            // Replace expression with variable
            TextEdit { location: node.location, new_text: var_name.to_string() },
        ],
    }
}

/// Extract statements to function
fn extract_function(source: &str, node: &Node, _range: (usize, usize)) -> CodeActionEdit {
    let body_text = &source[node.location.start..node.location.end];
    let func_name = "extracted_function";

    // Find a good place to insert the function
    let insert_pos = find_function_insert_position(source);

    CodeActionEdit {
        changes: vec![
            // Insert function definition
            TextEdit {
                location: SourceLocation { start: insert_pos, end: insert_pos },
                new_text: format!("\nsub {} {{\n{}\n}}\n", func_name, body_text),
            },
            // Replace statements with function call
            TextEdit { location: node.location, new_text: format!("{}();", func_name) },
        ],
    }
}
