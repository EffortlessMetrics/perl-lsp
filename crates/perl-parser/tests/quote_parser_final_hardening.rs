/// Final hardening tests targeting the last 13 mutation survivors
///
/// Based on partial mutation testing results, these survivors remain:
/// - FnValue mutations returning "xyzzy" combinations
/// - Arithmetic: + → *, >= → ==, += → -=
/// - Match arm deletions in get_closing_delimiter
/// - Boolean logic: && → ||, != → ==
///
/// Target: Kill remaining survivors to reach 85%+ mutation score
///
/// Labels: tests:hardening
use perl_parser::quote_parser::*;

/// Ultra-specific tests for remaining function return mutations
mod final_function_return_kills {
    use super::*;

    /// Kill mutation: extract_regex_parts -> (String::new(), "xyzzy".into())
    #[test]
    fn test_kill_regex_parts_string_new_xyzzy_mutation() {
        // Test cases that must NOT return (String::new(), "xyzzy".into())
        let cases = vec![
            ("qr/test/", ("/test/", "")), // Should return pattern, not String::new()
            ("m/pattern/i", ("/pattern/", "i")), // Should return pattern and modifier, not String::new() and "xyzzy"
        ];

        for (input, (expected_pattern, expected_mods)) in cases {
            let (actual_pattern, _body, actual_mods) = extract_regex_parts(input);

            // Explicit assertions to kill the mutation
            assert_ne!(actual_pattern, "", "Pattern should not be String::new() for '{}'", input);
            assert_ne!(actual_mods, "xyzzy", "Modifiers should not be 'xyzzy' for '{}'", input);

            assert_eq!(actual_pattern, expected_pattern, "Pattern mismatch for '{}'", input);
            assert_eq!(actual_mods, expected_mods, "Modifiers mismatch for '{}'", input);
        }
    }

    /// Kill mutation: extract_regex_parts -> ("xyzzy".into(), String::new())
    #[test]
        fn test_kill_regex_parts_xyzzy_string_new_mutation() {
        let cases = vec![
            ("qr/test/i", ("/test/", "i")), // Should return pattern and modifier, not "xyzzy" and String::new()
            ("m/pattern/", ("/pattern/", "")), // Pattern should not be "xyzzy", no modifiers expected
        ];

        for (input, (expected_pattern, expected_mods)) in cases {
            let (actual_pattern, _body, actual_mods) = extract_regex_parts(input);

            // Kill the specific mutation - pattern should never be "xyzzy"
            assert_ne!(actual_pattern, "xyzzy", "Pattern should not be 'xyzzy' for '{}'", input);
            // Only check that mods aren't empty when we expect non-empty mods
            if !expected_mods.is_empty() {
                assert_ne!(
                    actual_mods, "",
                    "Modifiers should not be String::new() when there are modifiers for '{}'",
                    input
                );
            }

            assert_eq!(actual_pattern, expected_pattern, "Pattern mismatch for '{}'", input);
            assert_eq!(actual_mods, expected_mods, "Modifiers mismatch for '{}'", input);
        }
    }

    /// Kill mutations in extract_transliteration_parts
    #[test]
        fn test_kill_transliteration_xyzzy_mutations() {
        // Kill: ("xyzzy".into(), "xyzzy".into(), String::new())
        // Kill: (String::new(), "xyzzy".into(), String::new())
        // Kill: (String::new(), String::new(), "xyzzy".into())

        let cases = vec![
            ("tr{abc}{xyz}d", ("abc", "xyz", "d")), // Paired delimiters with modifiers
            ("y/old/new/", ("old", "new", "")),     // Symmetric delimiters, no modifiers
            ("tr/a/b/c", ("a", "b", "c")), // Symmetric delimiters with modifiers after 3rd delimiter
        ];

        for (input, (expected_search, expected_replace, expected_mods)) in cases {
            let (actual_search, actual_replace, actual_mods) = extract_transliteration_parts(input);

            // Kill all "xyzzy" mutations
            assert_ne!(actual_search, "xyzzy", "Search should not be 'xyzzy' for '{}'", input);
            assert_ne!(actual_replace, "xyzzy", "Replace should not be 'xyzzy' for '{}'", input);
            assert_ne!(actual_mods, "xyzzy", "Modifiers should not be 'xyzzy' for '{}'", input);

            // Kill String::new() mutations when we expect content
            if !expected_search.is_empty() {
                assert_ne!(actual_search, "", "Search should not be String::new() for '{}'", input);
            }
            if !expected_replace.is_empty() {
                assert_ne!(
                    actual_replace, "",
                    "Replace should not be String::new() for '{}'",
                    input
                );
            }

            assert_eq!(actual_search, expected_search, "Search mismatch for '{}'", input);
            assert_eq!(actual_replace, expected_replace, "Replace mismatch for '{}'", input);
            assert_eq!(actual_mods, expected_mods, "Modifiers mismatch for '{}'", input);
        }
    }
}

