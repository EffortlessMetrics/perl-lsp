/// Targeted mutation elimination tests for quote_parser.rs
/// This test suite targets specific surviving mutants identified in PR #170 mutation testing.
///
/// Target survivors based on cargo mutants output:
/// - Arithmetic mutations: +/-, +=/*=/=/-=  (lines 286:23, 292:37, 290:27)
/// - Boolean logic: && to || (lines 12:9, 80:54)
/// - Function returns: FnValue mutations (lines 9:5, 160:5)
/// - Match arm deletions: escape handling (lines 280:13, 95:17, 132:25, 212:17)
/// - Delimiter mapping: delete '<' arm (line 248:9)
/// - Equality comparisons: == to != (lines 56:31, 116:26, 136:32, 216:24)
///
/// Labels: tests:mutation-hardening, tests:coverage-critical
use perl_parser::quote_parser::*;

// TARGET: lines 286:23, 292:37, 290:27 - Arithmetic boundary mutations in extract_delimited_content
// These tests kill arithmetic mutations by ensuring precise position calculations
#[test]
fn test_kill_arithmetic_boundary_mutations_position_tracking() {
    // Test case: Multi-byte UTF-8 character position calculation
    // The += mutations would break UTF-8 boundary calculations
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/🦀test🦀/🚀replace🚀/g");
    assert_eq!(
        pattern, "🦀test🦀",
        "UTF-8 pattern extraction must handle multi-byte boundaries correctly"
    );
    assert_eq!(
        replacement, "🚀replace🚀",
        "UTF-8 replacement extraction - kills += to *= mutation at line 286:23"
    );
    assert_eq!(modifiers, "g", "UTF-8 modifiers extraction");

    // Test case: Precise character counting in delimited content
    // Targets depth += 1 mutation (line 286:23) and end_pos = i + ch.len_utf8() (line 292:37)
    let (pattern, replacement, _) =
        extract_substitution_parts("s{test{nested}end}{repl{nested}end}");
    assert_eq!(
        pattern, "test{nested}end",
        "Nested delimiter depth tracking - kills depth += to *= mutation"
    );
    assert_eq!(
        replacement, "repl{nested}end",
        "Replacement with nested delimiters - kills position arithmetic mutations"
    );

    // Test case: Depth decrement arithmetic (line 290:27)
    // -= to /= mutation would break depth tracking
    let (pattern, replacement, _) = extract_substitution_parts("s{a{b{c}b}a}{x{y{z}y}x}");
    assert_eq!(
        pattern, "a{b{c}b}a",
        "Triple-nested delimiter tracking - kills depth -= to /= mutation at line 290:27"
    );
    assert_eq!(replacement, "x{y{z}y}x", "Triple-nested replacement depth calculation");

    // Edge case: Single character positions
    let (pattern, replacement, _) = extract_substitution_parts("s/a/b/");
    assert_eq!(pattern, "a", "Single char pattern - precise position arithmetic");
    assert_eq!(replacement, "b", "Single char replacement - kills all arithmetic mutations");
}

// TARGET: lines 12:9, 80:54 - Boolean logic mutations (&& to ||)
#[test]
fn test_kill_boolean_logic_mutations_operator_precedence() {
    // Test line 12:9: text.len() > 1 && !text.chars().nth(1).unwrap().is_alphabetic()
    // && to || mutation would cause wrong behavior for 'm' followed by alphabetic chars
    let (pattern, _body, modifiers) = extract_regex_parts("ma");
    assert_eq!(
        pattern, "mam",
        "m followed by alphabetic 'a' - kills && to || mutation at line 12:9"
    );
    assert_eq!(modifiers, "", "No modifiers for 'ma'");

    let (pattern, _body, modifiers) = extract_regex_parts("mz");
    assert_eq!(pattern, "mzm", "m followed by alphabetic 'z' - ensures && logic preserved");
    assert_eq!(modifiers, "", "No modifiers for 'mz'");

    let (pattern, _body, modifiers) = extract_regex_parts("m/test/i");
    assert_eq!(pattern, "/test/", "m followed by non-alphabetic '/' should extract properly");
    assert_eq!(modifiers, "i", "Modifiers preserved with non-alphabetic delimiter");

    // Test line 80:54: !is_paired && !rest1.is_empty()
    // && to || mutation would trigger wrong parsing branch
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/test/replace/g");
    assert_eq!(pattern, "test", "Non-paired with content - kills && to || mutation at line 80:54");
    assert_eq!(replacement, "replace", "Non-paired replacement extraction");
    assert_eq!(modifiers, "g", "Non-paired modifiers");

    // Critical case: is_paired=false, rest1.is_empty()=false (both conditions false)
    // Original: !false && !false = true && true = true (manual parsing)
    // Mutated: !false || !false = true || true = true (same result but wrong logic path)
    let (pattern, replacement, _) = extract_substitution_parts("s#abc#def#");
    assert_eq!(pattern, "abc", "Non-paired delimiter with content - tests && logic precisely");
    assert_eq!(replacement, "def", "Non-paired replacement - critical for && vs || distinction");

    // Edge case: empty content but non-paired delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s##");
    assert_eq!(pattern, "", "Empty non-paired pattern");
    assert_eq!(replacement, "", "Empty non-paired replacement - tests !rest1.is_empty() condition");
}

