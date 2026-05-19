use proptest::prelude::*;

use super::chars::{ascii_alphanumeric_or_underscore, ascii_letter_or_underscore};

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
