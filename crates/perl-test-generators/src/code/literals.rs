use proptest::prelude::*;

/// Generate a non-negative integer literal.
pub fn integer_literal() -> impl Strategy<Value = String> {
    (0_u32..=9999).prop_map(|value| value.to_string())
}

/// Generate a simple single-quoted string literal.
pub fn single_quoted_string_literal() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            prop::char::range('0', '9'),
            Just(' '),
            Just('_'),
            Just('-'),
        ],
        0..=16_usize,
    )
    .prop_map(|chars| {
        let body: String = chars.into_iter().collect();
        format!("'{body}'")
    })
}
