//! Diagnostics for `$@` / `$EVAL_ERROR` exception-flow mistakes.
//!
//! The rule is intentionally conservative:
//! - only same-block statement order is considered
//! - `eval` / `try` in the same statement are treated as valid sources
//! - nested blocks are analyzed independently
//! - no attempt is made to model interprocedural dataflow

use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use perl_parser_core::ast::{Node, NodeKind};

/// Warn on stale or context-free reads of `$@` / `$EVAL_ERROR`.
pub fn check_eval_error_flow(root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    visit_node(root, diagnostics);
}

fn visit_node(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            check_statement_list(statements, diagnostics);
        }
        NodeKind::Subroutine { body, .. } | NodeKind::Method { body, .. } => {
            visit_node(body, diagnostics);
        }
        NodeKind::Class { body, .. } => {
            visit_node(body, diagnostics);
        }
        NodeKind::Package { block: Some(block), .. } => {
            visit_node(block, diagnostics);
        }
        NodeKind::PhaseBlock { block, .. } => {
            visit_node(block, diagnostics);
        }
        NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
            visit_node(then_branch, diagnostics);
            for (_, branch) in elsif_branches {
                visit_node(branch, diagnostics);
            }
            if let Some(branch) = else_branch {
                visit_node(branch, diagnostics);
            }
        }
        NodeKind::While { body, continue_block, .. } => {
            visit_node(body, diagnostics);
            if let Some(block) = continue_block {
                visit_node(block, diagnostics);
            }
        }
        NodeKind::For { body, .. } | NodeKind::Foreach { body, .. } => {
            visit_node(body, diagnostics);
        }
        NodeKind::Given { body, .. } | NodeKind::When { body, .. } | NodeKind::Default { body } => {
            visit_node(body, diagnostics);
        }
        NodeKind::Do { block } => {
            visit_node(block, diagnostics);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            visit_node(statement, diagnostics);
        }
        // `eval` and `try` are statement-level sources; their nested blocks are
        // intentionally not walked in this conservative pass.
        NodeKind::Eval { .. } | NodeKind::Try { .. } => {}
        _ => {}
    }
}

fn check_statement_list(statements: &[Node], diagnostics: &mut Vec<Diagnostic>) {
    let mut last_exception_source: Option<usize> = None;

    for (idx, statement) in statements.iter().enumerate() {
        let facts = inspect_statement(statement);
        let immediate_after_source =
            last_exception_source == Some(idx.saturating_sub(1)) && idx > 0;

        if facts.reads_error_var && !facts.has_source && !immediate_after_source {
            diagnostics.push(make_diagnostic(statement, last_exception_source.is_some()));
        }

        if facts.has_source {
            last_exception_source = Some(idx);
        }

        // If this statement already reads `$@` / `$EVAL_ERROR`, treat it as the
        // current exception check/handler and do not immediately recurse into its
        // nested blocks as fresh scopes. That keeps `if ($@) { warn $@; }`
        // from warning on the handler body one statement later.
        if !facts.reads_error_var {
            visit_node(statement, diagnostics);
        }
    }
}

#[derive(Default)]
struct StatementFacts {
    has_source: bool,
    reads_error_var: bool,
}

fn inspect_statement(node: &Node) -> StatementFacts {
    let mut facts = StatementFacts::default();
    inspect_node(node, &mut facts);
    facts
}

fn inspect_node(node: &Node, facts: &mut StatementFacts) {
    match &node.kind {
        NodeKind::Eval { .. } | NodeKind::Try { .. } => {
            facts.has_source = true;
        }
        NodeKind::Variable { sigil, name } if is_error_variable(sigil, name) => {
            facts.reads_error_var = true;
        }
        NodeKind::Program { .. }
        | NodeKind::Block { .. }
        | NodeKind::Subroutine { .. }
        | NodeKind::Method { .. }
        | NodeKind::Class { .. }
        | NodeKind::Package { .. }
        | NodeKind::PhaseBlock { .. } => {}
        NodeKind::If { condition, .. } => {
            inspect_node(condition, facts);
        }
        NodeKind::While { condition, .. } => {
            inspect_node(condition, facts);
        }
        NodeKind::Given { expr, .. } => {
            inspect_node(expr, facts);
        }
        NodeKind::Binary { left, right, .. } => {
            inspect_node(left, facts);
            inspect_node(right, facts);
        }
        NodeKind::Unary { operand, .. } => {
            inspect_node(operand, facts);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            inspect_node(condition, facts);
            inspect_node(then_expr, facts);
            inspect_node(else_expr, facts);
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            inspect_node(lhs, facts);
            inspect_node(rhs, facts);
        }
        NodeKind::FunctionCall { args, .. } | NodeKind::MethodCall { args, .. } => {
            for arg in args {
                inspect_node(arg, facts);
            }
        }
        NodeKind::IndirectCall { object, args, .. } => {
            inspect_node(object, facts);
            for arg in args {
                inspect_node(arg, facts);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            inspect_node(expression, facts);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            inspect_node(variable, facts);
            if let Some(init) = initializer {
                inspect_node(init, facts);
            }
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            for variable in variables {
                inspect_node(variable, facts);
            }
            if let Some(init) = initializer {
                inspect_node(init, facts);
            }
        }
        NodeKind::Return { value: Some(value) } => {
            inspect_node(value, facts);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            inspect_node(statement, facts);
        }
        // Nested block-like nodes are handled by `visit_node` as independent
        // same-block scopes, so they do not contribute to the current statement.
        NodeKind::For { .. } | NodeKind::Foreach { .. } | NodeKind::Do { .. } => {}
        _ => {}
    }
}

fn is_error_variable(sigil: &str, name: &str) -> bool {
    sigil == "$" && matches!(name, "@" | "EVAL_ERROR")
}

fn make_diagnostic(node: &Node, has_prior_source: bool) -> Diagnostic {
    let message = if has_prior_source {
        "Reading `$@` or `$EVAL_ERROR` after an intervening statement can see stale exception state. Check it immediately after the `eval` or `try`."
            .to_string()
    } else {
        "Reading `$@` or `$EVAL_ERROR` without a preceding `eval` or `try` in this block may see stale exception state."
            .to_string()
    };

    Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::EvalErrorFlow.as_str().to_string()),
        message,
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "Move the exception check immediately after the `eval { ... }` or `try { ... }` statement.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(
            "Move the exception check immediately after the `eval` or `try` block."
                .to_string(),
        ),
    }
}
