/// Regression bank for fuzz-discovered transliteration extraction failures.
use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let input = "tr/abc/xyz/";
    let (search, replace, modifiers) = extract_transliteration_parts(input);

    assert_eq!(search.as_str(), "abc", "Search pattern incorrect");
    assert_eq!(replace.as_str(), "xyz", "Replace pattern incorrect");
    assert_eq!(modifiers.as_str(), "", "Modifiers incorrect");
}

#[test]
fn transliteration_non_paired_delimiters_regression_bank() {
    let test_cases = [
        ("tr/abc/xyz/", ("abc", "xyz", "")),
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")),
        ("tr!a\\!!b\\!!cdr", ("a\\!", "b\\!", "cdr")),
        ("tr /a/b/d", ("a", "b", "d")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "input: {input}");
    }
}

#[test]
fn transliteration_paired_delimiters_regression_bank() {
    let test_cases = [
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc][xyz]cdr", ("abc", "xyz", "cdr")),
        ("tr<abc>{xyz}s", ("abc", "xyz", "s")),
        ("y(a(b)c)(x(y)z)r", ("a(b)c", "x(y)z", "r")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "input: {input}");
    }
}

#[test]
fn transliteration_malformed_is_non_panicking_and_does_not_leak_modifiers() {
    let malformed_cases = [
        ("tr", ("", "", "")),
        ("trabc", ("", "", "")),
        ("tr/abc", ("abc", "", "")),
        ("tr/abc/xyz", ("abc", "xyz", "")),
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr{abc}xyzcdr", ("abc", "", "")),
    ];

    for (input, expected) in malformed_cases {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "input panicked: {input}");
        let (search, replace, modifiers) = match result {
            Ok(parts) => parts,
            Err(_) => unreachable!("asserted is_ok above"),
        };
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "input: {input}");
    }
}
