use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/");

    assert_eq!(search, "abc");
    assert_eq!(replace, "xyz");
    assert_eq!(modifiers, "");
}

#[test]
fn fuzz_transliteration_regression_suite() {
    let test_cases = [
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")),
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr /a\\//x\\//", ("a\\/", "x\\/", "")),
        ("tr/🦀/🐪/r", ("🦀", "🐪", "r")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_eq!((search.as_str(), replace.as_str(), modifiers.as_str()), expected, "{input}");
    }
}

#[test]
fn malformed_transliteration_never_panics() {
    // Each entry: (input, expected_search, expected_replace, expected_modifiers)
    // Backslash is an invalid delimiter, so the y\\ inputs return all-empty.
    let cases: &[(&str, &str, &str, &str)] = &[
        ("tr", "", "", ""),
        ("trabc", "", "", ""),          // alphanumeric delimiter rejected
        ("tr/unterminated", "unterminated", "", ""),  // missing replacement closure
        ("tr{abc}{xyz", "abc", "xyz", ""),  // unclosed replacement
        ("tr{abc}xyz}", "abc", "", ""),     // replacement doesn't start with paired open
        ("y\\abc\\xyz\\", "", "", ""),      // backslash is invalid delimiter
    ];

    for (input, exp_search, exp_replace, exp_mods) in cases {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "extract_transliteration_parts panicked for {input}");
        let (search, replace, mods) = result.unwrap();
        assert_eq!(
            (search.as_str(), replace.as_str(), mods.as_str()),
            (*exp_search, *exp_replace, *exp_mods),
            "value mismatch for malformed input {input}"
        );
    }
}
