/// Minimal reproduction case for transliteration parsing bug discovered in fuzz testing
///
/// CRASH DETAILS:
/// - Input: "tr/abc/xyz/"
/// - Expected: ("abc", "xyz", "")
/// - Actual: ("abc", "", "xyz")
///
/// This indicates a critical bug in extract_transliteration_parts where the replacement
/// and modifiers are being swapped for non-paired delimiters.
///
/// IMPACT: This affects Perl transliteration operator parsing throughout the entire
/// parser pipeline, potentially causing incorrect syntax highlighting, code analysis,
/// and refactoring operations.
///
/// REPRODUCTION: Run with `cargo test -p perl-parser --test fuzz_transliteration_crash_repro`
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
        // tr/// and y///
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")), // invalid transliteration modifier filtered out
        // paired delimiters
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc]{xyz}sr", ("abc", "xyz", "sr")),
        // delimiter edge cases
        (r"tr|a\|b|x\|y|d", (r"a\|b", r"x\|y", "d")),
        ("tr<ab<cd>><xy>r", ("ab<cd>", "xy", "r")),
        // malformed-but-nonpanicking
        ("tr/abc", ("abc", "", "")),
        ("tr{abc}xyz", ("abc", "", "")),
        ("tr abc xyz", ("", "", "")),
        ("tr", ("", "", "")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed transliteration parse for `{input}`");
    }
}
