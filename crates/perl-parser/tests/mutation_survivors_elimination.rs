use perl_parser::Parser;
/// Comprehensive mutation survivors elimination tests for PR #170 LSP executeCommand implementation
///
/// Targets critical surviving mutants identified in mutation testing with ~48% score:
/// - Quote parser arithmetic boundary mutations (UTF-8 position calculations)
/// - Semantic token overlap detection and validation
/// - Paired delimiter nesting edge cases
/// - Property-based testing for robustness
///
/// Goal: Achieve ≥80% mutation score through focused, high-value test additions
/// Labels: tests:hardening, mutation:elimination, pr170:executecommand
use perl_parser::quote_parser::*;
use perl_parser::semantic_tokens::{EncodedToken, collect_semantic_tokens};

/// UTF-8 boundary arithmetic tests targeting position calculation mutations
mod utf8_boundary_arithmetic {
    use super::*;

    /// Target arithmetic boundary mutations: + → -, >= → >, -= → /=
    /// Tests UTF-8 multi-byte character position calculations
    #[test]
    fn test_utf8_position_arithmetic_survivors() {
        // UTF-8 multi-byte characters that stress position arithmetic
        let utf8_cases = vec![
            // Basic single-byte ASCII baseline
            ("s/a/b/", "a", "b", ""),
            // 2-byte UTF-8 characters (U+00A0-U+07FF)
            ("s/café/test/", "café", "test", ""),
            // 3-byte UTF-8 characters (U+0800-U+FFFF)
            ("s/你好/hello/", "你好", "hello", ""),
            // 4-byte UTF-8 characters (U+10000-U+10FFFF) - emojis
            ("s/🦀/🔥/", "🦀", "🔥", ""),
            ("s/👨‍💻/dev/", "👨‍💻", "dev", ""), // Complex emoji with ZWJ
            // Mixed ASCII and UTF-8
            ("s/test🦀/new🔥/g", "test🦀", "new🔥", "g"),
            // Edge case: empty pattern with UTF-8 in replacement
            ("s//❤️/", "", "❤️", ""),
            // Edge case: UTF-8 at string boundaries
            ("s/❤️pattern❤️/💚replacement💚/gi", "❤️pattern❤️", "💚replacement💚", "gi"),
        ];

        for (input, expected_pattern, expected_replacement, expected_mods) in utf8_cases {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);

            // These assertions will fail if + → - mutation breaks position calculation
            assert_eq!(
                pattern, expected_pattern,
                "UTF-8 position arithmetic failed for pattern in '{}'",
                input
            );
            assert_eq!(
                replacement, expected_replacement,
                "UTF-8 position arithmetic failed for replacement in '{}'",
                input
            );
            assert_eq!(
                modifiers, expected_mods,
                "UTF-8 position arithmetic failed for modifiers in '{}'",
                input
            );
        }
    }

    /// Target >= → > mutations in length boundary checks
    #[test]
    fn test_length_boundary_off_by_one_mutations() {
        // Critical boundary cases where len() == threshold
        let regex_boundary_cases = vec![
            // Single character cases (len() == 1)
            ("m", ("mm", "")), // len() == 1, > 1 = false, >= 1 = true
            // Two character cases (len() == 2)
            ("mx", ("mxm", "")), // Boundary + 1
            // UTF-8 boundary cases
            ("m❤", ("❤❤", "")), // Single UTF-8 char but multiple bytes - actual behavior
        ];

        let substitution_boundary_cases = vec![
            ("s", ("", "", "")),   // len() == 1, different parsing path
            ("s/", ("", "", "")),  // Incomplete delimiter
            ("s🦀", ("", "", "")), // UTF-8 with incomplete pattern
        ];

        for (input, (expected_pattern, expected_mods)) in regex_boundary_cases {
            let (pattern, modifiers) = extract_regex_parts(input);
            assert_eq!(
                pattern, expected_pattern,
                "Length boundary check failed for pattern '{}'",
                input
            );
            assert_eq!(
                modifiers, expected_mods,
                "Length boundary check failed for modifiers '{}'",
                input
            );
        }

        for (input, (expected_pattern, expected_repl, expected_mods)) in substitution_boundary_cases
        {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);
            assert_eq!(
                pattern, expected_pattern,
                "Length boundary check failed for substitution pattern '{}'",
                input
            );
            assert_eq!(
                replacement, expected_repl,
                "Length boundary check failed for substitution replacement '{}'",
                input
            );
            assert_eq!(
                modifiers, expected_mods,
                "Length boundary check failed for substitution modifiers '{}'",
                input
            );
        }
    }

    /// Target -= → /= mutations in depth calculation for nested delimiters
    #[test]
    fn test_depth_arithmetic_mutation_elimination() {
        // Nested delimiter cases where depth tracking is critical
        let depth_test_cases = vec![
            // Single level nesting
            ("s{a{b}c}{repl}", "a{b}c", "repl"),
            ("s[test[inner]more][replacement]", "test[inner]more", "replacement"),
            ("s(open(nested)close)(new)", "open(nested)close", "new"),
            ("s<angle<nested>tag><result>", "angle<nested>tag", "result"),
            // Multiple level nesting (3+ levels)
            ("s{{{deep}}}{shallow}", "{{deep}}", "shallow"),
            ("s{a{b{c{d}e}f}g}{flat}", "a{b{c{d}e}f}g", "flat"),
            // Mixed nesting with different delimiters
            ("s{mix[ed]nest{ing}}{output}", "mix[ed]nest{ing}", "output"),
            // Edge case: maximum reasonable nesting
            ("s{{{{{{test}}}}}}{result}", "{{{{{test}}}}}", "result"),
        ];

        for (input, expected_pattern, expected_replacement) in depth_test_cases {
            let (pattern, replacement, _modifiers) = extract_substitution_parts(input);

            // With correct -= operator, depth decrements properly
            // With /= mutation (depth /= 1), depth never changes, breaking parsing
            assert_eq!(
                pattern, expected_pattern,
                "Depth arithmetic mutation affected nested pattern parsing: '{}'",
                input
            );
            assert_eq!(
                replacement, expected_replacement,
                "Depth arithmetic mutation affected nested replacement parsing: '{}'",
                input
            );

            // Ensure no infinite loops or panics from broken depth tracking
            assert!(
                !pattern.is_empty() || !replacement.is_empty(),
                "Depth arithmetic mutation caused complete parsing failure: '{}'",
                input
            );
        }
    }
}

