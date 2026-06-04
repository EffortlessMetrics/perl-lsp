//! Tests for unreachable code detection in continue blocks (PL406)
//!
//! Verifies that the unreachable_code lint correctly identifies statements
//! that cannot execute due to unconditional control-flow exits inside
//! `continue { }` blocks, and does NOT emit false positives for `next`/`redo`
//! in continue blocks (which do not terminate the continue block's execution
//! in the same way as `last`, `die`, `return`, etc.).
//!
//! Extracted from PR #4488 (feat: unreachable code detection in continue blocks,
//! issue #3374). The original test file targeted `perl-lsp-diagnostics` which
//! has since been absorbed into `perl-lsp-rs-core`.

use perl_lsp_rs_core::providers::diagnostics::Diagnostic;
use perl_lsp_rs_core::providers::diagnostics::unreachable_code::check_unreachable_code;
use perl_parser_core::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn program(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements: stmts }, loc(0, 200))
}

fn block(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements: stmts }, loc(0, 100))
}

fn sub_node(name: &str, body: Node) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: Some(name.to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 100),
    )
}

fn return_node() -> Node {
    Node::new(NodeKind::Return { value: None }, loc(10, 20))
}

fn print_stmt(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "print".to_string(),
                    args: vec![Node::new(
                        NodeKind::String { value: "dead".to_string(), interpolated: false },
                        loc(start + 6, end - 1),
                    )],
                },
                loc(start, end),
            )),
        },
        loc(start, end),
    )
}

fn die_call(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "die".to_string(),
                    args: vec![Node::new(
                        NodeKind::String { value: "err".to_string(), interpolated: false },
                        loc(start + 4, end - 1),
                    )],
                },
                loc(start, end),
            )),
        },
        loc(start, end),
    )
}

fn exit_call(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "exit".to_string(),
                    args: vec![Node::new(
                        NodeKind::Number { value: "0".to_string() },
                        loc(start + 5, end - 1),
                    )],
                },
                loc(start, end),
            )),
        },
        loc(start, end),
    )
}

fn croak_unqualified_call(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "croak".to_string(),
                    args: vec![Node::new(
                        NodeKind::String { value: "err".to_string(), interpolated: false },
                        loc(start + 6, end - 1),
                    )],
                },
                loc(start, end),
            )),
        },
        loc(start, end),
    )
}

fn my_var_decl(start: usize, end: usize, name: &str) -> Node {
    Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: name.to_string() },
                loc(start + 3, end),
            )),
            attributes: vec![],
            initializer: Some(Box::new(Node::new(
                NodeKind::Number { value: "1".to_string() },
                loc(end - 1, end),
            ))),
        },
        loc(start, end),
    )
}

fn last_stmt(start: usize, end: usize) -> Node {
    Node::new(NodeKind::LoopControl { op: "last".to_string(), label: None }, loc(start, end))
}

fn next_stmt(start: usize, end: usize) -> Node {
    Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(start, end))
}

fn redo_stmt(start: usize, end: usize) -> Node {
    Node::new(NodeKind::LoopControl { op: "redo".to_string(), label: None }, loc(start, end))
}

fn while_loop(body: Node) -> Node {
    Node::new(
        NodeKind::While {
            keyword: None,
            condition: Box::new(Node::new(NodeKind::Number { value: "1".to_string() }, loc(7, 8))),
            body: Box::new(body),
            continue_block: None,
        },
        loc(0, 60),
    )
}

fn while_loop_with_continue(body: Node, continue_body: Node) -> Node {
    Node::new(
        NodeKind::While {
            keyword: None,
            condition: Box::new(Node::new(NodeKind::Number { value: "1".to_string() }, loc(7, 8))),
            body: Box::new(body),
            continue_block: Some(Box::new(continue_body)),
        },
        loc(0, 120),
    )
}

