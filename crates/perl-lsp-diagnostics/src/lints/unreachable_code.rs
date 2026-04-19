//! Unreachable code detection (PL406)
//!
//! Identifies statements that cannot execute because they follow an unconditional
//! control-flow exit (`return`, `die`, `exit`, `croak`, `confess`, `last`,
//! `next`, `redo`).
//!
//! # Algorithm
//!
//! The lint uses **recursive statement-slice analysis** rather than a flat
//! pre-order AST walk. This is the only correct approach: a pre-order visitor
//! with a `reachable: bool` flag cannot distinguish "visiting a child of this
//! node" from "visiting the next sibling", so a `return` inside a nested
//! subroutine body would incorrectly poison sibling statements in the outer
//! scope.
//!
//! The correct algorithm:
//! 1. `check_unreachable_code` dispatches on the root, then calls `visit_node`
//!    for each statement in top-level lists.
//! 2. `check_statement_list` iterates a `&[Node]` linearly. When an
//!    unconditional exit is found, all subsequent siblings get a PL406
//!    diagnostic. Nested blocks are recursed into freshly.
//! 3. Subroutine and method bodies (`Subroutine`, `Method`) trigger a fresh
//!    call to `visit_node`, so a `return` in an inner sub never affects the
//!    outer statement list.
//! 4. `eval { }` blocks are intentionally **not** recursed into: `die` inside
//!    `eval { }` is caught and does not exit the outer scope.
//!
//! # Scope of detection
//!
//! | Unconditional exit | Detected? |
//! |--------------------|-----------|
//! | `return`           | Yes |
//! | `die "msg"`        | Yes (direct FunctionCall at statement level) |
//! | `exit $code`       | Yes |
//! | `croak "msg"`      | Yes |
//! | `Carp::croak "msg"` | Yes |
//! | `confess "msg"`    | Yes |
//! | `Carp::confess "msg"` | Yes |
//! | `last` in loop body | Yes |
//! | `next` in loop body | Yes |
//! | `redo` in loop body | Yes |
//! | `return if $cond`  | No (conditional via StatementModifier) |
//! | `die` inside `or`  | No (right operand of Binary, not a direct statement) |
//! | `die` inside `eval { }` | No (caught by eval) |

use crate::internal_types::{Diagnostic, DiagnosticTag};
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};

/// Entry point for unreachable code detection.
///
/// Walk the AST and emit `PL406` diagnostics for any statements that cannot
/// be reached due to a preceding unconditional control-flow exit.
pub fn check_unreachable_code(root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    visit_node(root, diagnostics);
}

/// Dispatch on a single node: recurse into any block-like children using fresh
/// reachability state, and process statement lists as slices.
fn visit_node(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    match &node.kind {
        // Top-level program: walk all top-level statements as a slice
        NodeKind::Program { statements } => {
            check_statement_list(statements, diagnostics);
        }

        // Subroutine body: fresh reachability scope — return here does not
        // affect the outer statement list
        NodeKind::Subroutine { body, .. } | NodeKind::Method { body, .. } => {
            visit_node(body, diagnostics);
        }

        // Plain block: walk its statements as a slice
        NodeKind::Block { statements } => {
            check_statement_list(statements, diagnostics);
        }

        // If/unless: each branch is an independent scope
        NodeKind::If {
            then_branch,
            elsif_branches,
            else_branch,
            ..
        } => {
            visit_node(then_branch, diagnostics);
            for (_, branch_body) in elsif_branches {
                visit_node(branch_body, diagnostics);
            }
            if let Some(else_body) = else_branch {
                visit_node(else_body, diagnostics);
            }
        }

        // Loop bodies: each body is an independent scope
        NodeKind::While { body, .. }
        | NodeKind::For { body, .. }
        | NodeKind::Foreach { body, .. } => {
            visit_node(body, diagnostics);
        }

        // Given/when/default
        NodeKind::Given { body, .. } | NodeKind::When { body, .. } | NodeKind::Default { body } => {
            visit_node(body, diagnostics);
        }

        // PhaseBlock (BEGIN, END, etc.): walk its block
        NodeKind::PhaseBlock { block, .. } => {
            visit_node(block, diagnostics);
        }

        // Class body
        NodeKind::Class { body, .. } => {
            visit_node(body, diagnostics);
        }

        // Do block: fresh scope (do { ... })
        NodeKind::Do { block } | NodeKind::Defer { block } => {
            visit_node(block, diagnostics);
        }

        // Try body and catch blocks: each is an independent scope
        NodeKind::Try {
            body,
            catch_blocks,
            finally_block,
        } => {
            visit_node(body, diagnostics);
            for (_, catch_body) in catch_blocks {
                visit_node(catch_body, diagnostics);
            }
            if let Some(finally) = finally_block {
                visit_node(finally, diagnostics);
            }
        }

        // ExpressionStatement: recurse into the expression to catch nested
        // subroutine literals (e.g., `my $f = sub { return 1; };`)
        NodeKind::ExpressionStatement { expression } => {
            visit_expr(expression, diagnostics);
        }

        // Variable declarations with initializers may contain anonymous subs
        NodeKind::VariableDeclaration {
            initializer: Some(init),
            ..
        }
        | NodeKind::VariableListDeclaration {
            initializer: Some(init),
            ..
        } => {
            visit_expr(init, diagnostics);
        }

        // Eval: intentionally NOT recursed into.
        // die inside eval { } is caught — the outer scope continues normally.
        NodeKind::Eval { .. } => {}

        // LabeledStatement: recurse into the inner statement
        NodeKind::LabeledStatement { statement, .. } => {
            visit_node(statement, diagnostics);
        }

        // All other nodes have no statement-list children
        _ => {}
    }
}