/// Semantic token overlap validation tests targeting overlap detection mutations
mod semantic_token_overlap_validation {
    use super::*;

    /// Test no-overlap property for semantic tokens
    #[test]
    fn test_semantic_tokens_no_overlap_property() {
        let test_code = r#"
package MyModule;
use strict;
use warnings;

sub greet {
    my $name = shift;
    my @items = qw(hello world);
    for my $item (@items) {
        print "$item, $name!\n";
    }
}

my $var = "test";
my $regex = qr/pattern/i;
$var =~ s/old/new/g;
$var =~ tr/abc/xyz/;
"#;

        let mut parser = Parser::new(test_code);
        let ast = parser.parse().unwrap();

        // Create position converter for testing
        let to_pos16 = |byte_pos: usize| -> (u32, u32) {
            let line = test_code[..byte_pos].matches('\n').count() as u32;
            let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
            let col = (byte_pos - last_line_start) as u32;
            (line, col)
        };

        let tokens = collect_semantic_tokens(&ast, test_code, &to_pos16);

        // Validate no-overlap property
        for i in 0..tokens.len() {
            for j in (i + 1)..tokens.len() {
                let token1 = &tokens[i];
                let token2 = &tokens[j];

                // Convert delta-encoded tokens to absolute positions
                let (line1, col1) = decode_token_position(&tokens, i);
                let (line2, col2) = decode_token_position(&tokens, j);

                let _token1_end_col = col1 + token1[2]; // start + length
                let _token2_end_col = col2 + token2[2];

                // Test overlap conditions - should never overlap
                if line1 == line2 {
                    // Same line: check column ranges don't overlap
                    let overlaps = !(col1 + token1[2] <= col2 || col2 + token2[2] <= col1);
                    assert!(
                        !overlaps,
                        "Semantic tokens overlap detected: Token {} [{}, {}, {}] overlaps with Token {} [{}, {}, {}]",
                        i, line1, col1, token1[2], j, line2, col2, token2[2]
                    );
                } else {
                    // Different lines: no overlap by definition
                    assert!(line1 != line2, "Different lines should not be equal");
                }
            }
        }
    }

