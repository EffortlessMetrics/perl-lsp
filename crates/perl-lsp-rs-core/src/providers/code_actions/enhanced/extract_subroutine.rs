//! Extract subroutine code action

use super::super::types::{CodeAction, CodeActionEdit, CodeActionKind};
use crate::providers::rename::TextEdit;
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

/// Detect return values in a block.
///
/// Uses a heuristic: if the last non-empty statement in the block is a bare
/// variable expression (implicit return convention in Perl), and that variable
/// was declared inside the block, treat it as the return value.
///
/// Returns a vec of bare name strings (without sigil, matching `detect_parameters`
/// convention, e.g. `["result"]`).
pub fn detect_return_values(node: &Node) -> Vec<String> {
    let statements = match &node.kind {
        NodeKind::Block { statements } => statements.as_slice(),
        _ => return Vec::new(),
    };

    // Collect all variables declared inside the block (bare names, no sigil).
    let mut declared_inside: HashSet<String> = HashSet::new();
    for stmt in statements {
        collect_declared_variables(stmt, &mut declared_inside);
    }

    // Look at the last non-empty statement for implicit or explicit return.
    let last = statements
        .iter()
        .rev()
        .find(|s| !matches!(&s.kind, NodeKind::Block { statements } if statements.is_empty()));

    if let Some(last_stmt) = last
        && let Some(var_name) = extract_last_expression_variable(last_stmt)
        && declared_inside.contains(&var_name)
    {
        return vec![var_name];
    }

    Vec::new()
}

/// Collect the bare names (no sigil) of all variables declared via `my` in a node.
fn collect_declared_variables(node: &Node, declared: &mut HashSet<String>) {
    match &node.kind {
        NodeKind::VariableDeclaration { variable, .. } => {
            if let NodeKind::Variable { name, .. } = &variable.kind {
                declared.insert(name.clone());
            }
        }
        NodeKind::VariableListDeclaration { variables, .. } => {
            for v in variables {
                if let NodeKind::Variable { name, .. } = &v.kind {
                    declared.insert(name.clone());
                }
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                collect_declared_variables(stmt, declared);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            collect_declared_variables(expression, declared);
        }
        _ => {}
    }
}

/// If a statement is a bare variable expression or an explicit return, return the
/// variable name (bare, no sigil). Used to detect the implicit return value.
fn extract_last_expression_variable(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => {
            extract_last_expression_variable(expression)
        }
        NodeKind::Variable { name, .. } => Some(name.clone()),
        NodeKind::Return { value } => {
            value.as_ref().and_then(|v| extract_last_expression_variable(v))
        }
        _ => None,
    }
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
        NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(start: usize, end: usize) -> SourceLocation {
        SourceLocation { start, end }
    }

    fn var(name: &str, start: usize) -> Node {
        Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() },
            loc(start, start + name.len() + 1),
        )
    }

    fn block(statements: Vec<Node>, start: usize, end: usize) -> Node {
        Node::new(NodeKind::Block { statements }, loc(start, end))
    }

    #[test]
    fn collect_variables_visits_if_and_while_children() {
        let if_node = Node::new(
            NodeKind::If {
                condition: Box::new(var("cond", 1)),
                then_branch: Box::new(block(vec![var("then_value", 10)], 9, 25)),
                elsif_branches: vec![(
                    Box::new(var("elsif_cond", 30)),
                    Box::new(block(vec![var("elsif_value", 44)], 43, 60)),
                )],
                else_branch: Some(Box::new(block(vec![var("else_value", 66)], 65, 80))),
                keyword: Some("unless".to_string()),
            },
            loc(0, 81),
        );
        let while_node = Node::new(
            NodeKind::While {
                condition: Box::new(var("loop_cond", 90)),
                body: Box::new(block(vec![var("loop_value", 106)], 105, 122)),
                continue_block: None,
                keyword: Some("until".to_string()),
            },
            loc(89, 123),
        );
        let root = block(vec![if_node, while_node], 0, 123);

        let mut vars = HashSet::new();
        collect_variables(&root, &mut vars);

        let expected = [
            "cond",
            "then_value",
            "elsif_cond",
            "elsif_value",
            "else_value",
            "loop_cond",
            "loop_value",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<HashSet<_>>();
        assert_eq!(vars, expected);
    }
}
