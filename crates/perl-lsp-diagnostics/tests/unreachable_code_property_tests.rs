//! Property-based tests for unreachable code detection (PL406)
//!
//! These tests verify invariants that should hold for all inputs, not just
//! specific examples.

use perl_lsp_diagnostics::DiagnosticTag;
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

fn croak_call(start: usize, end: usize) -> Node {
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

fn confess_call(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::FunctionCall {
                    name: "Carp::confess".to_string(),
                    args: vec![Node::new(
                        NodeKind::String { value: "err".to_string(), interpolated: false },
                        loc(start + 14, end - 1),
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
            init: None,
            condition: None,
            update: None,
            body: Box::new(body),
            continue_block: Some(Box::new(continue_body)),
        },
        loc(0, 120),
    )
}

fn foreach_loop_with_continue(body: Node, continue_body: Node) -> Node {
    Node::new(
        NodeKind::Foreach {
            variable: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "item".to_string() },
                loc(9, 11),
            )),
            list: Box::new(Node::new(
                NodeKind::Variable { sigil: "@".to_string(), name: "list".to_string() },
                loc(15, 20),
            )),
            body: Box::new(body),
            continue_block: Some(Box::new(continue_body)),
        },
        loc(0, 120),
    )
}

fn count_pl406(diagnostics: &[perl_lsp_diagnostics::Diagnostic]) -> usize {
    diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL406")).count()
}

// ---------------------------------------------------------------------------
// Property 1: No false positives in unconditional exit list
// For each exit type (die, exit, croak, confess, return, last),
// the number of PL406 diagnostics should equal the number of statements
// that DIRECTLY follow the exit in the same statement list.
// ---------------------------------------------------------------------------

