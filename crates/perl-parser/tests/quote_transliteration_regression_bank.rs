//! Parser-accuracy ratchet coverage for transliteration quote-like parsing.

use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn quote_transliteration_regression_bank_paired_and_unpaired() {
    let cases = [
        ("tr /a-z/A-Z/", ("a-z", "A-Z", "")),
        ("tr{abc}{xyz}cdsr", ("abc", "xyz", "cdsr")),
        ("tr[abc]{xyz}r", ("abc", "xyz", "r")),
        ("y<ab><xy>d", ("ab", "xy", "d")),
    ];

    for (input, expected) in cases {
        let (search, replacement, modifiers) = extract_transliteration_parts(input);
        assert_eq!(
            (search.as_str(), replacement.as_str(), modifiers.as_str()),
            expected,
            "failed parsing `{input}`"
        );
    }
}

#[test]
fn quote_transliteration_regression_bank_invalid_and_malformed_inputs() {
    let cases = [
        ("tr a/b/c/", ("", "", "")),
        ("tr\\a\\b\\", ("", "", "")),
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr(abc)(xyz", ("abc", "xyz", "")),
    ];

    for (input, expected) in cases {
        let (search, replacement, modifiers) = extract_transliteration_parts(input);
        assert_eq!(
            (search.as_str(), replacement.as_str(), modifiers.as_str()),
            expected,
            "failed parsing malformed `{input}`"
        );
    }
}
