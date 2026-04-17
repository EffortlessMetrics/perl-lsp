//! Inline variable refactoring code action
//!
//! Provides the "Inline variable" refactoring that replaces all usages of a
//! variable with its initializer expression and removes the variable declaration.
//!
//! # Behavior
//!
//! - Offered when cursor is on a variable declaration (`my $var = ...;`) or usage
//! - Removes the declaration line and replaces all usages with the initializer
//! - NOT offered for self-referential variables (`my $x = $x + 1;`)

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

/// Create an inline variable code action if applicable at the given position.
///
/// Returns `Some(CodeAction)` if the position is on or near a variable declaration
/// that can be safely inlined, or `None` if no inlineable variable is found.
///
/// The `var_decl` should be a VariableDeclaration node.
pub fn create_inline_variable_action(
    source: &str,
    root_ast: &Node,
    var_decl: &Node,
) -> Option<CodeAction> {
    // Get variable name and sigil
    let (var_name, sigil) = extract_var_name(var_decl)?;

    // Check if the variable is self-referential (cannot be inlined)
    if is_self_referential(var_decl, &var_name) {
        return None;
    }

    // Get the initializer expression
    let initializer = match &var_decl.kind {
        NodeKind::VariableDeclaration { initializer, .. } => initializer.as_ref()?,
        _ => return None,
    };

    // Get the text of the initializer expression
    let init_text = &source[initializer.location.start..initializer.location.end];

    // Build the list of edits
    let mut edits = Vec::new();

    // Find the end of the declaration line (including semicolon and newline)
    let decl_line_end = find_statement_end(source, var_decl.location.end);

    // Edit 1: Remove the declaration line
    edits.push(TextEdit {
        location: SourceLocation { start: var_decl.location.start, end: decl_line_end },
        new_text: String::new(),
    });

    // Find all usages of this variable and replace them
    // We need to search from the ROOT AST, not the VariableDeclaration node
    let usages = find_variable_usages(root_ast, &var_name, &sigil, var_decl.location.end, source);

    for usage in usages {
        edits.push(TextEdit {
            location: SourceLocation { start: usage.start, end: usage.end },
            new_text: init_text.to_string(),
        });
    }

    Some(CodeAction {
        title: format!("Inline variable '{}'", var_name),
        kind: CodeActionKind::RefactorInline,
        diagnostics: Vec::new(),
        edit: CodeActionEdit { changes: edits },
        is_preferred: false,
    })
}

/// Extract variable name and sigil from a VariableDeclaration node.
fn extract_var_name(var_decl: &Node) -> Option<(String, String)> {
    let NodeKind::VariableDeclaration { variable, .. } = &var_decl.kind else {
        return None;
    };

    match &variable.kind {
        NodeKind::Variable { name, sigil } => Some((name.clone(), sigil.clone())),
        _ => None,
    }
}

/// Check if the variable declaration is self-referential
/// (the initializer uses the same variable).
fn is_self_referential(var_decl: &Node, var_name: &str) -> bool {
    let NodeKind::VariableDeclaration { initializer, .. } = &var_decl.kind else {
        return false;
    };

    let Some(init) = initializer else {
        return false;
    };

    // Walk the initializer and check if it references the same variable
    contains_variable_recursive(init, var_name)
}

/// Check if a node or any of its children contains a variable with the given name.
fn contains_variable_recursive(node: &Node, var_name: &str) -> bool {
    if let NodeKind::Variable { name, .. } = &node.kind {
        if name == var_name {
            return true;
        }
    }

    // Recurse into children
    match &node.kind {
        NodeKind::Program { statements } => statements.iter().any(|s| contains_variable_recursive(s, var_name)),
        NodeKind::Block { statements } => statements.iter().any(|s| contains_variable_recursive(s, var_name)),
        NodeKind::ExpressionStatement { expression } => contains_variable_recursive(expression, var_name),
        NodeKind::Assignment { lhs, rhs, .. } => {
            contains_variable_recursive(lhs, var_name) || contains_variable_recursive(rhs, var_name)
        }
        NodeKind::Binary { left, right, .. } => {
            contains_variable_recursive(left, var_name) || contains_variable_recursive(right, var_name)
        }
        NodeKind::Unary { operand, .. } => contains_variable_recursive(operand, var_name),
        NodeKind::FunctionCall { args, .. } => args.iter().any(|a| contains_variable_recursive(a, var_name)),
        NodeKind::MethodCall { object, args, .. } => {
            contains_variable_recursive(object, var_name) || args.iter().any(|a| contains_variable_recursive(a, var_name))
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            contains_variable_recursive(condition, var_name)
                || contains_variable_recursive(then_expr, var_name)
                || contains_variable_recursive(else_expr, var_name)
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            contains_variable_recursive(condition, var_name)
                || contains_variable_recursive(then_branch, var_name)
                || elsif_branches.iter().any(|(c, b)| contains_variable_recursive(c, var_name) || contains_variable_recursive(b, var_name))
                || else_branch.as_ref().map_or(false, |b| contains_variable_recursive(b, var_name))
        }
        NodeKind::While { condition, body, .. } => {
            contains_variable_recursive(condition, var_name) || contains_variable_recursive(body, var_name)
        }
        NodeKind::For { init, condition, update, body, .. } => {
            init.as_ref().map_or(false, |i| contains_variable_recursive(i, var_name))
                || condition.as_ref().map_or(false, |c| contains_variable_recursive(c, var_name))
                || update.as_ref().map_or(false, |u| contains_variable_recursive(u, var_name))
                || contains_variable_recursive(body, var_name)
        }
        NodeKind::Foreach { list, body, continue_block, .. } => {
            contains_variable_recursive(list, var_name)
                || contains_variable_recursive(body, var_name)
                || continue_block.as_ref().map_or(false, |cb| contains_variable_recursive(cb, var_name))
        }
        NodeKind::Subroutine { body, .. } => contains_variable_recursive(body, var_name),
        NodeKind::Return { value } => value.as_ref().map_or(false, |v| contains_variable_recursive(v, var_name)),
        _ => false,
    }
}