fn for_loop_with_continue(body: Node, continue_body: Node) -> Node {
    Node::new(
        NodeKind::For {
            init: Some(Box::new(Node::new(
                NodeKind::VariableDeclaration {
                    declarator: "my".to_string(),
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "i".to_string() },
                        loc(5, 7),
                    )),
                    attributes: vec![],
                    initializer: Some(Box::new(Node::new(
                        NodeKind::Number { value: "0".to_string() },
                        loc(8, 9),
                    ))),
                },
                loc(0, 9),
            ))),
            condition: Some(Box::new(Node::new(
                NodeKind::Number { value: "1".to_string() },
                loc(10, 11),
            ))),
            update: Some(Box::new(Node::new(
                NodeKind::Unary {
                    op: "++".to_string(),
                    operand: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "i".to_string() },
                        loc(12, 14),
                    )),
                },
                loc(12, 14),
            ))),
            body: Box::new(body),
            continue_block: Some(Box::new(continue_body)),
        },
        loc(0, 140),
    )
}

fn foreach_loop_with_continue(body: Node, continue_body: Node) -> Node {
    Node::new(
        NodeKind::Foreach {
            variable: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(8, 10),
            )),
            list: Box::new(Node::new(
                NodeKind::ArrayLiteral {
                    elements: vec![
                        Node::new(NodeKind::Number { value: "1".to_string() }, loc(15, 16)),
                        Node::new(NodeKind::Number { value: "2".to_string() }, loc(18, 19)),
                    ],
                },
                loc(14, 20),
            )),
            body: Box::new(body),
            continue_block: Some(Box::new(continue_body)),
        },
        loc(0, 140),
    )
}

fn has_pl406(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.code.as_deref() == Some("PL406"))
}

fn count_pl406(diagnostics: &[Diagnostic]) -> usize {
    diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL406")).count()
}

// ===========================================================================
// Continue block tests (T-continue-1 through T-continue-10, N-continue-1, N-continue-2)
// ===========================================================================