/// Property: Each unconditional exit type produces exactly N diagnostics
/// for N statements following it in a flat statement list.
#[test]
fn property_die_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let mut stmts = vec![];
        let exit_call = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall {
                        name: "die".to_string(),
                        args: vec![Node::new(
                            NodeKind::String { value: "err".to_string(), interpolated: false },
                            loc(5, 10),
                        )],
                    },
                    loc(0, 15),
                )),
            },
            loc(0, 16),
        );
        stmts.push(exit_call);

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 20 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: die with {} following statements: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_exit_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let mut stmts = vec![];
        let exit_call = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall {
                        name: "exit".to_string(),
                        args: vec![Node::new(
                            NodeKind::Number { value: "0".to_string() },
                            loc(5, 6),
                        )],
                    },
                    loc(0, 10),
                )),
            },
            loc(0, 11),
        );
        stmts.push(exit_call);

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 20 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: exit with {} following statements: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_croak_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let mut stmts = vec![];
        let exit_call = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall {
                        name: "croak".to_string(),
                        args: vec![Node::new(
                            NodeKind::String { value: "err".to_string(), interpolated: false },
                            loc(7, 11),
                        )],
                    },
                    loc(0, 16),
                )),
            },
            loc(0, 17),
        );
        stmts.push(exit_call);

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 25 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: croak with {} following statements: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_confess_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let mut stmts = vec![];
        let exit_call = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall {
                        name: "Carp::confess".to_string(),
                        args: vec![Node::new(
                            NodeKind::String { value: "err".to_string(), interpolated: false },
                            loc(14, 18),
                        )],
                    },
                    loc(0, 23),
                )),
            },
            loc(0, 24),
        );
        stmts.push(exit_call);

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 30 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: confess with {} following statements: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_return_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let mut stmts = vec![];
        stmts.push(return_node());

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 25 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: return with {} following statements: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_last_loop_control_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let ctrl =
            Node::new(NodeKind::LoopControl { op: "last".to_string(), label: None }, loc(10, 15));

        let mut stmts = vec![ctrl];
        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 20 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![while_loop(block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: last in loop body with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_next_loop_control_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let ctrl =
            Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(10, 15));

        let mut stmts = vec![ctrl];
        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 20 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![while_loop(block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: next in loop body with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_redo_loop_control_produces_correct_diagnostic_count() {
    for num_following in 0..=10 {
        let ctrl =
            Node::new(NodeKind::LoopControl { op: "redo".to_string(), label: None }, loc(10, 15));

        let mut stmts = vec![ctrl];
        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 20 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let ast = program(vec![while_loop(block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P1: redo in loop body with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

// ---------------------------------------------------------------------------
// Property 2: No false positives — conditional exits produce 0 diagnostics
// ---------------------------------------------------------------------------

#[test]
fn property_conditional_die_no_false_positive() {
    for num_following in 0..=5 {
        let exit_call = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::FunctionCall {
                        name: "die".to_string(),
                        args: vec![Node::new(
                            NodeKind::String { value: "err".to_string(), interpolated: false },
                            loc(10, 15),
                        )],
                    },
                    loc(5, 20),
                )),
            },
            loc(5, 21),
        );

        let conditional_exit = Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(exit_call),
                modifier: "if".to_string(),
                condition: Box::new(Node::new(
                    NodeKind::Variable { sigil: "$".to_string(), name: "cond".to_string() },
                    loc(26, 31),
                )),
            },
            loc(5, 32),
        );

        let mut stmts = vec![conditional_exit];
        for i in 0..num_following {
            let start = 35 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
        }

        let ast = program(vec![sub_node("foo", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P2: conditional die should produce 0 PL406, got {}",
            actual_count
        );
    }
}

#[test]
fn property_conditional_return_no_false_positive() {
    let conditional_return = Node::new(
        NodeKind::StatementModifier {
            statement: Box::new(return_node()),
            modifier: "if".to_string(),
            condition: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "cond".to_string() },
                loc(22, 27),
            )),
        },
        loc(10, 28),
    );

    let stmts = vec![conditional_return, print_stmt(30, 50)];
    let ast = program(vec![sub_node("foo", block(stmts))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);
    let actual_count = count_pl406(&diagnostics);

    assert_eq!(
        actual_count, 0,
        "P2: conditional return should produce 0 PL406, got {}",
        actual_count
    );
}

// ---------------------------------------------------------------------------
// Property 3: eval boundary — die inside eval does NOT poison outer scope
// ---------------------------------------------------------------------------

#[test]
fn property_eval_contains_die_no_false_positive() {
    for num_following in 0..=5 {
        let die_inside = die_call(7, 20);
        let eval_stmt = Node::new(
            NodeKind::ExpressionStatement {
                expression: Box::new(Node::new(
                    NodeKind::Eval { block: Box::new(block(vec![die_inside])) },
                    loc(0, 22),
                )),
            },
            loc(0, 23),
        );

        let mut stmts = vec![eval_stmt];
        for i in 0..num_following {
            let start = 30 + i * 20;
            stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
        }

        let ast = program(stmts);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P3: eval containing die with {} following: expected 0 PL406, got {}",
            num_following, actual_count
        );
    }
}

// ---------------------------------------------------------------------------
// Property 4: Nested sub boundary — return in nested sub does NOT poison
// the outer statement list
// ---------------------------------------------------------------------------

#[test]
fn property_nested_sub_contains_return_no_false_positive() {
    for num_following in 0..=5 {
        let inner_return = return_node();
        let inner_sub = Node::new(
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block(vec![inner_return])),
            },
            loc(15, 40),
        );
        let my_f = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(Node::new(
                    NodeKind::Variable { sigil: "$".to_string(), name: "f".to_string() },
                    loc(13, 15),
                )),
                attributes: vec![],
                initializer: Some(Box::new(inner_sub)),
            },
            loc(12, 45),
        );

        let mut stmts = vec![my_f];
        for i in 0..num_following {
            let start = 50 + i * 20;
            stmts.push(print_stmt(start, start + 20));
        }

        let ast = program(vec![sub_node("outer", block(stmts))]);
        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P4: nested sub with return, {} following: expected 0 PL406, got {}",
            num_following, actual_count
        );
    }
}