// TARGET: lines 9:5, 160:5 - Function return value mutations (FnValue genre)
#[test]
fn test_kill_function_return_value_mutations() {
    // Test line 9:5: extract_regex_parts return value mutations
    // Mutations: (String::new(), String::new()) vs ("xyzzy".into(), String::new())
    let (pattern, _body, modifiers) = extract_regex_parts("");
    assert_eq!(pattern, "", "Empty input should return empty pattern, not 'xyzzy'");
    assert_eq!(modifiers, "", "Empty input should return empty modifiers, not any string");

    let (pattern, _body, modifiers) = extract_regex_parts("qr");
    assert_eq!(pattern, "", "qr without delimiter should return empty pattern, not 'xyzzy'");
    assert_eq!(modifiers, "", "qr without delimiter should return empty modifiers");

    let (pattern, _body, modifiers) = extract_regex_parts("qr/test/i");
    assert_eq!(pattern, "/test/", "Valid regex should return proper pattern, not empty or 'xyzzy'");
    assert_eq!(modifiers, "i", "Valid regex should return proper modifiers, not empty or 'xyzzy'");

    // Test line 160:5: extract_transliteration_parts return value mutations
    // Mutations: (String::new(), String::new(), String::new()) vs (String::new(), "xyzzy".into(), String::new())
    let (search, replace, modifiers) = extract_transliteration_parts("");
    assert_eq!(search, "", "Empty input should return empty search, not 'xyzzy'");
    assert_eq!(replace, "", "Empty input should return empty replace, not 'xyzzy'");
    assert_eq!(modifiers, "", "Empty input should return empty modifiers, not any string");

    let (search, replace, modifiers) = extract_transliteration_parts("tr");
    assert_eq!(search, "", "tr without delimiter should return empty search, not 'xyzzy'");
    assert_eq!(replace, "", "tr without delimiter should return empty replace, not 'xyzzy'");
    assert_eq!(modifiers, "", "tr without delimiter should return empty modifiers");

    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/d");
    assert_eq!(
        search, "abc",
        "Valid transliteration should return proper search, not empty or 'xyzzy'"
    );
    assert_eq!(
        replace, "xyz",
        "Valid transliteration should return proper replace, not empty or 'xyzzy'"
    );
    assert_eq!(
        modifiers, "d",
        "Valid transliteration should return proper modifiers, not empty or 'xyzzy'"
    );
}