/// Recursively visit expression nodes looking for anonymous subroutine literals
/// (so that `return` inside an anonymous sub does not appear to be a direct
/// child of the outer statement list).
fn visit_expr(expr: &Node, diagnostics: &mut Vec<Diagnostic>) {
    match &expr.kind {
        // Anonymous sub literal: fresh reachability scope
        NodeKind::Subroutine { body, .. } => {
            visit_node(body, diagnostics);
        }

        // Walk children of common expression forms
        NodeKind::Assignment { lhs, rhs, .. } => {
            visit_expr(lhs, diagnostics);
            visit_expr(rhs, diagnostics);
        }
        NodeKind::Binary { left, right, .. } => {
            visit_expr(left, diagnostics);
            visit_expr(right, diagnostics);
        }
        NodeKind::Unary { operand, .. } => {
            visit_expr(operand, diagnostics);
        }
        NodeKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            visit_expr(condition, diagnostics);
            visit_expr(then_expr, diagnostics);
            visit_expr(else_expr, diagnostics);
        }
        NodeKind::FunctionCall { args, .. } | NodeKind::MethodCall { args, .. } => {
            for arg in args {
                visit_expr(arg, diagnostics);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for elem in elements {
                visit_expr(elem, diagnostics);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, val) in pairs {
                visit_expr(key, diagnostics);
                visit_expr(val, diagnostics);
            }
        }
        // Other expression forms don't contain sub literals; stop recursing
        _ => {}
    }
}

/// Walk a statement slice linearly. When an unconditional exit is found, emit
/// PL406 for all remaining siblings in the same slice.
///
/// The key correctness property: after calling `check_statement_list`, nested
/// blocks are always entered with a *fresh* call to `visit_node`, which starts
/// with `found_exit = false`. This prevents a `return` in an inner sub from
/// poisoning the outer statement list.
fn check_statement_list(stmts: &[Node], diagnostics: &mut Vec<Diagnostic>) {
    let mut found_exit = false;

    for stmt in stmts {
        if found_exit {
            // Emit PL406 for this unreachable statement
            diagnostics.push(Diagnostic {
                range: (stmt.location.start, stmt.location.end),
                severity: DiagnosticSeverity::Hint,
                code: Some(DiagnosticCode::UnreachableCode.as_str().to_string()),
                message: "Unreachable code: this statement cannot be executed".to_string(),
                related_information: vec![],
                tags: vec![DiagnosticTag::Unnecessary],
                suggestion: Some("Remove unreachable code".to_string()),
            });
            // Still recurse into the unreachable node: nested subs deserve
            // independent analysis even if their containing block is dead.
            visit_node(stmt, diagnostics);
        } else {
            // Recurse first (to handle nested subs), then check for exit
            visit_node(stmt, diagnostics);
            if is_unconditional_exit(stmt) {
                found_exit = true;
            }
        }
    }
}

/// Returns true if this AST node represents an unconditional control-flow exit.
///
/// Only nodes that **directly exit** at the statement level qualify. The key
/// restriction is that `die` inside `or` (a binary expression) does NOT count
/// because the `or` branch is only taken when the left side is falsy — the
/// overall statement does not always exit.
///
/// `StatementModifier` is explicitly `false`: `return if $cond` is conditional.
fn is_unconditional_exit(node: &Node) -> bool {
    match &node.kind {
        // `return;` or `return $value;`
        NodeKind::Return { .. } => true,

        // Direct function call at statement level (not wrapped in ExpressionStatement)
        NodeKind::FunctionCall { name, .. } => is_exit_function(name),

        // `die "msg";` — the parser wraps bare function calls in ExpressionStatement
        NodeKind::ExpressionStatement { expression } => is_unconditional_exit(expression),

        // `last`, `next`, `redo` — exit the current loop iteration/block
        NodeKind::LoopControl { op, .. } => matches!(op.as_str(), "last" | "next" | "redo"),

        // `return if $cond` is CONDITIONAL — StatementModifier is never an unconditional exit
        NodeKind::StatementModifier { .. } => false,

        _ => false,
    }
}

/// Returns true if the function name is one of the known unconditional-exit functions.
fn is_exit_function(name: &str) -> bool {
    matches!(
        name,
        "die" | "exit" | "croak" | "Carp::croak" | "confess" | "Carp::confess"
    )
}
