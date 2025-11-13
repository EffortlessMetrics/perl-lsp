/// Critical mutation elimination tests for quote_parser.rs
/// These tests target the 52 highest-impact surviving mutants identified in PR #165
/// to improve mutation score from 37% to ≥80%.
///
/// Key targets:
/// - MISSED: replace != with == in extract_transliteration_parts (line 174:31)
/// - MISSED: delete match arm '\\' in extract_delimited_content (line 280:13)
/// - MISSED: delete match arm '<' in get_closing_delimiter (line 248:9)
/// - MISSED: replace == with != in extract_substitution_parts (line 136:32)
/// - MISSED: replace > with < in extract_regex_parts (line 12:23)
/// - MISSED: replace + with - in extract_substitution_parts (line 137:41)
/// - MISSED: replace && with || in extract_regex_parts (line 12:9)
/// - MISSED: delete match arm '{' in get_closing_delimiter (line 247:9)
/// - MISSED: delete match arm '(' in get_closing_delimiter (line 245:9)
/// - MISSED: delete match arm '[' in get_closing_delimiter (line 246:9)
use perl_parser::quote_parser::*;

/// Test specifically targeting the != vs == mutation in extract_transliteration_parts line 174:31
/// This mutation changes `delimiter != closing` to `delimiter == closing`
#[test]
fn test_kill_transliteration_delimiter_comparison_mutation() {
    // Case 1: Paired delimiters where delimiter != closing (should be true originally)
    // Original: '(' != ')' = true (is_paired = true)
    // Mutated:  '(' == ')' = false (is_paired = false) - would break paired delimiter logic
    let (search, replace, modifiers) = extract_transliteration_parts("tr(abc)(xyz)d");
    assert_eq!(search, "abc", "Paired delimiter search - kills != to == mutation");
    assert_eq!(replace, "xyz", "Paired delimiter replace - kills != to == mutation");
    assert_eq!(modifiers, "d", "Paired delimiter modifiers - kills != to == mutation");

    // Case 2: Non-paired delimiters where delimiter == closing (should be false originally)
    // Original: '/' != '/' = false (is_paired = false)
    // Mutated:  '/' == '/' = true (is_paired = true) - would break non-paired delimiter logic
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/d");
    assert_eq!(search, "abc", "Non-paired delimiter search - kills != to == mutation");
    assert_eq!(replace, "xyz", "Non-paired delimiter replace - kills != to == mutation");
    assert_eq!(modifiers, "d", "Non-paired delimiter modifiers - kills != to == mutation");

    // Case 3: Test all paired delimiter types to ensure comprehensive coverage
    let paired_cases = vec![
        ("tr{abc}{xyz}", ("abc", "xyz", "")),
        ("tr[abc][xyz]", ("abc", "xyz", "")),
        ("tr<abc><xyz>", ("abc", "xyz", "")),
        ("tr(abc)(xyz)", ("abc", "xyz", "")),
    ];

    for (input, expected) in paired_cases {
        let result = extract_transliteration_parts(input);
        assert_eq!(
            (result.0.as_str(), result.1.as_str(), result.2.as_str()),
            expected,
            "Paired delimiter test for {} - kills != to == mutation",
            input
        );
    }
}

/// Test targeting the escape character '\\' match arm deletion mutation in extract_delimited_content line 280:13
#[test]
fn test_kill_escape_character_match_arm_deletion() {
    // Test cases where the '\\' match arm is critical - if deleted, escaping would break

    // Case 1: Escaped delimiter in pattern - without '\\' match arm, this would terminate early
    let (pattern, replacement, _) =
        extract_substitution_parts("s/test\\/with\\/slashes/replacement/");
    assert_eq!(
        pattern, "test\\/with\\/slashes",
        "Escaped slashes in pattern - kills '\\\\' match arm deletion"
    );
    assert_eq!(
        replacement, "replacement",
        "Replacement with escaped pattern - kills '\\\\' match arm deletion"
    );

    // Case 2: Escaped delimiter in replacement - without '\\' match arm, this would terminate early
    let (pattern, replacement, _) = extract_substitution_parts("s/pattern/repl\\/with\\/slashes/");
    assert_eq!(
        pattern, "pattern",
        "Pattern with escaped replacement - kills '\\\\' match arm deletion"
    );
    assert_eq!(
        replacement, "repl\\/with\\/slashes",
        "Escaped slashes in replacement - kills '\\\\' match arm deletion"
    );

    // Case 3: Multiple escape sequences
    let (pattern, replacement, _) = extract_substitution_parts("s/a\\\\b\\\\c/x\\\\y\\\\z/");
    assert_eq!(
        pattern, "a\\\\b\\\\c",
        "Multiple escaped backslashes in pattern - kills '\\\\' match arm deletion"
    );
    assert_eq!(
        replacement, "x\\\\y\\\\z",
        "Multiple escaped backslashes in replacement - kills '\\\\' match arm deletion"
    );

    // Case 4: Escaped delimiter followed by actual delimiter
    let (pattern, replacement, _) = extract_substitution_parts("s/test\\/end/done/");
    assert_eq!(
        pattern, "test\\/end",
        "Escaped delimiter followed by content - kills '\\\\' match arm deletion"
    );
    assert_eq!(
        replacement, "done",
        "Replacement after escaped delimiter - kills '\\\\' match arm deletion"
    );

    // Case 5: Test with different delimiter types
    let (search, replace, _) = extract_transliteration_parts("tr{a\\}b\\}c}{x\\}y\\}z}");
    assert_eq!(
        search, "a\\}b\\}c",
        "Escaped braces in transliteration search - kills '\\\\' match arm deletion"
    );
    assert_eq!(
        replace, "x\\}y\\}z",
        "Escaped braces in transliteration replace - kills '\\\\' match arm deletion"
    );
}