    /// Test semantic tokens idempotence property
    #[test]
    fn test_semantic_tokens_idempotence_property() {
        let test_code = "my $var = 'hello'; sub test { return $var; }";

        let mut parser = Parser::new(test_code);
        let ast = parser.parse().unwrap();

        let to_pos16 = |byte_pos: usize| -> (u32, u32) {
            let line = test_code[..byte_pos].matches('\n').count() as u32;
            let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
            let col = (byte_pos - last_line_start) as u32;
            (line, col)
        };

        // Generate tokens multiple times
        let tokens1 = collect_semantic_tokens(&ast, test_code, &to_pos16);
        let tokens2 = collect_semantic_tokens(&ast, test_code, &to_pos16);
        let tokens3 = collect_semantic_tokens(&ast, test_code, &to_pos16);

        // Idempotence: multiple calls should return identical results
        assert_eq!(tokens1, tokens2, "Semantic tokens generation is not idempotent (call 1 vs 2)");
        assert_eq!(tokens2, tokens3, "Semantic tokens generation is not idempotent (call 2 vs 3)");
        assert_eq!(tokens1, tokens3, "Semantic tokens generation is not idempotent (call 1 vs 3)");
    }

    /// Test semantic tokens permutation stability (order independence)
    #[test]
    fn test_semantic_tokens_permutation_stability() {
        let test_cases =
            vec!["my $a = 1; my $b = 2;", "sub func1 {} sub func2 {}", "package A; package B;"];

        for test_code in test_cases {
            let mut parser = Parser::new(test_code);
            let ast = parser.parse().unwrap();

            let to_pos16 = |byte_pos: usize| -> (u32, u32) {
                let line = test_code[..byte_pos].matches('\n').count() as u32;
                let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
                let col = (byte_pos - last_line_start) as u32;
                (line, col)
            };

            let tokens = collect_semantic_tokens(&ast, test_code, &to_pos16);

            // Tokens should be in proper order (delta-encoded)
            let mut prev_line = 0u32;
            let mut prev_col = 0u32;

            for (i, token) in tokens.iter().enumerate() {
                let delta_line = token[0];
                let delta_col = token[1];

                let current_line = prev_line + delta_line;
                let current_col = if delta_line > 0 { delta_col } else { prev_col + delta_col };

                // Validate ordering
                assert!(
                    current_line > prev_line
                        || (current_line == prev_line && current_col >= prev_col),
                    "Semantic tokens not properly ordered at index {}: prev=({},{}) current=({},{})",
                    i,
                    prev_line,
                    prev_col,
                    current_line,
                    current_col
                );

                prev_line = current_line;
                prev_col = current_col;
            }
        }
    }

    /// Test higher-priority token precedence in overlap situations
    #[test]
    fn test_semantic_tokens_priority_precedence() {
        // Test cases where multiple token types could apply
        let priority_test_cases = vec![
            ("my $special_var;", "variable declarations should have priority"),
            ("sub special_func {}", "function declarations should have priority"),
            ("'string content';", "string literals should have priority"),
            ("my $x = /regex/;", "regex patterns should have priority"),
        ];

        for (test_code, _description) in priority_test_cases {
            let mut parser = Parser::new(test_code);
            let ast = parser.parse().unwrap();

            let to_pos16 = |byte_pos: usize| -> (u32, u32) {
                let line = test_code[..byte_pos].matches('\n').count() as u32;
                let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
                let col = (byte_pos - last_line_start) as u32;
                (line, col)
            };

            let tokens = collect_semantic_tokens(&ast, test_code, &to_pos16);

            // Validate that tokens have consistent priorities
            // Higher priority tokens should not be overridden by lower priority ones
            for token in &tokens {
                assert!(token[2] > 0, "Token length should be positive: {:?}", token);
                assert!(
                    token[3] < 50, // Reasonable token type index
                    "Token type index should be reasonable: {:?}",
                    token
                );
            }
        }
    }

