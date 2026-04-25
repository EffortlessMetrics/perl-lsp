//! Minimal reproduction cases for transliteration parsing issues discovered in fuzzing.
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
fn fuzz_transliteration_regression_suite() {
    let test_cases = [
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")),
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr{abc}/xyz/s", ("abc", "xyz", "s")),
        ("tr a/b/", ("", "", "")),
        ("tr   /ab/cd/", ("ab", "cd", "")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "transliteration parse mismatch for `{input}`");
    }
}