/// Test targeting match arm deletions for all delimiter types in get_closing_delimiter
/// Specifically targets deletions of '(', '[', '{', '<' match arms
#[test]
fn test_kill_delimiter_match_arm_deletions() {
    // Test each delimiter type individually to ensure the match arm is required

    // Test parentheses - kills deletion of '(' => ')' match arm
    let (pattern, _replacement, _) = extract_substitution_parts("s(test)(repl)");
    // Note: parentheses have special behavior in substitution, but the delimiter mapping must work
    assert_eq!(pattern, "test", "Parentheses pattern extraction - kills '(' match arm deletion");

    // Test square brackets - kills deletion of '[' => ']' match arm
    let (pattern, replacement, _) = extract_substitution_parts("s[test][repl]");
    assert_eq!(pattern, "test", "Brackets pattern extraction - kills '[' match arm deletion");
    assert_eq!(
        replacement, "repl",
        "Brackets replacement extraction - kills '[' match arm deletion"
    );

    // Test curly braces - kills deletion of '{' => '}' match arm
    let (pattern, replacement, _) = extract_substitution_parts("s{test}{repl}");
    assert_eq!(pattern, "test", "Braces pattern extraction - kills '{{' match arm deletion");
    assert_eq!(
        replacement, "repl",
        "Braces replacement extraction - kills '{{' match arm deletion"
    );

    // Test angle brackets - kills deletion of '<' => '>' match arm
    let (pattern, replacement, _) = extract_substitution_parts("s<test><repl>");
    assert_eq!(pattern, "test", "Angle brackets pattern extraction - kills '<' match arm deletion");
    assert_eq!(
        replacement, "repl",
        "Angle brackets replacement extraction - kills '<' match arm deletion"
    );

    // Test nested delimiters to ensure proper depth tracking with correct closing delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s{te{s}t}{re{p}l}");
    assert_eq!(
        pattern, "te{s}t",
        "Nested braces in pattern - kills '{{' match arm deletion with depth tracking"
    );
    assert_eq!(
        replacement, "re{p}l",
        "Nested braces in replacement - kills '{{' match arm deletion with depth tracking"
    );

    let (search, replace, _) = extract_transliteration_parts("tr[a[b]c][x[y]z]");
    assert_eq!(
        search, "a[b]c",
        "Nested brackets in transliteration search - kills '[' match arm deletion"
    );
    assert_eq!(
        replace, "x[y]z",
        "Nested brackets in transliteration replace - kills '[' match arm deletion"
    );
}

/// Test targeting the == vs != mutation in extract_substitution_parts line 136:32
/// This targets the condition `c if c == closing`
#[test]
fn test_kill_closing_delimiter_comparison_mutation() {
    // Test cases where the closing delimiter comparison is critical

    // Case 1: Closing delimiter should terminate parsing
    // Original: c == closing (true when c is closing delimiter)
    // Mutated:  c != closing (false when c is closing delimiter) - would not terminate
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/test/repl/g");
    assert_eq!(
        pattern, "test",
        "Pattern terminated by closing delimiter - kills == to != mutation"
    );
    assert_eq!(
        replacement, "repl",
        "Replacement terminated by closing delimiter - kills == to != mutation"
    );
    assert_eq!(modifiers, "g", "Modifiers after closing delimiter - kills == to != mutation");

    // Case 2: Different closing delimiters
    let test_cases = vec![
        ("s{pat}{repl}g", ("pat", "repl", "g")),
        ("s[pat][repl]g", ("pat", "repl", "g")),
        ("s<pat><repl>g", ("pat", "repl", "g")),
        ("s(pat)(repl)g", ("pat", "repl", "g")), // Parentheses behavior
        ("s#pat#repl#g", ("pat", "repl", "g")),
    ];

    for (input, expected) in test_cases {
        let result = extract_substitution_parts(input);
        assert_eq!(
            (result.0.as_str(), result.1.as_str(), result.2.as_str()),
            expected,
            "Closing delimiter test for {} - kills == to != mutation",
            input
        );
    }
}

