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

/// Collect variables used in a node, excluding those declared within it.
///
/// Variables that appear in `VariableDeclaration` nodes inside the block are
/// local to the extracted subroutine and should not be listed as parameters.
pub fn collect_variables(node: &Node, vars: &mut HashSet<String>) {
    collect_variables_inner(node, vars, &mut HashSet::new());
}

fn collect_variables_inner(node: &Node, vars: &mut HashSet<String>, locals: &mut HashSet<String>) {
    match &node.kind {
        NodeKind::Variable { name, .. } => {
            if !locals.contains(name.as_str()) {
                vars.insert(name.clone());
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                collect_variables_inner(stmt, vars, locals);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            collect_variables_inner(expression, vars, locals);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            // The initializer may reference outer variables — collect those first.
            if let Some(init) = initializer {
                collect_variables_inner(init, vars, locals);
            }
            // The declared variable is local to this block — do not treat it as a parameter.
            if let NodeKind::Variable { name, .. } = &variable.kind {
                locals.insert(name.clone());
            }
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            // Initializer may reference outer variables.
            if let Some(init) = initializer {
                collect_variables_inner(init, vars, locals);
            }
            // All declared variables are local to the block.
            for v in variables {
                if let NodeKind::Variable { name, .. } = &v.kind {
                    locals.insert(name.clone());
                }
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            collect_variables_inner(lhs, vars, locals);
            collect_variables_inner(rhs, vars, locals);
        }
        NodeKind::Binary { left, right, .. } => {
            collect_variables_inner(left, vars, locals);
            collect_variables_inner(right, vars, locals);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_variables_inner(arg, vars, locals);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            collect_variables_inner(object, vars, locals);
            for arg in args {
                collect_variables_inner(arg, vars, locals);
            }
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            collect_variables_inner(condition, vars, locals);
            collect_variables_inner(then_branch, vars, locals);
            for (cond, branch) in elsif_branches {
                collect_variables_inner(cond, vars, locals);
                collect_variables_inner(branch, vars, locals);
            }
            if let Some(branch) = else_branch {
                collect_variables_inner(branch, vars, locals);
            }
        }
        NodeKind::While { condition, body, .. } => {
            collect_variables_inner(condition, vars, locals);
            collect_variables_inner(body, vars, locals);
        }
        NodeKind::For { init, condition, update, body, .. } => {
            if let Some(init) = init {
                collect_variables_inner(init, vars, locals);
            }
            if let Some(condition) = condition {
                collect_variables_inner(condition, vars, locals);
            }
            if let Some(update) = update {
                collect_variables_inner(update, vars, locals);
            }
            collect_variables_inner(body, vars, locals);
        }
        _ => {}
    }
}
