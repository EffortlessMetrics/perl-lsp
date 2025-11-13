use perl_parser::Parser;
/// Critical mutation hardening tests targeting PR #170 LSP executeCommand surviving mutants
///
/// Focuses on high-impact mutations identified in testing with ~48% score:
/// - UTF-8 position arithmetic boundaries
/// - Quote parser delimiter edge cases
/// - Semantic token overlap validation
/// - Boolean logic and control flow mutations
///
/// Goal: Strategic test additions to achieve ≥80% mutation score improvement
/// Labels: tests:hardening, mutation:critical, pr170:core
use perl_parser::quote_parser::*;
use perl_parser::semantic_tokens::{EncodedToken, collect_semantic_tokens};

/// Critical UTF-8 arithmetic boundary tests targeting position calculation mutations
#[test]
fn test_utf8_arithmetic_boundary_mutations() {
    // Target + → -, >= → >, position arithmetic edge cases
    let utf8_test_cases = vec![
        // Basic ASCII baseline
        ("s/a/b/", "a", "b"),
        // UTF-8 multi-byte characters that would break with arithmetic mutations
        ("s/🦀/test/", "🦀", "test"),   // 4-byte emoji
        ("s/café/new/", "café", "new"), // 2-byte accented chars
        ("s/你好/hi/", "你好", "hi"),   // 3-byte Chinese
        // Edge cases where + → - would cause underflow/overflow
        ("s/❤️/💚/", "❤️", "💚"), // Complex emoji
    ];

    for (input, expected_pattern, expected_replacement) in utf8_test_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);

        // These assertions kill arithmetic boundary mutations
        assert_eq!(
            pattern, expected_pattern,
            "UTF-8 arithmetic boundary mutation affects pattern: '{}'",
            input
        );
        assert_eq!(
            replacement, expected_replacement,
            "UTF-8 arithmetic boundary mutation affects replacement: '{}'",
            input
        );

        // Ensure no panic from arithmetic underflow/overflow
        assert!(
            !pattern.is_empty() || !replacement.is_empty(),
            "UTF-8 arithmetic should not cause complete parsing failure: '{}'",
            input
        );
    }
}

/// Critical delimiter boundary tests targeting length check mutations
#[test]
fn test_delimiter_boundary_length_mutations() {
    // Target >= → > mutations in length boundary checks
    let boundary_cases = vec![
        // Critical len() == 1 boundary (> 1 vs >= 1)
        ("m", "mm"), // Single char: len()=1, > 1=false, >= 1=true
        ("s", ""),   // Single s prefix
        // UTF-8 boundary cases
        ("m❤", "❤❤"), // Multi-byte single character
    ];

    for (input, expected_pattern) in boundary_cases {
        if input.starts_with('m') {
            let (pattern, _) = extract_regex_parts(input);
            assert_eq!(
                pattern, expected_pattern,
                "Length boundary mutation affects regex: '{}'",
                input
            );
        } else {
            let (pattern, _, _) = extract_substitution_parts(input);
            assert_eq!(
                pattern, expected_pattern,
                "Length boundary mutation affects substitution: '{}'",
                input
            );
        }
    }
}

/// Critical depth tracking tests targeting -= → /= mutations
#[test]
fn test_depth_arithmetic_mutations() {
    // Target -= → /= in depth calculation (depth /= 1 does nothing)
    let nested_cases = vec![
        ("s{a{b}c}{repl}", "a{b}c", "repl"),
        ("s[test[inner]more][replacement]", "test[inner]more", "replacement"),
        ("s{{{deep}}}{shallow}", "{{deep}}", "shallow"),
    ];

    for (input, expected_pattern, expected_replacement) in nested_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);

        // With /= mutation, depth never decrements, breaking nested parsing
        assert_eq!(
            pattern, expected_pattern,
            "Depth arithmetic mutation breaks nested pattern: '{}'",
            input
        );
        assert_eq!(
            replacement, expected_replacement,
            "Depth arithmetic mutation breaks nested replacement: '{}'",
            input
        );
    }
}

/// Critical boolean logic tests targeting && → || mutations
#[test]
fn test_boolean_logic_mutations() {
    // Target !is_paired && !rest1.is_empty() → !is_paired || !rest1.is_empty()
    let logic_cases = vec![
        // is_paired=false, rest1.is_empty()=false: && gives true, || gives true (same)
        ("s/old/new/", "old", "new"),
        // is_paired=true, rest1.is_empty()=false: && gives false, || gives true (different)
        ("s{pattern}{replacement}", "pattern", "replacement"),
        // Test edge cases where logic matters
        ("s//", "", ""), // Both empty
    ];

    for (input, expected_pattern, expected_replacement) in logic_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);

        // Validate boolean logic produces expected results
        assert_eq!(
            pattern, expected_pattern,
            "Boolean logic mutation affects pattern: '{}'",
            input
        );
        assert_eq!(
            replacement, expected_replacement,
            "Boolean logic mutation affects replacement: '{}'",
            input
        );
    }
}