/// Test targeting the > vs < mutation in extract_regex_parts line 12:23
/// This targets the condition `text.len() > 1`
#[test]
fn test_kill_length_comparison_mutation_regex() {
    // Test boundary cases for length comparison

    // Case 1: Length exactly 1 - should not strip 'm' prefix
    // Original: "m".len() > 1 = false (don't strip)
    // Mutated:  "m".len() < 1 = false (don't strip) - same result but test boundary
    let (pattern, modifiers) = extract_regex_parts("m");
    assert_eq!(pattern, "mm", "Single 'm' should not strip prefix - tests > vs < boundary");
    assert_eq!(modifiers, "", "Single 'm' modifiers - tests > vs < boundary");

    // Case 2: Length exactly 2 with alphabetic - should not strip 'm' prefix
    // Original: "ma".len() > 1 = true, but followed by alphabetic so don't strip
    // Mutated:  "ma".len() < 1 = false (wrong branch) - would affect logic
    let (pattern, modifiers) = extract_regex_parts("ma");
    assert_eq!(pattern, "mam", "Two chars with alphabetic - kills > to < mutation");
    assert_eq!(modifiers, "", "Alphabetic after m - kills > to < mutation");

    // Case 3: Length exactly 2 with non-alphabetic - should strip 'm' prefix
    // Original: "m/".len() > 1 = true, not alphabetic so strip
    // Mutated:  "m/".len() < 1 = false (wrong branch) - would not strip
    let (pattern, modifiers) = extract_regex_parts("m/test/");
    assert_eq!(pattern, "/test/", "Two chars with non-alphabetic - kills > to < mutation");
    assert_eq!(modifiers, "", "Non-alphabetic after m - kills > to < mutation");

    // Case 4: Empty string
    // Original: "".len() > 1 = false
    // Mutated:  "".len() < 1 = true (wrong result) - would affect empty string handling
    let (pattern, modifiers) = extract_regex_parts("");
    assert_eq!(pattern, "", "Empty string handling - kills > to < mutation");
    assert_eq!(modifiers, "", "Empty string modifiers - kills > to < mutation");
}

/// Test targeting the + vs - mutation in extract_substitution_parts line 137:41
/// This targets character position calculation `i + ch.len_utf8()`
#[test]
fn test_kill_character_position_arithmetic_mutation() {
    // Test cases where character position calculation is critical

    // Case 1: Multi-byte UTF-8 characters where len_utf8() > 1
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/café/茶/g");
    assert_eq!(pattern, "café", "UTF-8 pattern with multi-byte chars - kills + to - mutation");
    assert_eq!(
        replacement, "茶",
        "UTF-8 replacement with multi-byte chars - kills + to - mutation"
    );
    assert_eq!(modifiers, "g", "UTF-8 modifiers - kills + to - mutation");

    // Case 2: Pattern with emoji (4-byte UTF-8 characters)
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/test🦀/repl🦀/");
    assert_eq!(
        pattern, "test🦀",
        "Emoji in pattern - kills + to - mutation in position calculation"
    );
    assert_eq!(
        replacement, "repl🦀",
        "Emoji in replacement - kills + to - mutation in position calculation"
    );
    assert_eq!(modifiers, "", "Emoji test modifiers - kills + to - mutation");

    // Case 3: Mixed ASCII and multi-byte characters
    let (pattern, replacement, _) = extract_substitution_parts("s/a🌟b/x🎯y/");
    assert_eq!(pattern, "a🌟b", "Mixed ASCII/multi-byte pattern - kills + to - mutation");
    assert_eq!(replacement, "x🎯y", "Mixed ASCII/multi-byte replacement - kills + to - mutation");

    // Case 4: Multiple delimiters in sequence with UTF-8
    let (search, replace, _) = extract_transliteration_parts("tr/αβγ/ΑΒΓ/");
    assert_eq!(search, "αβγ", "Greek letters in transliteration - kills + to - mutation");
    assert_eq!(replace, "ΑΒΓ", "Greek capitals in transliteration - kills + to - mutation");
}