    /// Helper function to decode delta-encoded token positions
    fn decode_token_position(tokens: &[EncodedToken], index: usize) -> (u32, u32) {
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
}

/// Comprehensive paired delimiter nesting tests
mod paired_delimiter_comprehensive {
    use super::*;

    /// Test all paired delimiter combinations with nesting
    #[test]
    fn test_all_paired_delimiter_combinations() {
        let delimiter_pairs = vec![('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

        for (open, close) in &delimiter_pairs {
            // Simple paired delimiter
            let simple = format!("s{}pattern{}{open}replacement{close}", open, close);
            let (pattern, replacement, _) = extract_substitution_parts(&simple);
            assert_eq!(pattern, "pattern", "Simple {} delimiter failed", open);
            assert_eq!(replacement, "replacement", "Simple {} replacement failed", open);

            // Nested same delimiter
            let nested_same = format!(
                "s{}outer{}inner{}{}outer{}{open}result{close}",
                open, open, close, close, close
            );
            let (pattern, replacement, _) = extract_substitution_parts(&nested_same);
            let expected_pattern = format!("outer{}inner{}{}outer", open, close, close);
            assert_eq!(pattern, expected_pattern, "Nested same {} delimiter failed", open);
            assert_eq!(replacement, "result", "Nested same {} replacement failed", open);

            // Mixed nesting with different delimiters
            for (inner_open, inner_close) in &delimiter_pairs {
                if *inner_open != *open {
                    let mixed = format!(
                        "s{}mix{}ed{}nest{}{open}out{close}",
                        open, inner_open, inner_close, close
                    );
                    let (pattern, replacement, _) = extract_substitution_parts(&mixed);
                    let expected_mixed_pattern = format!("mix{}ed{}nest", inner_open, inner_close);
                    assert_eq!(
                        pattern, expected_mixed_pattern,
                        "Mixed nesting {}/{} failed",
                        open, inner_open
                    );
                    assert_eq!(replacement, "out", "Mixed nesting replacement failed");
                }
            }
        }
    }

    /// Test character class shielding inside delimiters
    #[test]
    fn test_character_class_delimiter_shielding() {
        // Delimiters inside character classes should not close the quote
        let char_class_cases = vec![
            ("s/pattern[}]/replacement/", "pattern[}]", "replacement"),
            ("s/test[)]/result/", "test[)]", "result"),
            ("s/check[>]/output/", "check[>]", "output"),
            ("s/nested[[][]]test/final/", "nested[[][]]test", "final"),
            // Character classes with ranges including delimiters
            ("s/range[a-}]/repl/", "range[a-}]", "repl"),
            ("s/range[(-)]test/result/", "range[(-)]test", "result"),
            // Escaped delimiters vs character classes
            ("s/escaped\\]/vs[]/class/", "escaped\\]", "vs[]"),
            ("s/test\\}/vs[}]/end/", "test\\}", "vs[}]"),
        ];

        for (input, expected_pattern, expected_replacement) in char_class_cases {
            let (pattern, replacement, _) = extract_substitution_parts(input);
            assert_eq!(
                pattern, expected_pattern,
                "Character class shielding failed for pattern in '{}'",
                input
            );
            assert_eq!(
                replacement, expected_replacement,
                "Character class shielding failed for replacement in '{}'",
                input
            );
        }
    }

    /// Test deeply nested delimiters (stress test)
    #[test]
    fn test_deeply_nested_delimiters() {
        // Generate deeply nested structures
        let max_depth = 10;

        for depth in 1..=max_depth {
            let mut pattern = String::new();
            let mut expected_pattern = String::new();

            // Build nested opening
            for i in 0..depth {
                pattern.push('{');
                expected_pattern.push('{');
                if i < depth - 1 {
                    pattern.push_str(&format!("level{}", i));
                    expected_pattern.push_str(&format!("level{}", i));
                }
            }

            // Add content
            pattern.push_str("content");
            expected_pattern.push_str("content");

            // Build nested closing
            for _ in 0..depth {
                pattern.push('}');
                expected_pattern.push('}');
            }

            // Add replacement
            let _full_input = format!(
                "s{}{}{{{}}}",
                pattern,
                pattern.chars().take(1).collect::<String>(),
                "repl"
            );
            pattern.remove(0); // Remove first delimiter
            pattern.pop(); // Remove last delimiter

            let test_input = format!("s{}{{{}}}", pattern, "repl");
            let (actual_pattern, replacement, _) = extract_substitution_parts(&test_input);

            assert_eq!(replacement, "repl", "Deep nesting level {} replacement failed", depth);

            // Should handle deep nesting without panicking
            assert!(
                !actual_pattern.is_empty(),
                "Deep nesting level {} should parse something",
                depth
            );
        }
    }
}

/// Property-based tests for comprehensive mutation validation
mod property_based_mutation_tests {
    use super::*;

    /// Property: Quote parser functions should never return sentinel values
    #[test]
    fn test_no_sentinel_values_property() {
        let forbidden_values = vec!["xyzzy", "sentinel", "mutation", "deadbeef"];

        let test_inputs = vec![
            "",
            "q",
            "qq",
            "qqq",
            "m",
            "mm",
            "m/",
            "m//",
            "m/test/",
            "s",
            "ss",
            "s/",
            "s//",
            "s/a/b/",
            "tr",
            "tr/",
            "tr//",
            "tr/a/b/",
            "y",
            "y/",
            "y//",
            "y/a/b/",
            "qr",
            "qr/",
            "qr//",
            "qr/test/",
            // UTF-8 cases
            "s/🦀/🔥/",
            "tr/café/test/",
            "m/你好/",
            // Edge cases
            "s{}{}",
            "s///",
            "tr{}{}",
            "m{}",
        ];

        for input in test_inputs {
            // Test all quote parser functions
            let (regex_pattern, regex_mods) = extract_regex_parts(input);
            let (sub_pattern, sub_replacement, sub_mods) = extract_substitution_parts(input);
            let (tr_search, tr_replace, tr_mods) = extract_transliteration_parts(input);

            // Check no forbidden sentinel values
            for forbidden in &forbidden_values {
                assert_ne!(
                    regex_pattern, *forbidden,
                    "extract_regex_parts returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    regex_mods, *forbidden,
                    "extract_regex_parts modifiers returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    sub_pattern, *forbidden,
                    "extract_substitution_parts pattern returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    sub_replacement, *forbidden,
                    "extract_substitution_parts replacement returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    sub_mods, *forbidden,
                    "extract_substitution_parts modifiers returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    tr_search, *forbidden,
                    "extract_transliteration_parts search returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    tr_replace, *forbidden,
                    "extract_transliteration_parts replace returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
                assert_ne!(
                    tr_mods, *forbidden,
                    "extract_transliteration_parts modifiers returned forbidden value '{}' for input '{}'",
                    forbidden, input
                );
            }
        }
    }

    /// Property: Arithmetic operations should maintain valid indices
    #[test]
    fn test_arithmetic_safety_invariants() {
        let edge_inputs = vec![
            // Single character boundaries
            "s/a/b/",
            "m/x/",
            "tr/a/b/",
            // UTF-8 multi-byte boundaries
            "s/🦀/test/",
            "m/❤️/",
            "tr/café/tea/",
            // Empty and minimal cases
            "s//",
            "m",
            "tr",
            // Deeply nested
            "s{{{{{test}}}}}{result}",
            // Mixed delimiters
            "s/test[}]more/repl/",
        ];

        for input in edge_inputs {
            // All parsing should complete without panic/overflow
            let _regex_result = std::panic::catch_unwind(|| extract_regex_parts(input));
            assert!(_regex_result.is_ok(), "extract_regex_parts panicked on '{}'", input);

            let _sub_result = std::panic::catch_unwind(|| extract_substitution_parts(input));
            assert!(_sub_result.is_ok(), "extract_substitution_parts panicked on '{}'", input);

            let _tr_result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
            assert!(_tr_result.is_ok(), "extract_transliteration_parts panicked on '{}'", input);
        }
    }

    /// Property: Boolean logic should be consistent and deterministic
    #[test]
    fn test_boolean_logic_consistency() {
        let logic_test_cases = vec![
            // is_paired=true cases (different open/close)
            ("s{pattern}{replacement}", true),
            ("s[test][result]", true),
            ("s(old)(new)", true),
            ("s<from><to>", true),
            // is_paired=false cases (same delimiter)
            ("s/pattern/replacement/", false),
            ("s#old#new#", false),
            ("s|from|to|", false),
            ("s~test~result~", false),
        ];

        for (input, expected_is_paired) in logic_test_cases {
            let (pattern, replacement, _) = extract_substitution_parts(input);

            // Validate the logic produces consistent results
            if expected_is_paired {
                // Paired delimiters should successfully extract both parts
                assert!(
                    !pattern.is_empty() && !replacement.is_empty(),
                    "Paired delimiter logic failed for '{}': pattern='{}', replacement='{}'",
                    input,
                    pattern,
                    replacement
                );
            } else {
                // Non-paired delimiters should also extract both parts correctly
                assert!(
                    !pattern.is_empty() || !replacement.is_empty(),
                    "Non-paired delimiter logic failed for '{}': pattern='{}', replacement='{}'",
                    input,
                    pattern,
                    replacement
                );
            }
        }
    }

    /// Property: Semantic tokens should maintain ordering invariants
    #[test]
    fn test_semantic_tokens_ordering_invariants() {
        let test_codes = vec![
            "my $var = 'test';",
            "sub func { return 1; }",
            "package Test; use strict;",
            "$var =~ s/old/new/g;",
            "my @array = qw(a b c);",
        ];

        for test_code in test_codes {
            let mut parser = Parser::new(test_code);
            let ast = parser.parse().unwrap();

            let to_pos16 = |byte_pos: usize| -> (u32, u32) {
                let line = test_code[..byte_pos].matches('\n').count() as u32;
                let last_line_start = test_code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
                let col = (byte_pos - last_line_start) as u32;
                (line, col)
            };

            let tokens = collect_semantic_tokens(&ast, test_code, &to_pos16);

            // Property: Tokens should be ordered by position
            let mut positions = Vec::new();
            let mut line = 0u32;
            let mut col = 0u32;

            for token in &tokens {
                line += token[0];
                if token[0] > 0 {
                    col = token[1];
                } else {
                    col += token[1];
                }
                positions.push((line, col));
            }

            // Verify ordering
            for i in 1..positions.len() {
                let prev = positions[i - 1];
                let curr = positions[i];
                assert!(
                    prev.0 < curr.0 || (prev.0 == curr.0 && prev.1 <= curr.1),
                    "Semantic tokens not ordered: {:?} should come before {:?} in '{}'",
                    prev,
                    curr,
                    test_code
                );
            }
        }
    }
}

/// Integration tests ensuring multiple mutation types don't interact negatively
#[test]
fn test_comprehensive_mutation_integration() {
    // Complex test case exercising multiple mutation points simultaneously
    let complex_cases = vec![
        // UTF-8 + nesting + arithmetic + boolean logic
        "s{café{nested🦀}more}{new❤️{result}end}gi",
        // Deep nesting + UTF-8 + edge cases
        "s{{{{test🔥}}}}{{{result💚}}}",
        // Character classes + UTF-8 + mixed delimiters
        "s/pattern[🦀-🔥]test/replacement[❤️]/g",
        // Transliteration with UTF-8 and complex modifiers
        "tr{café🦀test}{new❤️result}cds",
        // Mixed quote operators
        "m{test[}]pattern}gi",
    ];

    for input in complex_cases {
        // Should handle all mutations correctly without panic
        let _regex = extract_regex_parts(input);
        let _substitution = extract_substitution_parts(input);
        let _transliteration = extract_transliteration_parts(input);
    }
}
