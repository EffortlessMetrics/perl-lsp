use proptest::prelude::*;

pub(super) fn ascii_letter_or_underscore() -> impl Strategy<Value = char> {
    prop_oneof![prop::char::range('a', 'z'), prop::char::range('A', 'Z'), Just('_')]
}

pub(super) fn ascii_alphanumeric_or_underscore() -> impl Strategy<Value = char> {
    prop_oneof![
        prop::char::range('a', 'z'),
        prop::char::range('A', 'Z'),
        prop::char::range('0', '9'),
        Just('_'),
    ]
}