/// Test targeting the && vs || mutation in extract_regex_parts line 12:9
/// This targets `text.starts_with('m') && text.len() > 1 && !text.chars().nth(1).unwrap().is_alphabetic()`
#[test]
fn test_kill_logical_and_to_or_mutation_regex() {
    // Test all combinations of the three conditions to distinguish && from ||

    // Case 1: All conditions true - should strip 'm'
    // Original: true && true && true = true (strip 'm')
    // Mutated:  true || true || true = true (same result but need to verify correct behavior)
    let (pattern, modifiers) = extract_regex_parts("m/test/");
    assert_eq!(pattern, "/test/", "All conditions true - kills && to || mutation");
    assert_eq!(modifiers, "", "All conditions true modifiers - kills && to || mutation");

    // Case 2: First condition false - should not strip
    // Original: false && ... = false (don't strip)
    // Mutated:  false || ... = depends on other conditions (could be different)
    let (pattern, modifiers) = extract_regex_parts("x/test/");
    assert_eq!(pattern, "x/test/x", "First condition false - kills && to || mutation");
    assert_eq!(modifiers, "", "First condition false modifiers - kills && to || mutation");

    // Case 3: First two conditions true, third false - should not strip
    // Original: true && true && false = false (don't strip)
    // Mutated:  true || true || false = true (would strip incorrectly)
    let (pattern, modifiers) = extract_regex_parts("ma");
    assert_eq!(pattern, "mam", "Third condition false - kills && to || mutation");
    assert_eq!(modifiers, "", "Third condition false modifiers - kills && to || mutation");

    // Case 4: First condition true, second false - should not strip
    // Original: true && false && ... = false (don't strip)
    // Mutated:  true || false || ... = depends on third condition
    let (pattern, modifiers) = extract_regex_parts("m");
    assert_eq!(pattern, "mm", "Second condition false - kills && to || mutation");
    assert_eq!(modifiers, "", "Second condition false modifiers - kills && to || mutation");

    // Case 5: Verify alphabetic detection specifically
    let alphabetic_cases = vec![
        ("ma", "mam"), // 'a' is alphabetic
        ("mz", "mzm"), // 'z' is alphabetic
        ("mA", "mAm"), // 'A' is alphabetic
        ("mZ", "mZm"), // 'Z' is alphabetic
    ];

    for (input, expected_pattern) in alphabetic_cases {
        let (pattern, modifiers) = extract_regex_parts(input);
        assert_eq!(
            pattern, expected_pattern,
            "Alphabetic test for {} - kills && to || mutation",
            input
        );
        assert_eq!(modifiers, "", "Alphabetic modifiers for {} - kills && to || mutation", input);
    }
}