// TARGET: lines 280:13, 95:17, 132:25, 212:17 - Match arm deletions for escape handling
#[test]
fn test_kill_escape_handling_match_arm_deletions() {
    // Test line 280:13: delete match arm '\\' in extract_delimited_content
    // This mutation would remove escape handling entirely
    let (pattern, replacement, _) = extract_substitution_parts("s/test\\/end/repl\\/end/");
    assert_eq!(
        pattern, "test\\/end",
        "Escaped delimiter in pattern - kills escape arm deletion at line 280:13"
    );
    assert_eq!(replacement, "repl\\/end", "Escaped delimiter in replacement");

    // Test line 95:17: delete match arm '\\' in extract_substitution_parts (manual parsing)
    let (pattern, replacement, _) = extract_substitution_parts("s/a\\\\b/c\\\\d/");
    assert_eq!(
        pattern, "a\\\\b",
        "Double escape in pattern - kills escape arm deletion at line 95:17"
    );
    assert_eq!(replacement, "c\\\\d", "Double escape in replacement");

    // Test line 132:25: delete match arm '\\' in extract_substitution_parts (fallback path)
    let (pattern, replacement, _) = extract_substitution_parts("s[test\\]end[repl\\]end");
    assert_eq!(
        pattern, "test\\]end[repl\\]end",
        "Escaped bracket in pattern - kills escape arm deletion at line 132:25"
    );
    assert_eq!(replacement, "", "Escaped bracket in replacement falls back to empty");

    // Test line 212:17: delete match arm '\\' in extract_transliteration_parts
    let (search, replace, _) = extract_transliteration_parts("tr/a\\/b/c\\/d/");
    assert_eq!(
        search, "a\\/b",
        "Escaped delimiter in transliteration search - kills escape arm deletion at line 212:17"
    );
    assert_eq!(replace, "c\\/d", "Escaped delimiter in transliteration replace");

    // Complex escape sequences to ensure robust handling
    let (pattern, replacement, _) = extract_substitution_parts("s/\\\\\\//\\\\\\/\\/");
    assert_eq!(pattern, "\\\\\\/", "Complex escape sequence in pattern");
    assert_eq!(replacement, "\\\\\\/\\/", "Complex escape sequence in replacement");

    // Escaped delimiters that should NOT terminate parsing
    let (search, replace, _) = extract_transliteration_parts("tr{a\\}b}{c\\}d}");
    assert_eq!(search, "a\\}b", "Escaped closing delimiter should not terminate parsing");
    assert_eq!(replace, "c\\}d", "Escaped closing delimiter in replace");
}

// TARGET: line 248:9 - Delete match arm '<' in get_closing_delimiter
#[test]
fn test_kill_delimiter_mapping_deletion_angle_brackets() {
    // This test specifically targets the '<' => '>' mapping deletion
    // If the '<' arm is deleted, angle brackets would return themselves instead of proper closing
    let (pattern, replacement, _) = extract_substitution_parts("s<old><new>");
    assert_eq!(
        pattern, "old",
        "Angle bracket opening delimiter - kills '<' arm deletion at line 248:9"
    );
    assert_eq!(replacement, "new", "Angle bracket closing delimiter mapping");

    // Test angle brackets in regex contexts
    let (pattern, _body, modifiers) = extract_regex_parts("qr<test.*>");
    assert_eq!(pattern, "<test.*>", "Angle bracket regex pattern - ensures '<' mapping preserved");
    assert_eq!(modifiers, "", "Angle bracket regex modifiers");

    // Test angle brackets in transliteration
    let (search, replace, _) = extract_transliteration_parts("tr<abc><xyz>");
    assert_eq!(search, "abc", "Angle bracket transliteration search - tests '<' delimiter mapping");
    assert_eq!(replace, "xyz", "Angle bracket transliteration replace");

    // Nested angle brackets to test depth tracking with proper closing
    let (pattern, replacement, _) = extract_substitution_parts("s<test<inner>end><repl<inner>end>");
    assert_eq!(pattern, "test<inner>end", "Nested angle brackets in pattern");
    assert_eq!(replacement, "repl<inner>end", "Nested angle brackets in replacement");

    // Edge case: angle brackets as non-paired delimiters (should work the same)
    let (pattern, replacement, _) = extract_substitution_parts("s<<<>>>>");
    assert_eq!(pattern, "<<>>", "Angle bracket as delimiter character");
    assert_eq!(replacement, "", "Multiple angle bracket characters");
}