// ---------------------------------------------------------------------------
// Property 5: Continue block — exit in continue block produces diagnostics,
// but next/redo do NOT (they re-run the continue block)
// ---------------------------------------------------------------------------

#[test]
fn property_continue_block_die_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(die_call(20, 35));

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 40 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: die in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_exit_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(exit_call(20, 30));

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 35 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: exit in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_croak_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(croak_call(20, 35));

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 40 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: croak in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_confess_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(confess_call(20, 40));

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 45 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: confess in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_return_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(return_node());

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 30 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: return in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_last_produces_diagnostics() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(last_stmt(20, 25));

        let mut expected_count = 0;
        for i in 0..num_following {
            let start = 30 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
            expected_count += 1;
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, expected_count,
            "P5: last in continue block with {} following: expected {} PL406, got {}",
            num_following, expected_count, actual_count
        );
    }
}

#[test]
fn property_continue_block_next_no_false_positive() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(next_stmt(20, 25));

        for i in 0..num_following {
            let start = 30 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P5: next in continue block with {} following: expected 0 PL406, got {}",
            num_following, actual_count
        );
    }
}

#[test]
fn property_continue_block_redo_no_false_positive() {
    for num_following in 0..=5 {
        let mut continue_stmts = vec![];
        continue_stmts.push(redo_stmt(20, 25));

        for i in 0..num_following {
            let start = 30 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(block(vec![]), continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P5: redo in continue block with {} following: expected 0 PL406, got {}",
            num_following, actual_count
        );
    }
}

// ---------------------------------------------------------------------------
// Property 6: Branch independence — exit in one branch of if/elsif/else
// does NOT affect statements in other branches
// ---------------------------------------------------------------------------

#[test]
fn property_if_branch_independence() {
    // Case: return in then-branch, statement in else-branch — 0 PL406
    let then_branch = block(vec![return_node()]);
    let else_body = block(vec![print_stmt(50, 70)]);

    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(4, 6),
            )),
            then_branch: Box::new(then_branch),
            elsif_branches: vec![],
            else_branch: Some(Box::new(else_body)),
        },
        loc(0, 80),
    );

    let ast = program(vec![sub_node("foo", block(vec![if_node]))]);
    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);
    let actual_count = count_pl406(&diagnostics);

    assert_eq!(
        actual_count, 0,
        "P6: return in then-branch should not affect else-branch: expected 0 PL406, got {}",
        actual_count
    );
}

// ---------------------------------------------------------------------------
// Property 7: Loop body vs continue block are independent scopes
// An exit in the loop body does NOT affect the continue block
// ---------------------------------------------------------------------------

#[test]
fn property_loop_body_does_not_affect_continue_block() {
    for num_continue_stmts in 0..=5 {
        let loop_body = block(vec![die_call(10, 25)]);
        let mut continue_stmts = vec![];
        for i in 0..num_continue_stmts {
            let start = 40 + i * 20;
            continue_stmts.push(my_var_decl(start, start + 10, &format!("x{}", i)));
        }

        let continue_body = block(continue_stmts);
        let ast = program(vec![while_loop_with_continue(loop_body, continue_body)]);

        let mut diagnostics = vec![];
        check_unreachable_code(&ast, &mut diagnostics);
        let actual_count = count_pl406(&diagnostics);

        assert_eq!(
            actual_count, 0,
            "P7: die in loop body should not affect continue block with {} stmts: expected 0 PL406, got {}",
            num_continue_stmts, actual_count
        );
    }
}

// ---------------------------------------------------------------------------
// Property 8: Multiple exit points — each independent scope is analyzed
// separately
// ---------------------------------------------------------------------------

