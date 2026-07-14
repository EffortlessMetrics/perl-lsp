//! Mutation-proof boundary tests for `try` call disambiguation.
//!
//! A statement-start `try` is a language construct only when it is followed by
//! a block. Mutating that discriminator must not turn a parenthesized user
//! call into a `Try` AST node.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::{must, must_some};

#[test]
fn parenthesized_try_is_a_user_defined_function_call() {
    let source = "sub try { 1 }; try({ key => 1 }, 'argument');";
    assert_clean_parse(source);

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let statements = must_some(match ast.kind {
        NodeKind::Program { statements } => Some(statements),
        _ => None,
    });
    let call_statement = must_some(statements.get(1));

    assert!(
        matches!(
            call_statement.kind,
            NodeKind::ExpressionStatement { ref expression }
                if matches!(
                    expression.kind,
                    NodeKind::FunctionCall { ref name, ref args }
                        if name == "try" && args.len() == 2
                )
        ),
        "parenthesized try must remain a two-argument function call"
    );
}

#[test]
fn braced_try_remains_a_try_catch_construct() {
    let source = "try { risky() } catch ($error) { recover($error) };";
    assert_clean_parse(source);

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let statements = must_some(match ast.kind {
        NodeKind::Program { statements } => Some(statements),
        _ => None,
    });
    let try_statement = must_some(statements.first());

    assert!(matches!(try_statement.kind, NodeKind::Try { .. }));
}