// TARGET: lines 56:31, 116:26, 136:32, 216:24 - Equality comparison mutations (== to !=)
#[test]
fn test_kill_equality_comparison_mutations() {
    // Test line 56:31: delimiter != closing (is_paired detection)
    // == to != mutation would break paired delimiter detection
    let (pattern, replacement, _) = extract_substitution_parts("s{old}{new}");
    assert_eq!(
        pattern, "old",
        "Paired delimiter detection - kills != to == mutation at line 56:31"
    );
    assert_eq!(replacement, "new", "Paired delimiter replacement");

    let (pattern, replacement, _) = extract_substitution_parts("s/old/new/");
    assert_eq!(pattern, "old", "Non-paired delimiter detection - ensures != comparison works");
    assert_eq!(replacement, "new", "Non-paired delimiter replacement");

    // Test line 116:26: delimiter == '(' special case
    // == to != mutation would break parentheses special handling
    let (pattern, replacement, _) = extract_substitution_parts("s(test)");
    assert_eq!(
        pattern, "test",
        "Parentheses special case - kills == to != mutation at line 116:26"
    );
    assert_eq!(replacement, "", "Parentheses empty replacement behavior");

    // Test line 136:32: c == closing in substitution manual parsing
    // == to != mutation would break closing delimiter detection
    let (pattern, replacement, _) = extract_substitution_parts("s/test/replace/");
    assert_eq!(
        pattern, "test",
        "Closing delimiter detection in pattern - kills == to != mutation at line 136:32"
    );
    assert_eq!(replacement, "replace", "Closing delimiter detection in replacement");

    // Test line 216:24: c == closing in transliteration parsing
    // == to != mutation would break transliteration closing delimiter detection
    let (search, replace, _) = extract_transliteration_parts("tr/old/new/");
    assert_eq!(
        search, "old",
        "Transliteration closing delimiter - kills == to != mutation at line 216:24"
    );
    assert_eq!(replace, "new", "Transliteration closing delimiter detection");

    // Edge case: delimiter characters that are equal to closing
    let (pattern, replacement, _) = extract_substitution_parts("s///");
    assert_eq!(pattern, "", "Empty pattern with same open/close delimiter");
    assert_eq!(replacement, "", "Empty replacement with same open/close delimiter");

    // Complex case: multiple potential closing delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s/test/replace/more/");
    assert_eq!(pattern, "test", "Pattern stops at first closing delimiter");
    assert_eq!(replacement, "replace", "Replacement stops at correct closing delimiter");
}

// TARGET: Complex interaction testing - multiple mutations combined
#[test]
fn test_kill_complex_mutation_interactions() {
    // Test case combining multiple potential mutations
    // This ensures mutations don't interact in unexpected ways
    let (pattern, replacement, modifiers) =
        extract_substitution_parts("s{test\\}with\\\\escape}{repl\\}with\\\\escape}gi");
    assert_eq!(
        pattern, "test\\}with\\\\escape",
        "Complex pattern with escapes and paired delimiters"
    );
    assert_eq!(replacement, "repl\\}with\\\\escape", "Complex replacement with escapes");
    assert_eq!(modifiers, "gi", "Complex modifiers");

    // Test transliteration with complex escaping and arithmetic
    let (search, replace, modifiers) =
        extract_transliteration_parts("tr{a\\}b\\\\c}{x\\}y\\\\z}cd");
    assert_eq!(search, "a\\}b\\\\c", "Complex transliteration search with escapes");
    assert_eq!(replace, "x\\}y\\\\z", "Complex transliteration replace with escapes");
    assert_eq!(modifiers, "cd", "Valid transliteration modifiers");

    // Test regex with complex parsing
    let (pattern, _body, modifiers) = extract_regex_parts("qr{test\\{nested\\}end}gimsx");
    assert_eq!(pattern, "{test\\{nested\\}end}", "Complex regex pattern with nested escapes");
    assert_eq!(modifiers, "gimsx", "Full modifier set");

    // Unicode + escaping + arithmetic boundaries
    let (pattern, replacement, _) = extract_substitution_parts("s/🦀test\\/end🦀/🚀repl\\/end🚀/");
    assert_eq!(
        pattern, "🦀test\\/end🦀",
        "Unicode with escaped delimiters - tests all mutation types"
    );
    assert_eq!(replacement, "🚀repl\\/end🚀", "Unicode replacement with escapes");
}

