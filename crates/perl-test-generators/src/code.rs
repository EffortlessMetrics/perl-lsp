//! Generators for syntactically conservative Perl snippets.
//!
//! These strategies intentionally produce a small, valid subset of Perl so
//! parser property tests can spend cases on AST invariants instead of filtering
//! out invalid random text.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;

fn ascii_letter_or_underscore() -> impl Strategy<Value = char> {
    prop_oneof![prop::char::range('a', 'z'), prop::char::range('A', 'Z'), Just('_')]
}

fn ascii_alphanumeric_or_underscore() -> impl Strategy<Value = char> {
    prop_oneof![
        prop::char::range('a', 'z'),
        prop::char::range('A', 'Z'),
        prop::char::range('0', '9'),
        Just('_'),
    ]
}

/// Generate a plain ASCII Perl identifier without a sigil.
pub fn perl_identifier() -> impl Strategy<Value = String> {
    (
        ascii_letter_or_underscore(),
        prop::collection::vec(ascii_alphanumeric_or_underscore(), 0..=10_usize),
    )
        .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

/// Generate a scalar variable name such as `$value`.
pub fn scalar_variable() -> impl Strategy<Value = String> {
    perl_identifier().prop_map(|name| format!("${name}"))
}

/// Generate a non-negative integer literal.
pub fn integer_literal() -> impl Strategy<Value = String> {
    (0_u32..=9999).prop_map(|value| value.to_string())
}

/// Generate a simple single-quoted string literal.
pub fn single_quoted_string_literal() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![ascii_alphanumeric_or_underscore(), Just(' '), Just('-')],
        0..=16_usize,
    )
    .prop_map(|chars| {
        let body: String = chars.into_iter().collect();
        format!("'{body}'")
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn identifiers_are_not_empty(identifier in perl_identifier()) {
            prop_assert!(!identifier.is_empty(), "identifier must not be empty");
        }

        #[test]
        fn scalar_variables_start_with_dollar(variable in scalar_variable()) {
            prop_assert!(variable.starts_with('$'), "scalar variable must start with '$': {variable}");
            prop_assert!(variable.len() > 1, "scalar variable must include a name: {variable}");
        }

        #[test]
        fn single_quoted_literals_are_balanced(literal in single_quoted_string_literal()) {
            prop_assert!(literal.starts_with('\''), "literal must start with quote: {literal}");
            prop_assert!(literal.ends_with('\''), "literal must end with quote: {literal}");
        }

        #[test]
        fn simple_programs_do_not_contain_nul(program in simple_program()) {
            prop_assert!(!program.contains('\0'), "generated program contained NUL");
        }
    }
}