#[test]
fn property_multiple_exits_cumulative() {
    // sub { return; my $x; return; my $y; }
    // First return makes $x unreachable AND the second return unreachable,
    // then $y is also unreachable. Expected: 3 PL406 diagnostics.
    //
    // Key insight: once found_exit=true, EVERY subsequent statement (including
    // subsequent exit statements) is flagged as unreachable, not just the
    // non-exit statements that follow.
    let stmts =
        vec![return_node(), my_var_decl(25, 35, "x"), return_node(), my_var_decl(40, 50, "y")];
    let ast = program(vec![sub_node("foo", block(stmts))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);
    let actual_count = count_pl406(&diagnostics);

    // 3 diagnostics: $x, second return, and $y are all unreachable
    assert_eq!(
        actual_count, 3,
        "P8: return; $x; return; $y => expected 3 PL406, got {}",
        actual_count
    );
}

// ---------------------------------------------------------------------------
// Property 9: All four loop types (while, for, foreach) with continue blocks
// behave identically for unreachable code detection
// ---------------------------------------------------------------------------

#[test]
fn property_all_loop_types_continue_block_same_behavior() {
    // Test while, for, foreach with die in continue block followed by 1 statement
    let while_continue = block(vec![die_call(20, 35), my_var_decl(40, 50, "x")]);
    let while_loop = while_loop_with_continue(block(vec![]), while_continue);

    let for_continue = block(vec![die_call(20, 35), my_var_decl(40, 50, "x")]);
    let for_loop = for_loop_with_continue(block(vec![]), for_continue);

    let foreach_continue = block(vec![die_call(20, 35), my_var_decl(40, 50, "x")]);
    let foreach_loop = foreach_loop_with_continue(block(vec![]), foreach_continue);

    let ast_while = program(vec![while_loop]);
    let ast_for = program(vec![for_loop]);
    let ast_foreach = program(vec![foreach_loop]);

    let mut diag_while = vec![];
    let mut diag_for = vec![];
    let mut diag_foreach = vec![];
    check_unreachable_code(&ast_while, &mut diag_while);
    check_unreachable_code(&ast_for, &mut diag_for);
    check_unreachable_code(&ast_foreach, &mut diag_foreach);

    let cnt_while = count_pl406(&diag_while);
    let cnt_for = count_pl406(&diag_for);
    let cnt_foreach = count_pl406(&diag_foreach);

    assert_eq!(cnt_while, cnt_for, "P9: while and for should produce same count");
    assert_eq!(cnt_while, cnt_foreach, "P9: while and foreach should produce same count");
    assert_eq!(cnt_while, 1, "P9: all should produce 1 PL406");
}

// ---------------------------------------------------------------------------
// Property 10: Diagnostic tag is always Unnecessary for PL406
// ---------------------------------------------------------------------------

#[test]
fn property_pl406_always_has_unnecessary_tag() {
    let stmts = vec![return_node(), my_var_decl(25, 35, "x"), my_var_decl(40, 50, "y")];
    let ast = program(vec![sub_node("foo", block(stmts))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    for diag in &diagnostics {
        if diag.code.as_deref() == Some("PL406") {
            assert!(
                diag.tags.contains(&DiagnosticTag::Unnecessary),
                "P10: PL406 should always have Unnecessary tag"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 11: PL406 severity is always Hint
// ---------------------------------------------------------------------------

#[test]
fn property_pl406_always_has_hint_severity() {
    let stmts = vec![return_node(), my_var_decl(25, 35, "x")];
    let ast = program(vec![sub_node("foo", block(stmts))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    for diag in &diagnostics {
        if diag.code.as_deref() == Some("PL406") {
            assert!(
                matches!(diag.severity, perl_diagnostics::codes::DiagnosticSeverity::Hint),
                "P11: PL406 should always have Hint severity"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 12: PL406 always has a suggestion
// ---------------------------------------------------------------------------

#[test]
fn property_pl406_always_has_suggestion() {
    let stmts = vec![return_node(), my_var_decl(25, 35, "x")];
    let ast = program(vec![sub_node("foo", block(stmts))]);

    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);

    for diag in &diagnostics {
        if diag.code.as_deref() == Some("PL406") {
            assert!(diag.suggestion.is_some(), "P12: PL406 should always have a suggestion");
        }
    }
}