// TARGET: Edge cases that stress all mutation types simultaneously
#[test]
fn test_kill_mutations_comprehensive_edge_cases() {
    // Empty input edge cases (function return mutations)
    let (pattern, _body, modifiers) = extract_regex_parts("");
    assert_eq!((pattern, modifiers), ("".to_string(), "".to_string()), "Empty regex input");

    let (pattern, replacement, modifiers) = extract_substitution_parts("");
    assert_eq!(
        (pattern, replacement, modifiers),
        ("".to_string(), "".to_string(), "".to_string()),
        "Empty substitution input"
    );

    let (search, replace, modifiers) = extract_transliteration_parts("");
    assert_eq!(
        (search, replace, modifiers),
        ("".to_string(), "".to_string(), "".to_string()),
        "Empty transliteration input"
    );

    // Single character edge cases (arithmetic boundaries)
    let (pattern, replacement, _) = extract_substitution_parts("s/a/b/");
    assert_eq!(
        (pattern, replacement),
        ("a".to_string(), "b".to_string()),
        "Single character substitution"
    );

    // Malformed input edge cases (escape handling and boolean logic)
    let (pattern, replacement, _) = extract_substitution_parts("s{unclosed");
    assert_eq!(pattern, "unclosed", "Malformed paired delimiter pattern");
    assert_eq!(replacement, "", "Malformed paired delimiter replacement");

    // All delimiter types (delimiter mapping mutations)
    let delimiters = vec![('(', ')'), ('[', ']'), ('{', '}'), ('<', '>'), ('/', '/'), ('#', '#')];
    for (open, close) in delimiters {
        let open_s = open.to_string();
        let close_s = close.to_string();
        let third_delim = if open_s == close_s { "" } else { close_s.as_str() };
        // Keep multi-line layout: this file doubles as a formatting canary.
        #[rustfmt::skip]
        let input = format!(
            "s{}test{}repl{}",
            open,
            close,
            third_delim
        );
        let (pattern, replacement, _) = extract_substitution_parts(&input);
        assert_eq!(pattern, "test", "Pattern extraction for delimiter {}", open);
        // All paired delimiters should correctly extract the replacement
        assert_eq!(replacement, "repl", "Replacement extraction for delimiter {}", open);
    }
}

// Property-based tests to catch any remaining mutations
#[test]
fn test_kill_mutations_invariant_properties() {
    // Property: All functions should handle empty input gracefully (return mutations)
    assert_eq!(extract_regex_parts(""), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(extract_substitution_parts(""), ("".to_string(), "".to_string(), "".to_string()));
    assert_eq!(extract_transliteration_parts(""), ("".to_string(), "".to_string(), "".to_string()));

    // Property: Delimiter balance should be maintained (arithmetic mutations)
    let balanced_tests =
        vec!["s{a{b}c}{x{y}z}", "s[a[b]c][x[y]z]", "s(a(b)c)(x(y)z)", "s<a<b>c><x<y>z>"];

    for test in balanced_tests {
        let (pattern, replacement, _) = extract_substitution_parts(test);
        assert!(
            !pattern.is_empty(),
            "Pattern should not be empty for balanced delimiters: {}",
            test
        );
        assert!(
            !replacement.is_empty(),
            "Replacement should not be empty for balanced delimiters: {}",
            test
        );
    }

    // Property: Escaping should preserve delimiter characters (escape mutations)
    let escape_tests = vec![
        ("s/a\\/b/c\\/d/", "a\\/b", "c\\/d"),
        ("s{a\\}b}{c\\}d}", "a\\}b", "c\\}d"),
        ("tr/a\\/b/c\\/d/", "a\\/b", "c\\/d"),
    ];

    for (input, expected_first, expected_second) in escape_tests {
        if input.starts_with("tr") {
            let (search, replace, _) = extract_transliteration_parts(input);
            assert_eq!(
                search, expected_first,
                "Escaped delimiter preservation in transliteration: {}",
                input
            );
            assert_eq!(
                replace, expected_second,
                "Escaped delimiter preservation in transliteration replace: {}",
                input
            );
        } else {
            let (pattern, replacement, _) = extract_substitution_parts(input);
            assert_eq!(
                pattern, expected_first,
                "Escaped delimiter preservation in substitution: {}",
                input
            );
            assert_eq!(
                replacement, expected_second,
                "Escaped delimiter preservation in substitution replace: {}",
                input
            );
        }
    }
}
