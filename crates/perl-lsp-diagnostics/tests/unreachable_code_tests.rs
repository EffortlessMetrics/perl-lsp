//! Tests for unreachable code detection (PL406)
//!
//! Verifies that the unreachable_code lint correctly identifies statements
//! that cannot execute due to unconditional control-flow exits, and does NOT
//! emit false positives for conditional exits or eval boundaries.

use perl_lsp_diagnostics::unreachable_code::check_unreachable_code;
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

fn return_value_node() -> Node {
    Node::new(
        NodeKind::Return {
            value: Some(Box::new(Node::new(
                NodeKind::Number {
                    value: "42".to_string(),
                },
                loc(17, 19),
            ))),
        },
        loc(10, 20),
    )
}

fn print_stmt(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "print".to_string(),
                    args: vec![Node::new(
                        NodeKind::String {
                            value: "dead".to_string(),
                            interpolated: false,
                        },
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
                        NodeKind::String {
                            value: "err".to_string(),
                            interpolated: false,
                        },
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
                        NodeKind::Number {
                            value: "0".to_string(),
                        },
                        loc(start + 5, end - 1),
                    )],
                },
                loc(start, end),
            )),
        },
        loc(start, end),
    )
}

fn croak_qualified_call(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "Carp::croak".to_string(),
                    args: vec![Node::new(
                        NodeKind::String {
                            value: "err".to_string(),
                            interpolated: false,
                        },
                        loc(start + 12, end - 1),
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
                        NodeKind::String {
                            value: "err".to_string(),
                            interpolated: false,
                        },
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
                NodeKind::Variable {
                    sigil: "$".to_string(),
                    name: name.to_string(),
                },
                loc(start + 3, end),
            )),
            attributes: vec![],
            initializer: Some(Box::new(Node::new(
                NodeKind::Number {
                    value: "1".to_string(),
                },
                loc(end - 1, end),
            ))),
        },
        loc(start, end),
    )
}

fn last_stmt(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::LoopControl {
            op: "last".to_string(),
            label: None,
        },
        loc(start, end),
    )
}

fn while_loop(body: Node) -> Node {
    Node::new(
        NodeKind::While {
            condition: Box::new(Node::new(
                NodeKind::Number {
                    value: "1".to_string(),
                },
                loc(7, 8),
            )),
            body: Box::new(body),
            continue_block: None,
        },
        loc(0, 60),
    )
}

fn if_node(condition: Node, then_stmts: Vec<Node>, else_stmts: Option<Vec<Node>>) -> Node {
    Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(block(then_stmts)),
            elsif_branches: vec![],
            else_branch: else_stmts.map(|s| Box::new(block(s))),
        },
        loc(0, 80),
    )
}

fn var_node(name: &str) -> Node {
    Node::new(
        NodeKind::Variable {
            sigil: "$".to_string(),
            name: name.to_string(),
        },
        loc(4, 10),
    )
}

fn eval_block(stmts: Vec<Node>) -> Node {
    Node::new(
        NodeKind::Eval {
            block: Box::new(block(stmts)),
        },
        loc(0, 40),
    )
}

fn eval_stmt(stmts: Vec<Node>) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(eval_block(stmts)),
        },
        loc(0, 42),
    )
}

fn anonymous_sub(body_stmts: Vec<Node>) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block(body_stmts)),
        },
        loc(15, 55),
    )
}

fn has_pl406(diagnostics: &[perl_lsp_diagnostics::Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| d.code.as_deref() == Some("PL406"))
}

fn count_pl406(diagnostics: &[perl_lsp_diagnostics::Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some("PL406"))
        .count()
}

// ---------------------------------------------------------------------------
// T1: return followed by statement in sub body
// "sub foo { return 42; print 'dead'; }"
// expect: 1 diagnostic at the print statement, code PL406
// ---------------------------------------------------------------------------

