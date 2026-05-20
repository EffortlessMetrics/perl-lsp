use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

use super::identifiers::scalar_variable;
use super::literals::{integer_literal, single_quoted_string_literal};

fn expression_leaf() -> BoxedStrategy<String> {
    prop_oneof![scalar_variable(), integer_literal(), single_quoted_string_literal()].boxed()
}

/// Generate a small expression that should be valid Perl syntax.
pub fn simple_expression() -> BoxedStrategy<String> {
    expression_leaf()
        .prop_recursive(3, 24, 3, |inner| {
            prop_oneof![
                (inner.clone(), prop_oneof![Just('+'), Just('-'), Just('*')], inner.clone())
                    .prop_map(|(left, op, right)| format!("({left} {op} {right})")),
                (inner.clone(), inner).prop_map(|(left, right)| format!("({left} . {right})")),
            ]
        })
        .boxed()
}