/// Test targeting return value mutations that replace function results with hardcoded strings
#[test]
fn test_kill_return_value_hardcoded_mutations() {
    // These tests target mutations that replace return values with "xyzzy" or String::new()

    // Test extract_transliteration_parts return value mutations
    let (search, replace, modifiers) = extract_transliteration_parts("tr{search}{replace}d");
    assert_ne!(
        search, "xyzzy",
        "Search should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(
        replace, "xyzzy",
        "Replace should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(search, "", "Search should not be empty string - kills String::new() mutation");
    assert_ne!(replace, "", "Replace should not be empty string - kills String::new() mutation");
    assert_eq!(search, "search", "Search should be actual content");
    assert_eq!(replace, "replace", "Replace should be actual content");
    assert_eq!(modifiers, "d", "Modifiers should be actual content");

    // Test with different content to ensure not hardcoded
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/cd");
    assert_ne!(
        search, "xyzzy",
        "Different search should not be xyzzy - kills return value mutation"
    );
    assert_ne!(
        replace, "xyzzy",
        "Different replace should not be xyzzy - kills return value mutation"
    );
    assert_eq!(search, "abc", "Search should match input content");
    assert_eq!(replace, "xyz", "Replace should match input content");
    assert_eq!(modifiers, "cd", "Modifiers should match input content");

    // Test extract_substitution_parts return value mutations
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/pattern/replacement/gi");
    assert_ne!(
        pattern, "xyzzy",
        "Pattern should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(
        replacement, "xyzzy",
        "Replacement should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(pattern, "", "Pattern should not be empty - kills String::new() mutation");
    assert_ne!(replacement, "", "Replacement should not be empty - kills String::new() mutation");
    assert_eq!(pattern, "pattern", "Pattern should match input");
    assert_eq!(replacement, "replacement", "Replacement should match input");
    assert_eq!(modifiers, "gi", "Modifiers should match input");

    // Test extract_regex_parts return value mutations
    let (pattern, modifiers) = extract_regex_parts("qr{regex}ig");
    assert_ne!(
        pattern, "xyzzy",
        "Regex pattern should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(
        modifiers, "xyzzy",
        "Regex modifiers should not be hardcoded xyzzy - kills return value mutation"
    );
    assert_ne!(pattern, "", "Regex pattern should not be empty - kills String::new() mutation");
    assert_eq!(pattern, "{regex}", "Regex pattern should match input");
    assert_eq!(modifiers, "ig", "Regex modifiers should match input");
}

/// Test edge cases that stress the interaction between multiple mutant-prone functions
#[test]
fn test_kill_function_interaction_mutations() {
    // Test complex cases that exercise multiple functions with potential mutations

    // Case 1: Paired delimiters with escaped content
    let (pattern, replacement, modifiers) = extract_substitution_parts("s{a\\}b\\}c}{x\\}y\\}z}g");
    assert_eq!(
        pattern, "a\\}b\\}c",
        "Complex paired delimiter pattern - kills multiple interaction mutations"
    );
    assert_eq!(
        replacement, "x\\}y\\}z",
        "Complex paired delimiter replacement - kills multiple interaction mutations"
    );
    assert_eq!(
        modifiers, "g",
        "Complex paired delimiter modifiers - kills multiple interaction mutations"
    );

    // Case 2: Non-paired delimiters with complex escape sequences
    let (pattern, replacement, modifiers) =
        extract_substitution_parts("s/a\\/b\\\\c/x\\/y\\\\z/gi");
    assert_eq!(
        pattern, "a\\/b\\\\c",
        "Complex non-paired delimiter pattern - kills interaction mutations"
    );
    assert_eq!(
        replacement, "x\\/y\\\\z",
        "Complex non-paired delimiter replacement - kills interaction mutations"
    );
    assert_eq!(
        modifiers, "gi",
        "Complex non-paired delimiter modifiers - kills interaction mutations"
    );

    // Case 3: Transliteration with all delimiter types
    let delimiter_tests = vec![
        ("tr(a)(b)", ("a", "b", "")),
        ("tr[a][b]", ("a", "b", "")),
        ("tr{a}{b}", ("a", "b", "")),
        ("tr<a><b>", ("a", "b", "")),
        ("tr/a/b/", ("a", "b", "")),
        ("tr#a#b#", ("a", "b", "")),
    ];

    for (input, expected) in delimiter_tests {
        let result = extract_transliteration_parts(input);
        assert_eq!(
            (result.0.as_str(), result.1.as_str(), result.2.as_str()),
            expected,
            "Delimiter type test for {} - kills function interaction mutations",
            input
        );
    }
}

/// Test boundary conditions that are particularly prone to off-by-one mutations
#[test]
fn test_kill_boundary_arithmetic_mutations() {
    // Test cases that are sensitive to +/- mutations and comparison mutations

    // Case 1: Single character delimiters and content
    let (pattern, replacement, _) = extract_substitution_parts("s/a/b/");
    assert_eq!(pattern, "a", "Single char pattern - kills boundary arithmetic mutations");
    assert_eq!(replacement, "b", "Single char replacement - kills boundary arithmetic mutations");

    // Case 2: Empty content between delimiters
    let (pattern, replacement, _) = extract_substitution_parts("s///");
    assert_eq!(pattern, "", "Empty pattern - kills boundary arithmetic mutations");
    assert_eq!(replacement, "", "Empty replacement - kills boundary arithmetic mutations");

    // Case 3: Content with exactly one character that needs special handling
    let test_cases = vec![
        ("s/x/y/", ("x", "y")),
        ("s#x#y#", ("x", "y")),
        ("s{x}{y}", ("x", "y")),
        ("s[x][y]", ("x", "y")),
        ("s<x><y>", ("x", "y")),
    ];

    for (input, expected) in test_cases {
        let result = extract_substitution_parts(input);
        assert_eq!(
            (result.0.as_str(), result.1.as_str()),
            expected,
            "Boundary test for {} - kills arithmetic boundary mutations",
            input
        );
    }

    // Case 4: Test UTF-8 boundary handling with single multi-byte characters
    let (pattern, replacement, _) = extract_substitution_parts("s/🦀/🎯/");
    assert_eq!(pattern, "🦀", "Single emoji pattern - kills UTF-8 boundary arithmetic mutations");
    assert_eq!(
        replacement, "🎯",
        "Single emoji replacement - kills UTF-8 boundary arithmetic mutations"
    );
}
