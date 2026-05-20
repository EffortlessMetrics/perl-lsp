use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

use super::expressions::simple_expression;
use super::identifiers::scalar_variable;

fn simple_statement_with_depth(depth: u32) -> BoxedStrategy<String> {
    let var = scalar_variable().boxed();
    let expr = simple_expression();
    let base = prop_oneof![
        var.clone().prop_map(|name| format!("my {name};")),
        (var.clone(), expr.clone()).prop_map(|(name, value)| format!("my {name} = {value};")),
        (var.clone(), expr.clone()).prop_map(|(name, value)| format!("{name} = {value};")),
        expr.clone().prop_map(|value| format!("print {value};")),
    ];

    if depth == 0 {
        return base.boxed();
    }

    prop_oneof![
        base,
        (expr, simple_statement_with_depth(depth - 1))
            .prop_map(|(condition, body)| format!("if ({condition}) {{ {body} }}")),
    ]
    .boxed()
}

/// Generate a simple statement that should be valid Perl syntax.
pub fn simple_statement() -> BoxedStrategy<String> {
    simple_statement_with_depth(2)
}

/// Generate a small Perl program made from conservative statement forms.
pub fn simple_program() -> impl Strategy<Value = String> {
    prop::collection::vec(simple_statement(), 0..=8_usize)
        .prop_map(|statements| statements.join("\n"))
}
