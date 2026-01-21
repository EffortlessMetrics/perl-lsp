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
        format!("my ${} = {} > {} ? {} : {};\n", name, lhs, rhs, lhs, rhs)
    })
}

fn defined_or_statement() -> impl Strategy<Value = String> {
    (
        scalar_name(),
        prop_oneof![Just("undef".to_string()), Just("0".to_string()), Just("\"\"".to_string()),],
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
            format!("my ${} = ({} << {}) | {};\n", name, value, shift, mask)
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

fn repeat_statement() -> impl Strategy<Value = String> {
    (scalar_name(), prop::sample::select(vec!["\"-\"", "\"*\"", "\".\""]), small_literal())
        .prop_map(|(name, token, count)| format!("my ${} = {} x {};\n", name, token, count))
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

fn compound_assignment_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        (
            scalar_name(),
            int_literal(),
            prop::sample::select(vec!["+=", "-=", "*=", ".=", "||=", "//="]),
        )
            .prop_map(|(name, value, op)| {
                format!("my ${} = {};\n${} {} {};\n", name, value, name, op, value)
            }),
        (scalar_name(), nonzero_literal()).prop_map(|(name, value)| {
            format!("my ${} = {};\n${} /= {};\n", name, value, name, value)
        }),
    ]
}

fn binding_statement() -> impl Strategy<Value = String> {
    (scalar_name(), prop::sample::select(vec!["alpha", "beta", "gamma"])).prop_map(
        |(name, token)| {
            format!("my ${} = \"{}\";\nmy $ok = ${} =~ /{}/;\n", name, token, name, token)
        },
    )
}

fn smartmatch_statement() -> impl Strategy<Value = String> {
    (array_name(), scalar_name()).prop_map(|(array, scalar)| {
        format!(
            "my @{} = qw(admin user);\nmy ${} = \"admin\";\nmy $has = ${} ~~ @{};\n",
            array, scalar, scalar, array
        )
    })
}

fn isa_statement() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["Thing", "Widget", "Demo"]).prop_map(|class| {
        format!(
            "my $object = bless {{ id => 1 }}, \"{}\";\nif ($object isa {}) {{\n    print \"ok\";\n}}\n",
            class, class
        )
    })
}

fn exists_statement() -> impl Strategy<Value = String> {
    (scalar_name(), prop::sample::select(vec!["HOME", "PATH", "SHELL"]))
        .prop_map(|(name, key)| format!("my ${} = exists $ENV{{{}}} ? 1 : 0;\n", name, key))
}

fn defined_statement() -> impl Strategy<Value = String> {
    scalar_name().prop_map(|name| format!("my ${} = defined $ARGV[0] ? $ARGV[0] : \"\";\n", name))
}

fn flipflop_statement() -> impl Strategy<Value = String> {
    (scalar_name(), small_literal(), small_literal()).prop_map(|(name, a, b)| {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        format!(
            "my ${} = 0;\nfor my $i (1..10) {{\n    ${} = 1 if $i == {} .. $i == {};\n}}\n",
            name, name, start, end
        )
    })
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
        repeat_statement(),
        logical_statement(),
        compound_assignment_statement(),
        binding_statement(),
        smartmatch_statement(),
        isa_statement(),
        exists_statement(),
        defined_statement(),
        flipflop_statement(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn expressions_are_assignments(code in expression_in_context()) {
            assert!(code.contains("my $") || code.contains("my @") || code.contains("my %"));
            assert!(code.contains(';'));
        }
    }
}
