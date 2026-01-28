//! Extract variable code action

use crate::ide::lsp_compat::code_actions::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

use super::helpers::Helpers;

/// Create extract variable action with smart naming
pub fn create_extract_variable_action(
    node: &Node,
    source: &str,
    helpers: &Helpers<'_>,
) -> CodeAction {
    let expr_text = &source[node.location.start..node.location.end];
    let var_name = suggest_variable_name(node);

    // Find the best insertion point
    let stmt_start = helpers.find_statement_start(node.location.start);
    let indent = helpers.get_indent_at(stmt_start);

    CodeAction {
        title: format!("Extract '{}' to variable", helpers.truncate_expr(expr_text, 30)),
        kind: CodeActionKind::RefactorExtract,
        diagnostics: Vec::new(),
        edit: CodeActionEdit {
            changes: vec![
                // Insert variable declaration
                TextEdit {
                    location: SourceLocation { start: stmt_start, end: stmt_start },
                    new_text: format!("{}my ${} = {};\n", indent, var_name, expr_text),
                },
                // Replace expression with variable
                TextEdit { location: node.location, new_text: format!("${}", var_name) },
            ],
        },
        is_preferred: false,
    }
}

/// Suggest a variable name based on the expression
pub fn suggest_variable_name(node: &Node) -> String {
    match &node.kind {
        NodeKind::FunctionCall { name, .. } => {
            let func_name = name.as_str();
            match func_name {
                "length" | "size" => "len",
                "split" => "parts",
                "join" => "joined",
                "sort" => "sorted",
                "reverse" => "reversed",
                "grep" | "filter" => "filtered",
                "map" => "mapped",
                _ => "result",
            }
        }
        NodeKind::Binary { op, .. } => match op.as_str() {
            "+" | "-" | "*" | "/" | "%" => "result",
            "." | "x" => "str",
            "&&" | "||" | "and" | "or" => "condition",
            "==" | "!=" | "<" | ">" | "<=" | ">=" => "is_valid",
            _ => "value",
        },
        _ => "extracted",
    }
    .to_string()
}
