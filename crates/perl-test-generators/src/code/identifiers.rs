use proptest::prelude::*;

/// Generate a plain ASCII Perl identifier without a sigil.
pub fn perl_identifier() -> impl Strategy<Value = String> {
    (
        prop_oneof![prop::char::range('a', 'z'), prop::char::range('A', 'Z'), Just('_')],
        prop::collection::vec(
            prop_oneof![
                prop::char::range('a', 'z'),
                prop::char::range('A', 'Z'),
                prop::char::range('0', '9'),
                Just('_'),
            ],
            0..=10_usize,
        ),
    )
        .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

/// Generate a scalar variable name such as `$value`.
pub fn scalar_variable() -> impl Strategy<Value = String> {
    perl_identifier().prop_map(|name| format!("${name}"))
}
