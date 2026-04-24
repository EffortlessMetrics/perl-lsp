//! Regression bank for fuzz-discovered transliteration parsing failures.

use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let input = "tr/abc/xyz/";
    let (search, replace, modifiers) = extract_transliteration_parts(input);

    assert_eq!(search, "abc", "search should be extracted from the first body");
    assert_eq!(replace, "xyz", "replacement should be extracted from the second body");
    assert_eq!(modifiers, "", "no modifiers are present in the minimal reproducer");
}

#[test]
fn transliteration_regression_bank_non_paired_tr_and_y() {
    let cases = [
        ("tr/a-z/A-Z/", ("a-z", "A-Z", "")),
        ("y/a-z/A-Z/", ("a-z", "A-Z", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")), // `g` is invalid for transliteration and is filtered
        ("tr /left/right/c", ("left", "right", "c")), // optional whitespace after op
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed non-paired transliteration case: {input}");
    }
}

#[test]
fn transliteration_regression_bank_paired_and_delimiter_edges() {
    let cases = [
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("y(abc)(xyz)r", ("abc", "xyz", "r")),
        ("tr[abc]{xyz}", ("abc", "xyz", "")), // different paired delimiter for replacement
        ("tr<ab\\>c><xy\\>z>cd", ("ab\\>c", "xy\\>z", "cd")),
        ("tr##", ("", "", "")),
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed paired/delimiter transliteration case: {input}");
    }
}

#[test]
fn transliteration_regression_bank_malformed_nonpanicking_cases() {
    let malformed_cases = [
        "tr",
        "trabc",
        "tr{abc}incomplete",
        "tr/abc",
        "tr/abc/def",
        "y{a}{b", // missing closing delimiter for replacement
        "y /a/b", // missing closing delimiter for replacement with whitespace after op
    ];

    for input in malformed_cases {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "parser panicked for malformed case: {input}");
    }

    assert_eq!(
        extract_transliteration_parts("tr{abc}incomplete"),
        ("abc".to_string(), String::new(), String::new()),
        "missing replacement delimiter should preserve parsed search and clear replacement/modifiers"
    );
    assert_eq!(
        extract_transliteration_parts("tr/abc/def"),
        ("abc".to_string(), "def".to_string(), String::new()),
        "unterminated non-paired replacement should not treat replacement bytes as modifiers"
    );
}
