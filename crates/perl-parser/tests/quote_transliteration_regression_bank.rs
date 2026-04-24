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
