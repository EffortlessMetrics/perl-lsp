use perl_parser::quote_parser::extract_transliteration_parts;

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
        ("tr   /abc/xyz/", ("abc", "xyz", ""), "optional whitespace after operator"),
        ("tr", ("", "", ""), "missing delimiter"),
        ("trabc/xyz/", ("", "", ""), "invalid alphanumeric delimiter rejected"),
        ("tr/abc", ("abc", "", ""), "malformed missing replacement closure does not panic"),
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