#[test]
fn t1_return_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![return_value_node(), print_stmt(25, 45)]);
    let ast = program(vec![sub_node("foo", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T1: Expected PL406 for statement after return, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T1: Expected exactly 1 PL406, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// T2: die followed by statement
// "sub bar { die 'err'; print 'dead'; }"
// expect: 1 diagnostic
// ---------------------------------------------------------------------------

#[test]
fn t2_die_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![die_call(10, 25), print_stmt(30, 50)]);
    let ast = program(vec![sub_node("bar", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T2: Expected PL406 for statement after die, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T2: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// T3: exit at top level followed by statement
// "exit(0); print 'dead';"
// expect: 1 diagnostic
// ---------------------------------------------------------------------------

#[test]
fn t3_exit_at_top_level() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![exit_call(0, 10), print_stmt(12, 30)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T3: Expected PL406 for statement after exit, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T3: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// T4: Carp::croak followed by statement
// "sub baz { Carp::croak 'err'; print 'dead'; }"
// expect: 1 diagnostic
// ---------------------------------------------------------------------------

#[test]
fn t4_croak_qualified_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![croak_qualified_call(10, 30), print_stmt(35, 55)]);
    let ast = program(vec![sub_node("baz", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T4: Expected PL406 for statement after Carp::croak, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T4: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// T5: multiple unreachable statements
// "sub f { return; my $x = 1; my $y = 2; print 'dead'; }"
// expect: 3 diagnostics
// ---------------------------------------------------------------------------

#[test]
fn t5_multiple_unreachable_statements() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![
        return_node(),
        my_var_decl(25, 35, "x"),
        my_var_decl(37, 47, "y"),
        print_stmt(49, 65),
    ]);
    let ast = program(vec![sub_node("f", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        3,
        "T5: Expected exactly 3 PL406 diagnostics, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// T6: last inside a loop body
// "while (1) { last; print 'dead'; }"
// expect: 1 diagnostic on print inside loop body
// ---------------------------------------------------------------------------

#[test]
fn t6_last_inside_loop_body() -> Result<(), Box<dyn std::error::Error>> {
    let loop_body = block(vec![last_stmt(13, 18), print_stmt(20, 40)]);
    let ast = program(vec![while_loop(loop_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T6: Expected PL406 for statement after last in loop, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T6: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// T7: unqualified croak
// "sub f { croak 'err'; my $x = 1; }"
// expect: 1 diagnostic
// ---------------------------------------------------------------------------

#[test]
fn t7_unqualified_croak_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![
        croak_unqualified_call(10, 25),
        my_var_decl(27, 37, "x"),
    ]);
    let ast = program(vec![sub_node("f", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T7: Expected PL406 for statement after croak, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T7: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// N1: conditional return via StatementModifier
// "sub foo { return if $cond; print 'reachable'; }"
// expect: 0 diagnostics
// ---------------------------------------------------------------------------

#[test]
fn n1_conditional_return_via_statement_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let conditional_return = Node::new(
        NodeKind::StatementModifier {
            statement: Box::new(return_node()),
            modifier: "if".to_string(),
            condition: Box::new(var_node("cond")),
        },
        loc(10, 30),
    );
    let body = block(vec![conditional_return, print_stmt(32, 52)]);
    let ast = program(vec![sub_node("foo", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N1: Expected 0 PL406 for conditional return, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N2: return in one branch only -- sibling print is still reachable
// "sub foo { if ($x) { return; } print 'reachable'; }"
// expect: 0 diagnostics on print
// ---------------------------------------------------------------------------

#[test]
fn n2_return_in_one_branch_only() -> Result<(), Box<dyn std::error::Error>> {
    let if_stmt = if_node(var_node("x"), vec![return_node()], None);
    let body = block(vec![if_stmt, print_stmt(40, 60)]);
    let ast = program(vec![sub_node("foo", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N2: Expected 0 PL406 for return in one branch only, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N3: eval block containing die -- outer scope continues
// "eval { die 'err' }; print 'reachable';"
// expect: 0 diagnostics on print
// ---------------------------------------------------------------------------

#[test]
fn n3_eval_block_containing_die() -> Result<(), Box<dyn std::error::Error>> {
    let die_inside_eval = die_call(7, 20);
    let ast = program(vec![eval_stmt(vec![die_inside_eval]), print_stmt(45, 65)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N3: Expected 0 PL406 for die inside eval, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N4: nested sub return does not poison outer scope
// "sub outer { my $f = sub { return 1; }; print 'reachable'; }"
// expect: 0 diagnostics on print in outer sub
// ---------------------------------------------------------------------------

#[test]
fn n4_nested_sub_return_no_false_positive() -> Result<(), Box<dyn std::error::Error>> {
    let inner_return = return_value_node();
    let inner_sub = anonymous_sub(vec![inner_return]);

    let my_f = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(Node::new(
                NodeKind::Variable {
                    sigil: "$".to_string(),
                    name: "f".to_string(),
                },
                loc(13, 15),
            )),
            attributes: vec![],
            initializer: Some(Box::new(inner_sub)),
        },
        loc(12, 56),
    );

    let body = block(vec![my_f, print_stmt(58, 78)]);
    let ast = program(vec![sub_node("outer", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N4: Expected 0 PL406: nested sub return should not poison outer scope, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N5: return at end of block (nothing after it)
// "sub foo { print 'ok'; return 1; }"
// expect: 0 diagnostics
// ---------------------------------------------------------------------------

#[test]
fn n5_return_at_end_of_block() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![print_stmt(10, 25), return_value_node()]);
    let ast = program(vec![sub_node("foo", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N5: Expected 0 PL406 for return at end of block, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N6: die inside 'or' short-circuit -- not a direct statement exit
// "open(my $fh, '<', $f) or die 'err'; print 'reachable';"
// expect: 0 diagnostics on print
// ---------------------------------------------------------------------------

#[test]
fn n6_die_inside_or_not_unconditional() -> Result<(), Box<dyn std::error::Error>> {
    // The `or die` pattern: the `die` is the right operand of a Binary `or`
    // expression, which is itself inside an ExpressionStatement. This is NOT
    // an unconditional exit at the statement level.
    let die_expr = Node::new(
        NodeKind::FunctionCall {
            name: "die".to_string(),
            args: vec![Node::new(
                NodeKind::String {
                    value: "err".to_string(),
                    interpolated: false,
                },
                loc(26, 31),
            )],
        },
        loc(22, 32),
    );
    let open_or_die = Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::Binary {
                    op: "or".to_string(),
                    left: Box::new(Node::new(
                        NodeKind::FunctionCall {
                            name: "open".to_string(),
                            args: vec![],
                        },
                        loc(0, 21),
                    )),
                    right: Box::new(die_expr),
                },
                loc(0, 32),
            )),
        },
        loc(0, 33),
    );

    let ast = program(vec![open_or_die, print_stmt(35, 55)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N6: Expected 0 PL406 for die inside 'or' expression, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// T8: next inside a loop body
// "while (1) { next; print 'dead'; }"
// expect: 1 diagnostic on print inside loop body
// ---------------------------------------------------------------------------

#[test]
fn t8_next_inside_loop_body() -> Result<(), Box<dyn std::error::Error>> {
    let next_stmt = Node::new(
        NodeKind::LoopControl {
            op: "next".to_string(),
            label: None,
        },
        loc(13, 18),
    );
    let loop_body = block(vec![next_stmt, print_stmt(20, 40)]);
    let ast = program(vec![while_loop(loop_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T8: Expected PL406 for statement after next in loop, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T8: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// T9: redo inside a loop body
// "while (1) { redo; print 'dead'; }"
// expect: 1 diagnostic on print inside loop body
// ---------------------------------------------------------------------------

#[test]
fn t9_redo_inside_loop_body() -> Result<(), Box<dyn std::error::Error>> {
    let redo_stmt = Node::new(
        NodeKind::LoopControl {
            op: "redo".to_string(),
            label: None,
        },
        loc(13, 18),
    );
    let loop_body = block(vec![redo_stmt, print_stmt(20, 40)]);
    let ast = program(vec![while_loop(loop_body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T9: Expected PL406 for statement after redo in loop, got: {:?}",
        diagnostics
    );
    assert_eq!(count_pl406(&diagnostics), 1, "T9: Expected exactly 1 PL406");
    Ok(())
}

// ---------------------------------------------------------------------------
// N7: confess (Carp::confess) followed by statement
// "sub f { Carp::confess 'err'; my $x = 1; }"
// expect: 1 diagnostic
// ---------------------------------------------------------------------------

#[test]
fn t10_confess_qualified_followed_by_statement() -> Result<(), Box<dyn std::error::Error>> {
    let confess_call = Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "Carp::confess".to_string(),
                    args: vec![Node::new(
                        NodeKind::String {
                            value: "err".to_string(),
                            interpolated: false,
                        },
                        loc(14, 19),
                    )],
                },
                loc(10, 30),
            )),
        },
        loc(10, 31),
    );
    let body = block(vec![confess_call, my_var_decl(33, 43, "x")]);
    let ast = program(vec![sub_node("f", body)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    assert!(
        has_pl406(&diagnostics),
        "T10: Expected PL406 for statement after Carp::confess, got: {:?}",
        diagnostics
    );
    assert_eq!(
        count_pl406(&diagnostics),
        1,
        "T10: Expected exactly 1 PL406"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// N7: goto LABEL does not currently emit PL406 (known false negative,
// tracked for future work — goto is rare in modern Perl)
// This test documents the current behavior.
// ---------------------------------------------------------------------------

#[test]
fn n7_goto_label_not_yet_detected() -> Result<(), Box<dyn std::error::Error>> {
    let goto_stmt = Node::new(
        NodeKind::Goto {
            target: Box::new(Node::new(
                NodeKind::Identifier {
                    name: "END".to_string(),
                },
                loc(5, 8),
            )),
        },
        loc(0, 9),
    );
    let ast = program(vec![goto_stmt, print_stmt(12, 30)]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    // goto is not yet in the unconditional-exit list.
    // This test documents the current false-negative, not desired behavior.
    assert_eq!(
        count_pl406(&diagnostics),
        0,
        "N7: goto false-negative documented — no PL406 yet, got: {:?}",
        diagnostics
    );
    Ok(())
}
