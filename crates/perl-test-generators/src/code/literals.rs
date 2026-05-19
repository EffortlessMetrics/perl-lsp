use proptest::prelude::*;

use super::chars::ascii_alphanumeric_or_underscore;

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
