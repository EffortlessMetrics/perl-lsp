use proptest::prelude::*;

use super::qw::identifier;

fn scalar_name() -> impl Strategy<Value = String> {
    identifier()
}

fn array_name() -> impl Strategy<Value = String> {
    identifier()
}

fn int_literal() -> impl Strategy<Value = i64> {
    0i64..1000
}

fn nonzero_literal() -> impl Strategy<Value = i64> {
    1i64..1000
}

fn small_literal() -> impl Strategy<Value = i64> {
    0i64..64
}

fn arithmetic_statement() -> impl Strategy<Value = String> {
    (
        scalar_name(),
        int_literal(),
        nonzero_literal(),
        prop::sample::select(vec!["+", "-", "*", "/"]),
    )
        .prop_map(|(name, lhs, rhs, op)| format!("my ${} = {} {} {};\n", name, lhs, op, rhs))
}

fn ternary_statement() -> impl Strategy<Value = String> {
    (scalar_name(), int_literal(), int_literal()).prop_map(|(name, lhs, rhs)| {
        format!(
            "my ${} = {} > {} ? {} : {};\n",
            name, lhs, rhs, lhs, rhs
        )
    })
}

fn defined_or_statement() -> impl Strategy<Value = String> {
    (
        scalar_name(),
        prop_oneof![
            Just("undef".to_string()),
            Just("0".to_string()),
            Just("\"\"".to_string()),
        ],
        prop_oneof![
            int_literal().prop_map(|value| value.to_string()),
            Just("\"fallback\"".to_string()),
        ],
    )
        .prop_map(|(name, lhs, rhs)| format!("my ${} = {} // {};\n", name, lhs, rhs))
}

fn range_statement() -> impl Strategy<Value = String> {
    (array_name(), small_literal(), small_literal()).prop_map(|(name, a, b)| {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        format!("my @{} = ({}..{});\n", name, start, end)
    })
}

fn bitwise_statement() -> impl Strategy<Value = String> {
    (scalar_name(), small_literal(), 0i64..8i64, small_literal()).prop_map(
        |(name, value, shift, mask)| {
            format!(
                "my ${} = ({} << {}) | {};\n",
                name, value, shift, mask
            )
        },
    )
}

fn concat_statement() -> impl Strategy<Value = String> {
    (
        scalar_name(),
        prop::sample::select(vec!["\"foo\"", "\"bar\"", "\"baz\"", "\"qux\""]),
        prop::sample::select(vec!["\"alpha\"", "\"beta\"", "\"gamma\"", "\"delta\""]),
    )
        .prop_map(|(name, left, right)| format!("my ${} = {} . {};\n", name, left, right))
}

fn logical_statement() -> impl Strategy<Value = String> {
    (
        scalar_name(),
        prop::sample::select(vec!["0", "1"]),
        prop::sample::select(vec!["0", "1"]),
        prop::sample::select(vec!["&&", "||"]),
    )
        .prop_map(|(name, lhs, rhs, op)| format!("my ${} = {} {} {};\n", name, lhs, op, rhs))
}

/// Generate expression-focused statements for operator coverage.
pub fn expression_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        arithmetic_statement(),
        ternary_statement(),
        defined_or_statement(),
        range_statement(),
        bitwise_statement(),
        concat_statement(),
        logical_statement(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn expressions_are_assignments(code in expression_in_context()) {
            assert!(code.contains("my $"));
            assert!(code.contains(';'));
        }
    }
}