/// Tests targeting specific arithmetic mutations
mod final_arithmetic_kills {
    use super::*;

    /// Kill mutation: + → * in position calculation (line 209:37)
    #[test]
    fn test_kill_position_plus_to_multiply_mutation() {
        // Test case where + vs * would give very different results
        // Position calculation: end_pos = i + ch.len_utf8() vs end_pos = i * ch.len_utf8()

        let input = "s/test/replacement/";
        let (pattern, replacement, _) = extract_substitution_parts(input);

        // With correct +: Should properly extract
        // With * mutation: Would likely give wrong position and break parsing
        assert_eq!(pattern, "test", "Pattern parsing should work with + arithmetic");
        assert_eq!(replacement, "replacement", "Replacement parsing should work with + arithmetic");

        // Additional specific test for position arithmetic
        let input2 = "s/a/b/"; // Simple case where i=1, ch.len_utf8()=1
        let (pattern2, replacement2, _) = extract_substitution_parts(input2);

        // With +: end_pos = 1 + 1 = 2 (correct)
        // With *: end_pos = 1 * 1 = 1 (wrong, would not advance)
        assert_eq!(pattern2, "a", "Simple arithmetic should work correctly");
        assert_eq!(replacement2, "b", "Simple replacement should work correctly");
    }

    /// Kill mutation: > → >= in length check (line 12:23)
    #[test]
    fn test_kill_length_greater_than_to_greater_equal_mutation() {
        // Specific boundary case where > vs >= matters
        let boundary_input = "mx"; // len() = 2
        let (pattern, _body, _modifiers) = extract_regex_parts(boundary_input);

        // With > 1: len()=2 > 1 is true, so check alphabetic
        // With >= 1: len()=2 >= 1 is true, so check alphabetic (same result)
        // Need a different approach - test with len()=1

        let single_char = "m"; // len() = 1
        let (pattern_single, _body, _) = extract_regex_parts(single_char);

        // With > 1: len()=1 > 1 is false, should use text directly -> "mm"
        // With >= 1: len()=1 >= 1 is true, would try other logic
        assert_eq!(pattern_single, "mm", "Single 'm' should use > 1 check correctly");

        // Verify the >= mutation would behave differently
        assert_eq!(pattern, "mxm", "Two char should also work correctly");
    }

    /// Kill mutation: += → -= in depth tracking (line 203:23)
    #[test]
    fn test_kill_depth_plus_equal_to_minus_equal_mutation() {
        // Test nested delimiters where += vs -= in depth tracking matters
        let nested_input = "s{a{inner}b}{replacement}";
        let (pattern, replacement, _) = extract_substitution_parts(nested_input);

        // With +=: depth increments correctly (1 -> 2 -> 1 -> 0)
        // With -=: depth would decrement (1 -> 0 immediately, breaking parsing)
        assert_eq!(pattern, "a{inner}b", "Nested parsing should work with += depth tracking");
        assert_eq!(
            replacement, "replacement",
            "Replacement should be extracted with correct depth tracking"
        );

        // Test deeper nesting to ensure proper increment
        let deep_nested = "s{{{test}}}{repl}";
        let (deep_pattern, deep_replacement, _) = extract_substitution_parts(deep_nested);
        assert_eq!(deep_pattern, "{{test}}", "Deep nesting should work with += depth tracking");
        assert_eq!(deep_replacement, "repl", "Deep replacement should work");
    }
}

