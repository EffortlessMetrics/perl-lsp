//! Extract subroutine code action

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use std::collections::HashSet;

use super::helpers::Helpers;

/// Create extract subroutine action
pub fn create_extract_subroutine_action(
    node: &Node,
    source: &str,
    helpers: &Helpers<'_>,
) -> CodeAction {
    let body_text = &source[node.location.start..node.location.end];
    let sub_name = suggest_subroutine_name(node);
    let params = detect_parameters(node);
    let returns = detect_return_values(node);

    // Generate function signature
    let signature = if params.is_empty() {
        format!("sub {} {{\n", sub_name)
    } else {
        format!("sub {} {{\n    my ({}) = @_;\n", sub_name, params.join(", "))
    };

    // Find insertion position (before current sub or at end)
    let insert_pos = helpers.find_subroutine_insert_position(node.location.start);

    // Generate function call
    let call = if returns.is_empty() {
        format!("{}({});", sub_name, params.join(", "))
    } else {
        format!("my {} = {}({});", returns.join(", "), sub_name, params.join(", "))
    };

    CodeAction {
        title: "Extract to subroutine".to_string(),
        kind: CodeActionKind::RefactorExtract,
        diagnostics: Vec::new(),
        edit: CodeActionEdit {
            changes: vec![
                // Insert function definition
                TextEdit {
                    location: SourceLocation { start: insert_pos, end: insert_pos },
                    new_text: format!("{}{}\n}}\n\n", signature, body_text),
                },
                // Replace block with function call
                TextEdit { location: node.location, new_text: call },
            ],
        },
        is_preferred: false,
    }
}

/// Suggest a subroutine name
pub fn suggest_subroutine_name(_node: &Node) -> String {
    // Could analyze the code to suggest better names
    "process_data".to_string()
}

/// Detect parameters used in a block
pub fn detect_parameters(node: &Node) -> Vec<String> {
    let mut params = HashSet::new();
    collect_variables(node, &mut params);
    params.into_iter().collect()
}

/// Detect return values in a block
pub fn detect_return_values(_node: &Node) -> Vec<String> {
    // For now, return empty - could analyze return statements
    Vec::new()
}

/// Collect variables used in a node
#[allow(clippy::only_used_in_recursion)]
pub fn collect_variables(node: &Node, vars: &mut HashSet<String>) {
    match &node.kind {
        NodeKind::Variable { name, .. } => {
            let var_name = name.as_str();
            vars.insert(var_name.to_string());
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                collect_variables(stmt, vars);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            collect_variables(left, vars);
            collect_variables(right, vars);
        }
        _ => {}
    }
}
