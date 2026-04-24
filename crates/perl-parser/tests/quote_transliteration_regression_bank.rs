//! Focused transliteration regression bank used by parser accuracy ratchets.

use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn transliteration_regression_bank_cases() {
    let cases = [
        // Escaped delimiter in search body.
        ("tr/a\\/b/c/", ("a\\/b", "c", "")),
        // Unicode and multibyte content.
        ("tr/🦀α/🐪β/", ("🦀α", "🐪β", "")),
        // Empty search and replacement bodies (no panic; stable extraction).
        ("tr///", ("", "", "")),
        ("tr//x/", ("", "x", "")),
        ("tr/x//", ("x", "", "")),
        // Malformed closures should not leak replacement/modifiers.
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr{abc}xyz", ("abc", "", "")),
        // Valid transliteration modifiers.
        ("tr/a/b/c", ("a", "b", "c")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("tr/a/b/s", ("a", "b", "s")),
        ("tr/a/b/r", ("a", "b", "r")),
        ("tr/a/b/cdsr", ("a", "b", "cdsr")),
        // Invalid modifiers are ignored by non-strict extraction.
        ("tr/a/b/z", ("a", "b", "")),
    ];

    for (input, expected) in cases {
        let (search, replacement, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replacement.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "regression case failed for `{input}`");
    }
}
