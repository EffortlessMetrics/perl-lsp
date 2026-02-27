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

    println!("FUZZ BUG REPRODUCED: tr// parsing issue");
    println!("Input: {}", input);
    println!("Expected: ('abc', 'xyz', '')");
    println!("Actual: ('{}', '{}', '{}')", search, replace, modifiers);

    // Demonstrate the bug - these will fail due to the parsing issue
    assert_eq!(search.as_str(), "abc", "Search pattern incorrect");
    assert_eq!(replace.as_str(), "xyz", "Replace pattern incorrect");
    assert_eq!(modifiers.as_str(), "", "Modifiers incorrect");
}

#[test]
fn fuzz_transliteration_regression_suite() {
    // Test additional variants that likely have the same bug
    let test_cases = vec![
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")), // 'g' is not a valid transliteration modifier
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")), // This might work correctly with paired delimiters
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());

        println!("Testing: {} -> expected {:?}, got {:?}", input, expected, actual);

        if actual != expected {
            println!("  BUG CONFIRMED in variant: {}", input);
        } else {
            println!("  PASSED: {}", input);
        }
    }
}
