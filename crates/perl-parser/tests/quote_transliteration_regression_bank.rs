use perl_parser::quote_parser::{
    TransliterationError, extract_transliteration_parts, extract_transliteration_parts_strict,
};

#[test]
fn transliteration_regression_bank_valid_cases() {
    let cases = [
        ("tr/a\\/b/c\\/d/", ("a\\/b", "c\\/d", "")),
        ("tr/🦀/🐪/", ("🦀", "🐪", "")),
        ("tr///", ("", "", "")),
        ("tr{abc}/xyz/cdsr", ("abc", "xyz", "cdsr")),
        ("y  {αβ}{γδ}r", ("αβ", "γδ", "r")),
        // '#' is a valid non-paired, non-alphanumeric delimiter
        ("tr#abc#xyz#", ("abc", "xyz", "")),
        // nested brackets in search list (Perl allows depth-tracking)
        ("tr{a{b}c}{xyz}", ("a{b}c", "xyz", "")),
        // delete mode: no replacement characters
        ("tr/abc//d", ("abc", "", "d")),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!(
            (actual.0.as_str(), actual.1.as_str(), actual.2.as_str()),
            expected,
            "unexpected parse for {input:?}"
        );
    }
}

#[test]
fn transliteration_regression_bank_strict_errors() {
    let cases = [
        ("tr/a\\/b/c\\/d/z", TransliterationError::InvalidModifier('z')),
        ("tr a/b/", TransliterationError::InvalidDelimiter('a')),
        ("tr/abc/xyz", TransliterationError::MissingClosingDelimiter),
        ("tr{abc}{xyz", TransliterationError::MissingClosingDelimiter),
        ("tr{abc}xyz", TransliterationError::InvalidDelimiter('x')),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts_strict(input);
        assert_eq!(actual, Err(expected), "expected strict error for {input:?}");
    }
}

#[test]
fn transliteration_regression_bank_non_strict_cases() {
    let cases = [
        ("tr/a\\/b/c\\/d/", ("a\\/b", "c\\/d", ""), "escaped delimiter in search and replacement"),
        ("tr/αβγ/ΑΒΓ/", ("αβγ", "ΑΒΓ", ""), "unicode multibyte bodies"),
        ("tr///", ("", "", ""), "empty search and replacement"),
        ("tr/a/b/cdsr", ("a", "b", "cdsr"), "all supported modifiers"),
        ("tr/a/b/z", ("a", "b", ""), "invalid modifiers are ignored by non-strict parser"),
        ("tr{abc}{xyz}d", ("abc", "xyz", "d"), "paired delimiters"),
        ("tr[abc]{xyz}r", ("abc", "xyz", "r"), "mixed paired delimiters"),
        ("tr   /abc/xyz/", ("abc", "xyz", ""), "optional whitespace after tr operator"),
        ("y   /abc/xyz/", ("abc", "xyz", ""), "optional whitespace after y operator"),
        ("tr", ("", "", ""), "missing delimiter"),
        ("trabc/xyz/", ("", "", ""), "invalid alphanumeric delimiter rejected after tr"),
        ("yabc/xyz/", ("", "", ""), "invalid alphanumeric delimiter rejected after y"),
        ("tr/abc", ("abc", "", ""), "malformed missing replacement closure does not panic"),
        // Spaces between modifiers: take_while(is_ascii_alphabetic) stops at the space,
        // so only 'c' is collected. Perl does not allow spaces inside modifier strings.
        ("tr/a/b/c d s r", ("a", "b", "c"), "spaces in modifier string truncates at first space"),
    ];

    for (input, expected, label) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!(
            actual,
            (expected.0.to_string(), expected.1.to_string(), expected.2.to_string()),
            "{label}: `{input}`"
        );
    }
}