/// Tests targeting boolean logic mutations
mod final_boolean_logic_kills {
    use super::*;

    /// Kill mutation: && → || in extract_regex_parts (line 12:9)
    #[test]
    fn test_kill_and_to_or_mutation_in_regex() {
        // Test the specific condition: text.starts_with('m') && text.len() > 1 && !text.chars().nth(1).unwrap().is_alphabetic()

        // Case where && vs || matters:
        // starts_with('m')=true, len()>1=true, !is_alphabetic()=true
        // && result: true && true && true = true
        // || result: true || true || true = true (same)

        // Need case where one condition is false:
        // starts_with('m')=true, len()>1=false, !is_alphabetic()=N/A
        let single_m = "m";
        let (pattern, _body, _) = extract_regex_parts(single_m);

        // With &&: true && false = false, so use text directly
        // With ||: true || false = true, so would try [1..] and panic/error
        assert_eq!(pattern, "mm", "Single 'm' should work with && logic");

        // Case: starts_with('m')=true, len()>1=true, !is_alphabetic()=false
        let m_alpha = "ma";
        let (pattern_alpha, _body, _) = extract_regex_parts(m_alpha);

        // With &&: true && true && false = false, use text directly
        // With ||: true || true || false = true, would try [1..]
        assert_eq!(pattern_alpha, "mam", "Alphabetic case should work with && logic");
    }

    /// Kill mutation: != → == in extract_transliteration_parts (line 137:31)
    #[test]
        fn test_kill_not_equal_to_equal_mutation() {
        // Test delimiter comparison logic
        let paired_input = "tr{abc}{xyz}";
        let (search, replace, _) = extract_transliteration_parts(paired_input);

        // The mutation is likely in: delimiter != closing
        // With !=: '{' != '}' is true (paired delimiters)
        // With ==: '{' == '}' is false (would treat as non-paired)
        assert_eq!(search, "abc", "Paired delimiter search should work with !=");
        assert_eq!(replace, "xyz", "Paired delimiter replace should work with !=");

        // Test non-paired (symmetric) delimiter to ensure correct behavior
        let non_paired = "tr/abc/xyz/";
        let (np_search, np_replace, np_mods) = extract_transliteration_parts(non_paired);

        // With !=: '/' != '/' is false (symmetric delimiter, not paired)
        // With ==: '/' == '/' is true (would incorrectly treat as paired)
        // For symmetric delimiters like /, the parsing correctly extracts:
        //   search = "abc", replace = "xyz", mods = ""
        assert_eq!(np_search, "abc", "Symmetric delimiter search extraction");
        assert_eq!(np_replace, "xyz", "Symmetric delimiter replace extraction");
        assert_eq!(np_mods, "", "Symmetric delimiter modifiers extraction");
    }
}

/// Tests targeting match arm deletion mutations
mod final_match_arm_kills {
    use super::*;

