//! Diagnostics for `$@` / `$EVAL_ERROR` exception-flow mistakes.
//!
//! The rule is intentionally conservative:
//! - only same-block statement order is considered
//! - `eval` / `try` in the same statement are treated as valid sources
//! - nested blocks are analyzed independently
//! - no attempt is made to model interprocedural dataflow

use crate::internal_types::{Diagnostic, RelatedInformation};
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};

/// Warn on stale or context-free reads of `$@` / `$EVAL_ERROR`.
pub fn check_eval_error_flow(root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    visit_node(root, diagnostics, FlowState::default());
}

#[derive(Clone, Copy, Default)]
struct FlowState {
    source_seen: bool,
    immediate_after_source: bool,
}

fn visit_node(node: &Node, diagnostics: &mut Vec<Diagnostic>, state: FlowState) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            check_statement_list(statements, diagnostics, state);
        }
        NodeKind::Subroutine { body, .. } | NodeKind::Method { body, .. } => {
            visit_node(body, diagnostics, FlowState::default());
        }
        NodeKind::Class { body, .. } => {
            visit_node(body, diagnostics, FlowState::default());
        }
        NodeKind::Package { block: Some(block), .. } => {
            visit_node(block, diagnostics, FlowState::default());
        }
        NodeKind::PhaseBlock { block, .. } => {
            visit_node(block, diagnostics, FlowState::default());
        }
        NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
            visit_node(then_branch, diagnostics, state);
            for (_, branch) in elsif_branches {
                visit_node(branch, diagnostics, FlowState::default());
            }
            if let Some(branch) = else_branch {
                visit_node(branch, diagnostics, FlowState::default());
            }
        }
        NodeKind::While { body, continue_block, .. } => {
            visit_node(body, diagnostics, state);
            if let Some(block) = continue_block {
                visit_node(block, diagnostics, state);
            }
        }
        NodeKind::For { body, .. } | NodeKind::Foreach { body, .. } => {
            visit_node(body, diagnostics, FlowState::default());
        }
        NodeKind::Given { body, .. } | NodeKind::When { body, .. } | NodeKind::Default { body } => {
            visit_node(body, diagnostics, FlowState::default());
        }
        NodeKind::Do { block } | NodeKind::Defer { block } => {
            visit_node(block, diagnostics, FlowState::default());
        }
        NodeKind::LabeledStatement { statement, .. } => {
            visit_node(statement, diagnostics, state);
        }
        // `eval` and `try` are statement-level sources; their nested blocks are
        // intentionally not walked in this conservative pass.
        NodeKind::Eval { .. } | NodeKind::Try { .. } => {}
        _ => {}
    }
}

fn check_statement_list(
    statements: &[Node],
    diagnostics: &mut Vec<Diagnostic>,
    mut state: FlowState,
) {
    for statement in statements {
        let entry_state = state;
        let facts = inspect_statement(statement);
        let is_handler_block =
            matches!(&statement.kind, NodeKind::If { .. } | NodeKind::While { .. })
                && facts.reads_error_var;

        if facts.reads_error_var && !facts.has_source && !entry_state.immediate_after_source {
            diagnostics.push(make_diagnostic(statement, entry_state.source_seen));
        }

        if facts.has_source {
            state.source_seen = true;
            state.immediate_after_source = true;
        } else {
            state.immediate_after_source = false;
        }

        // Handler blocks need the outer exception-flow state so the body can
        // still report stale reads after an intervening statement.
        if is_handler_block
            || !facts.reads_error_var
            || matches!(&statement.kind, NodeKind::LabeledStatement { .. })
        {
            visit_node(statement, diagnostics, entry_state);
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
        NodeKind::StatementModifier { statement, condition, .. } => {
            inspect_node(statement, facts);
            inspect_node(condition, facts);
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
            if !matches!(lhs.kind, NodeKind::Variable { .. }) {
                inspect_node(lhs, facts);
            }
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
        NodeKind::VariableDeclaration { initializer: Some(init), .. } => {
            inspect_node(init, facts);
        }
        NodeKind::VariableListDeclaration { initializer: Some(init), .. } => {
            inspect_node(init, facts);
        }
        NodeKind::Return { value: Some(value) } => {
            inspect_node(value, facts);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            inspect_node(statement, facts);
        }
        // Nested block-like nodes are handled by `visit_node` as independent
        // same-block scopes, so they do not contribute to the current statement.
        NodeKind::For { .. }
        | NodeKind::Foreach { .. }
        | NodeKind::Do { .. }
        | NodeKind::Defer { .. } => {}
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