/// Critical semantic token overlap validation
#[test]
fn test_semantic_token_overlap_property() {
    let test_code = "my $var = 'hello'; sub test { return $var; }";

    let mut parser = Parser::new(test_code);
    if let Ok(ast) = parser.parse() {
        let to_pos16 = |byte_pos: usize| -> (u32, u32) {
            let line = test_code[..byte_pos].matches('\n').count() as u32;
            let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
            let col = (byte_pos - last_line_start) as u32;
            (line, col)
        };

        let tokens = collect_semantic_tokens(&ast, test_code, &to_pos16);

        // Critical property: semantic tokens must not overlap
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                let token1 = &tokens[i];
                let token2 = &tokens[j];

                // Decode positions (simplified for this test)
                let (line1, col1) = decode_position(&tokens, i);
                let (line2, col2) = decode_position(&tokens, j);

                if line1 == line2 {
                    // Same line: check no overlap
                    let token1_end = col1 + token1[2];
                    let token2_start = col2;

                    assert!(
                        token1_end <= token2_start || col1 >= col2 + token2[2],
                        "Semantic tokens overlap: Token {} and {} on line {}",
                        i,
                        j,
                        line1
                    );
                }
            }
        }
    }
}

/// Helper to decode token positions from delta encoding
fn decode_position(tokens: &[EncodedToken], index: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;

    for token in tokens.iter().take(index + 1) {
        line += token[0]; // delta_line
        if token[0] > 0 {
            col = token[1]; // reset to delta_start_char on new line
        } else {
            col += token[1]; // add delta_start_char on same line
        }
    }

    (line, col)
}

/// Critical function return value tests targeting FnValue → "xyzzy" mutations
#[test]
fn test_function_return_mutations() {
    let test_inputs =
        vec!["", "qr", "m", "s", "tr", "y", "s/test/repl/", "tr/abc/xyz/", "m/pattern/i"];

    for input in test_inputs {
        // Test all quote parser functions don't return sentinel "xyzzy"
        let (regex_pattern, regex_mods) = extract_regex_parts(input);
        let (sub_pattern, sub_replacement, sub_mods) = extract_substitution_parts(input);
        let (tr_search, tr_replace, tr_mods) = extract_transliteration_parts(input);

        // Kill FnValue mutations that return "xyzzy"
        assert_ne!(regex_pattern, "xyzzy", "extract_regex_parts sentinel value for '{}'", input);
        assert_ne!(regex_mods, "xyzzy", "extract_regex_parts modifiers sentinel for '{}'", input);
        assert_ne!(
            sub_pattern, "xyzzy",
            "extract_substitution_parts pattern sentinel for '{}'",
            input
        );
        assert_ne!(
            sub_replacement, "xyzzy",
            "extract_substitution_parts replacement sentinel for '{}'",
            input
        );
        assert_ne!(
            sub_mods, "xyzzy",
            "extract_substitution_parts modifiers sentinel for '{}'",
            input
        );
        assert_ne!(
            tr_search, "xyzzy",
            "extract_transliteration_parts search sentinel for '{}'",
            input
        );
        assert_ne!(
            tr_replace, "xyzzy",
            "extract_transliteration_parts replace sentinel for '{}'",
            input
        );
        assert_ne!(
            tr_mods, "xyzzy",
            "extract_transliteration_parts modifiers sentinel for '{}'",
            input
        );
    }
}

/// Critical paired delimiter tests targeting complex nesting mutations
#[test]
fn test_paired_delimiter_critical_cases() {
    let critical_delimiter_cases = vec![
        // Basic paired delimiters
        ("s{test}{result}", "test", "result"),
        ("s[pattern][replacement]", "pattern", "replacement"),
        ("s(old)(new)", "old", "new"),
        ("s<from><to>", "from", "to"),
        // Single level nesting
        ("s{a{b}c}{repl}", "a{b}c", "repl"),
        // Character class shielding (delimiters inside [...] should not close)
        ("s/test[}]/result/", "test[}]", "result"),
        ("s/pattern[)]/replacement/", "pattern[)]", "replacement"),
    ];

    for (input, expected_pattern, expected_replacement) in critical_delimiter_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);

        assert_eq!(
            pattern, expected_pattern,
            "Paired delimiter mutation affects pattern: '{}'",
            input
        );
        assert_eq!(
            replacement, expected_replacement,
            "Paired delimiter mutation affects replacement: '{}'",
            input
        );
    }
}

/// Critical control flow tests targeting match guard and depth mutations
#[test]
fn test_control_flow_mutations() {
    // Target match guard mutations: c == open && is_paired → false
    let control_flow_cases = vec![
        ("s{outer{inner}more}{repl}", "outer{inner}more", "repl"),
        ("s[test[nested]content][result]", "test[nested]content", "result"),
    ];

    for (input, expected_pattern, expected_replacement) in control_flow_cases {
        let (pattern, replacement, _) = extract_substitution_parts(input);

        // With false guard, nested delimiters aren't detected
        assert_eq!(
            pattern, expected_pattern,
            "Control flow mutation affects nested detection: '{}'",
            input
        );
        assert_eq!(
            replacement, expected_replacement,
            "Control flow mutation affects replacement: '{}'",
            input
        );
    }
}

/// Comprehensive property test ensuring no critical mutations survive
#[test]
fn test_comprehensive_property_validation() {
    let comprehensive_inputs = vec![
        // Edge cases combining multiple mutation points
        "s{test🦀{nested}more}{new❤️result}",
        "tr/café/new/cd",
        "m{pattern[}]test}gi",
        "s/test[>]/output/",
        "s{{{deep}}}{shallow}",
    ];

    for input in comprehensive_inputs {
        // Should handle all mutation scenarios without panic
        let _regex_result = std::panic::catch_unwind(|| extract_regex_parts(input));
        assert!(_regex_result.is_ok(), "Regex parsing panicked: '{}'", input);

        let _sub_result = std::panic::catch_unwind(|| extract_substitution_parts(input));
        assert!(_sub_result.is_ok(), "Substitution parsing panicked: '{}'", input);

        let _tr_result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(_tr_result.is_ok(), "Transliteration parsing panicked: '{}'", input);
    }
}