/// Find all usages of a variable after a given position.
fn find_variable_usages(
    ast: &Node,
    var_name: &str,
    sigil: &str,
    after_pos: usize,
    source: &str,
) -> Vec<UsageLocation> {
    let mut usages = Vec::new();
    collect_usages(ast, var_name, sigil, after_pos, source, &mut usages);
    usages
}

struct UsageLocation {
    start: usize,
    end: usize,
}

fn collect_usages(
    node: &Node,
    var_name: &str,
    sigil: &str,
    after_pos: usize,
    source: &str,
    usages: &mut Vec<UsageLocation>,
) {
    // Check if this node is a usage of the variable
    if let NodeKind::Variable { name, .. } = &node.kind {
        if name == var_name && node.location.start > after_pos {
            // Verify this is actually a usage (text matches with sigil)
            let text = &source[node.location.start..node.location.end];
            if text.starts_with(sigil) {
                usages.push(UsageLocation { start: node.location.start, end: node.location.end });
            }
        }
    }

    // Recurse into children based on node type
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                collect_usages(stmt, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                collect_usages(stmt, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            collect_usages(expression, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            // Don't count the declared variable as a usage
            if let NodeKind::Variable { name, .. } = &variable.kind {
                if name == var_name {
                    // This is the declaration, skip but check initializer
                    if let Some(init) = initializer {
                        collect_usages(init, var_name, sigil, after_pos, source, usages);
                    }
                    return;
                }
            }
            if let Some(init) = initializer {
                collect_usages(init, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            // Check if any of the declared variables match
            let mut is_decl = false;
            for v in variables {
                if let NodeKind::Variable { name, .. } = &v.kind {
                    if name == var_name {
                        is_decl = true;
                        break;
                    }
                }
            }
            if is_decl {
                // Skip the declaration but check initializer
                if let Some(init) = initializer {
                    collect_usages(init, var_name, sigil, after_pos, source, usages);
                }
                return;
            }
            if let Some(init) = initializer {
                collect_usages(init, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            collect_usages(lhs, var_name, sigil, after_pos, source, usages);
            collect_usages(rhs, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::Binary { left, right, .. } => {
            collect_usages(left, var_name, sigil, after_pos, source, usages);
            collect_usages(right, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::Unary { operand, .. } => {
            collect_usages(operand, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_usages(arg, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            collect_usages(object, var_name, sigil, after_pos, source, usages);
            for arg in args {
                collect_usages(arg, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            collect_usages(condition, var_name, sigil, after_pos, source, usages);
            collect_usages(then_expr, var_name, sigil, after_pos, source, usages);
            collect_usages(else_expr, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            collect_usages(condition, var_name, sigil, after_pos, source, usages);
            collect_usages(then_branch, var_name, sigil, after_pos, source, usages);
            for (cond, branch) in elsif_branches {
                collect_usages(cond, var_name, sigil, after_pos, source, usages);
                collect_usages(branch, var_name, sigil, after_pos, source, usages);
            }
            if let Some(b) = else_branch {
                collect_usages(b, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::While { condition, body, .. } => {
            collect_usages(condition, var_name, sigil, after_pos, source, usages);
            collect_usages(body, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::For { init, condition, update, body, .. } => {
            if let Some(i) = init {
                collect_usages(i, var_name, sigil, after_pos, source, usages);
            }
            if let Some(c) = condition {
                collect_usages(c, var_name, sigil, after_pos, source, usages);
            }
            if let Some(u) = update {
                collect_usages(u, var_name, sigil, after_pos, source, usages);
            }
            collect_usages(body, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::Foreach { list, body, continue_block, .. } => {
            collect_usages(list, var_name, sigil, after_pos, source, usages);
            collect_usages(body, var_name, sigil, after_pos, source, usages);
            if let Some(cb) = continue_block {
                collect_usages(cb, var_name, sigil, after_pos, source, usages);
            }
        }
        NodeKind::Subroutine { body, .. } => {
            collect_usages(body, var_name, sigil, after_pos, source, usages);
        }
        NodeKind::Return { value } => {
            if let Some(v) = value {
                collect_usages(v, var_name, sigil, after_pos, source, usages);
            }
        }
        _ => {}
    }
}

/// Find the end position of a statement (including semicolon and newline).
fn find_statement_end(source: &str, after_pos: usize) -> usize {
    let rest = &source[after_pos..];
    let mut end = after_pos;

    for (i, ch) in rest.chars().enumerate() {
        end = after_pos + i + 1;
        if ch == ';' {
            // Include the semicolon
            continue;
        }
        if ch == '\n' {
            // Don't include the newline (we want to remove the whole line including newline)
            end = after_pos + i;
            break;
        }
        if ch == '#' {
            // Comment - go back to before the comment
            end = after_pos + i;
            if end > 0 && source.as_bytes()[end - 1] == b' ' {
                end -= 1; // Remove trailing space before comment
            }
            break;
        }
    }

    end
}