    /// Kill match arm deletion mutations in get_closing_delimiter
    #[test]
    fn test_kill_match_arm_deletions() {
        // Test each delimiter type to ensure all match arms are needed

        // Test '{' arm deletion (line 164:9)
        let brace_input = "s{pattern}{replacement}";
        let (pattern, replacement, _) = extract_substitution_parts(brace_input);
        assert_eq!(pattern, "pattern", "Brace delimiter should work (match arm must exist)");
        assert_eq!(replacement, "replacement", "Brace replacement should work");

        // Test '[' arm deletion (line 163:9)
        let bracket_input = "s[pattern][replacement]";
        let (bracket_pattern, bracket_replacement, _) = extract_substitution_parts(bracket_input);
        assert_eq!(
            bracket_pattern, "pattern",
            "Bracket delimiter should work (match arm must exist)"
        );
        assert_eq!(bracket_replacement, "replacement", "Bracket replacement should work");

        // Test '<' arm deletion (line 165:9)
        let angle_input = "s<pattern><replacement>";
        let (angle_pattern, angle_replacement, _) = extract_substitution_parts(angle_input);
        assert_eq!(angle_pattern, "pattern", "Angle delimiter should work (match arm must exist)");
        assert_eq!(angle_replacement, "replacement", "Angle replacement should work");

        // Test through regex parsing as well
        let regex_cases = vec![
            ("qr{test}", ("{test}", "")),
            ("qr[test]", ("[test]", "")),
            ("qr<test>", ("<test>", "")),
        ];

        for (input, (expected_pattern, expected_mods)) in regex_cases {
            let (actual_pattern, _body, actual_mods) = extract_regex_parts(input);
            assert_eq!(actual_pattern, expected_pattern, "Regex delimiter {} should work", input);
            assert_eq!(actual_mods, expected_mods, "Regex modifiers for {}", input);
        }
    }
}

/// Debug actual behavior to fix test expectations
#[test]
fn debug_actual_behavior() {
    // Debug the failing cases
    println!("=== Debugging actual quote parser behavior ===");

    // Test transliteration
    let cases = vec!["tr{abc}{xyz}d", "y/old/new/m"];

    for input in cases {
        let (search, replace, mods) = extract_transliteration_parts(input);
        println!("{} -> search:'{}', replace:'{}', mods:'{}'", input, search, replace, mods);
    }

    // Test substitution with braces
    let sub_cases = vec!["s{pattern}{replacement}", "s{old{nested}path}{new{nested}path}gi"];

    for input in sub_cases {
        let (pattern, replacement, mods) = extract_substitution_parts(input);
        println!(
            "{} -> pattern:'{}', replacement:'{}', mods:'{}'",
            input, pattern, replacement, mods
        );
    }

    // Test regex cases
    let regex_cases = vec!["qr/test/i", "m/pattern/"];

    for input in regex_cases {
        let (pattern, _body, mods) = extract_regex_parts(input);
        println!("{} -> pattern:'{}', mods:'{}'", input, pattern, mods);
    }
}

/// Ultra-comprehensive integration test to catch any remaining edge cases
#[test]
fn test_comprehensive_integration_final_push() {
    // Test complex cases that exercise multiple mutation points simultaneously

    // Case 1: Complex substitution with nested delimiters and arithmetic
    let complex = "s{old{nested}path}{new{nested}path}gi";
    let (pattern, replacement, modifiers) = extract_substitution_parts(complex);

    // This should work with:
    // - Correct arithmetic (+ not *, += not -=)
    // - Correct boolean logic (&& not ||, != not ==)
    // - Correct match arms (all delimiters supported)
    // - Correct function returns (not "xyzzy" variants)
    assert_eq!(pattern, "old{nested}path", "Complex pattern extraction");
    assert_eq!(replacement, "new{nested}path", "Complex replacement extraction");
    assert_eq!(modifiers, "gi", "Complex modifiers extraction");

    // Case 2: Edge arithmetic case
    let arithmetic_edge = "s/🦀/🔥/"; // Unicode chars to stress arithmetic
    let (a_pattern, a_replacement, _) = extract_substitution_parts(arithmetic_edge);
    assert_eq!(a_pattern, "🦀", "Unicode arithmetic should work");
    assert_eq!(a_replacement, "🔥", "Unicode replacement arithmetic should work");

    // Case 3: All delimiter types
    let delimiter_test_cases = vec![
        ("s(a)(b)", ("a", "b")),
        ("s[a][b]", ("a", "b")),
        ("s{a}{b}", ("a", "b")),
        ("s<a><b>", ("a", "b")),
        ("s/a/b/", ("a", "b")),
        ("s#a#b#", ("a", "b")),
    ];

    for (input, (exp_pattern, exp_replacement)) in delimiter_test_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);
        assert_eq!(pattern, exp_pattern, "Delimiter test pattern for {}", input);
        assert_eq!(replacement, exp_replacement, "Delimiter test replacement for {}", input);
    }
}