// --------------------------------------------------------------------------
// T-continue-1: die in continue block followed by statement
// "while (1) { } continue { die 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_1_die_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: die "err"; print "dead";
    let continue_body = block(vec![die_call(20, 35), print_stmt(37, 57)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-1: Expected PL406 for statement after die in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-1: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-2: exit in continue block followed by statement
// "while (1) { } continue { exit(0); print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_2_exit_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: exit(0); print "dead";
    let continue_body = block(vec![exit_call(20, 30), print_stmt(32, 52)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-2: Expected PL406 for statement after exit in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-2: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-3: croak in continue block followed by statement
// "while (1) { } continue { croak 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_3_croak_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: croak "err"; print "dead";
    let continue_body = block(vec![croak_unqualified_call(20, 35), print_stmt(37, 57)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-3: Expected PL406 for statement after croak in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-3: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-4: last in continue block followed by statement
// "while (1) { } continue { last; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement (last exits the entire loop)
// --------------------------------------------------------------------------

#[test]
fn t_continue_4_last_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: last; print "dead";
    let continue_body = block(vec![last_stmt(20, 25), print_stmt(27, 47)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-4: Expected PL406 for statement after last in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-4: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-5: return in continue block followed by statement (in sub context)
// "sub f { while (1) { } continue { return; print 'dead'; } }"
// expect: 1 PL406 diagnostic on the print statement
// Note: return in continue block exits the containing sub
// --------------------------------------------------------------------------

#[test]
fn t_continue_5_return_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: return; print "dead";
    let continue_body = block(vec![return_node(), print_stmt(30, 50)]);
    let loop_with_continue = while_loop_with_continue(block(vec![]), continue_body);
    let ast = program(vec![sub_node("f", block(vec![loop_with_continue]))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-5: Expected PL406 for statement after return in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-5: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// N-continue-1: next in continue block followed by statement — NO false positive
// "while (1) { } continue { next; print 'reachable'; }"
// expect: 0 PL406 diagnostics (next jumps to next iteration, continue block re-runs)
// --------------------------------------------------------------------------

#[test]
fn n_continue_1_next_in_continue_block_no_false_positive() -> Result<(), Box<dyn std::error::Error>>
{
    // continue block: next; print "reachable";
    let continue_body = block(vec![next_stmt(20, 25), print_stmt(27, 47)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N-continue-1: Expected 0 PL406 for next in continue block (next re-runs continue), got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// N-continue-2: redo in continue block followed by statement — NO false positive
// "while (1) { } continue { redo; print 'reachable'; }"
// expect: 0 PL406 diagnostics (redo re-runs the continue block)
// --------------------------------------------------------------------------

#[test]
fn n_continue_2_redo_in_continue_block_no_false_positive() -> Result<(), Box<dyn std::error::Error>>
{
    // continue block: redo; print "reachable";
    let continue_body = block(vec![redo_stmt(20, 25), print_stmt(27, 47)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N-continue-2: Expected 0 PL406 for redo in continue block (redo re-runs continue), got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-8: multiple unreachable statements in continue block
// "while (1) { } continue { die 'err'; my $x = 1; my $y = 2; print 'dead'; }"
// expect: 3 PL406 diagnostics (one each for $x, $y, and print)
// --------------------------------------------------------------------------

#[test]
fn t_continue_8_multiple_unreachable_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: die "err"; my $x = 1; my $y = 2; print "dead";
    let continue_body = block(vec![
        die_call(20, 35),
        my_var_decl(37, 47, "x"),
        my_var_decl(49, 59, "y"),
        print_stmt(61, 81),
    ]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        3,
        "T-continue-8: Expected exactly 3 PL406 diagnostics for multiple unreachable statements, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-9: loop body unreachable detection unchanged
// "while (1) { die 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on print in the loop body (not in continue block)
// --------------------------------------------------------------------------

#[test]
fn t_continue_9_loop_body_detection_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    // Loop body: die "err"; print "dead";
    // The die and print are in the loop body, not the continue block
    let loop_body = block(vec![die_call(20, 35), print_stmt(37, 57)]);
    let ast = program(vec![while_loop(loop_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-9: Expected PL406 for statement after die in loop body, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-9: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-10: confess (Carp::confess) in continue block followed by statement
// "while (1) { } continue { Carp::confess 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_10_confess_in_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    // continue block: Carp::confess "err"; print "dead";
    let confess_call = Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "Carp::confess".to_string(),
                    args: vec![Node::new(
                        NodeKind::String { value: "err".to_string(), interpolated: false },
                        loc(25, 30),
                    )],
                },
                loc(20, 40),
            )),
        },
        loc(20, 41),
    );
    let continue_body = block(vec![confess_call, print_stmt(43, 63)]);
    let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-10: Expected PL406 for statement after confess in continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-10: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-11: die in for-loop continue block followed by statement
// "for (...) { } continue { die 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_11_die_in_for_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    let continue_body = block(vec![die_call(20, 35), print_stmt(37, 57)]);
    let ast = program(vec![for_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-11: Expected PL406 for statement after die in for-loop continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-11: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}

// --------------------------------------------------------------------------
// T-continue-12: die in foreach-loop continue block followed by statement
// "foreach (...) { } continue { die 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on the print statement
// --------------------------------------------------------------------------

#[test]
fn t_continue_12_die_in_foreach_continue_block() -> Result<(), Box<dyn std::error::Error>> {
    let continue_body = block(vec![die_call(20, 35), print_stmt(37, 57)]);
    let ast = program(vec![foreach_loop_with_continue(block(vec![]), continue_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T-continue-12: Expected PL406 for statement after die in foreach-loop continue block, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T-continue-12: Expected exactly 1 PL406 diagnostic, got: {:?}",
        diagnostics
    );
    Ok(())
}
