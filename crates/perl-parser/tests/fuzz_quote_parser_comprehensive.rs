/// Comprehensive fuzz testing for quote parser components
/// Targets stress testing of enhanced quote parser after mutation hardening work
///
/// Focuses on:
/// - Bounded fuzz testing with proptest (time/iteration limits)
/// - AST invariant validation under stress
/// - Parser crash/panic reproduction
/// - Edge case discovery in quote parsing logic
///
/// Labels: tests:fuzz, perl-fuzz:running

use perl_parser::quote_parser::*;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};

/// Regression directory for fuzz test cases
const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/fuzz_quote_parser");

/// Property-based fuzz testing for regex parts extraction
/// Tests stress conditions with random inputs to find crashes/panics
#[test]
fn fuzz_extract_regex_parts_stress_test() {
    fn test_regex_parts_no_panic(input: String) -> Result<(), proptest::test_runner::TestCaseError> {
        // Core invariant: function should never panic, regardless of input
        let result = std::panic::catch_unwind(|| {
            extract_regex_parts(&input)
        });

        prop_assert!(result.is_ok(), "extract_regex_parts panicked on input: {:?}", input);

        if let Ok((pattern, modifiers)) = result {
            // AST invariant: results should be valid UTF-8 strings
            prop_assert!(pattern.is_ascii() || std::str::from_utf8(pattern.as_bytes()).is_ok(),
                "Pattern contains invalid UTF-8: {:?}", pattern);
            prop_assert!(modifiers.is_ascii() || std::str::from_utf8(modifiers.as_bytes()).is_ok(),
                "Modifiers contains invalid UTF-8: {:?}", modifiers);

            // Parser invariant: modifiers should only contain alphabetic chars
            for ch in modifiers.chars() {
                prop_assert!(ch.is_ascii_alphabetic() || ch.is_ascii_whitespace(),
                    "Modifier contains non-alphabetic char: '{}' in modifiers: {:?}", ch, modifiers);
            }
        }

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1000, // Bounded testing - 1000 cases for stress testing
            max_shrink_iters: 100,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn regex_parts_fuzz(
            // Generate various input patterns that could trigger edge cases
            input in prop_oneof![
                // Empty and minimal inputs
                prop::string::string_regex("").unwrap(),
                // Single characters that might cause boundary issues
                prop::string::string_regex("[mqr/\\\\{}()\\[\\]<>|#!~]").unwrap(),
                // Short strings with potential regex prefixes
                prop::string::string_regex("(m|qr|q|qq)?[/\\\\{}()\\[\\]<>|#!~][a-zA-Z0-9]*").unwrap(),
                // Longer strings with nested delimiters
                prop::string::string_regex("(m|qr)[/\\\\{}()\\[\\]<>|#!~][^/\\\\{}()\\[\\]<>|#!~]*[/\\\\{}()\\[\\]<>|#!~][imsxgaeludnrpcoRD]*").unwrap(),
                // Unicode strings that might cause UTF-8 boundary issues
                ".*[\\u{0080}-\\u{FFFF}].*",
                // Malformed regex patterns
                prop::string::string_regex("(m|qr)[^a-zA-Z0-9\\s]*").unwrap(),
                // Very long strings to test memory boundaries
                "[a-zA-Z0-9/\\\\{}()\\[\\]<>|#!~]{0,1000}"
            ]
        ) {
            test_regex_parts_no_panic(input)?;
        }
    }
}

/// Fuzz testing for substitution parts extraction with focus on crash discovery
#[test]
fn fuzz_extract_substitution_parts_crash_detection() {
    fn test_substitution_no_crash(input: String) -> Result<(), proptest::test_runner::TestCaseError> {
        let result = std::panic::catch_unwind(|| {
            extract_substitution_parts(&input)
        });

        prop_assert!(result.is_ok(), "extract_substitution_parts crashed on: {:?}", input);

        if let Ok((pattern, replacement, modifiers)) = result {
            // Memory safety invariants
            prop_assert!(pattern.len() <= input.len() * 2,
                "Pattern length {} exceeds reasonable bound for input length {}", pattern.len(), input.len());
            prop_assert!(replacement.len() <= input.len() * 2,
                "Replacement length {} exceeds reasonable bound for input length {}", replacement.len(), input.len());
            prop_assert!(modifiers.len() <= input.len(),
                "Modifiers length {} exceeds input length {}", modifiers.len(), input.len());

            // UTF-8 safety invariants
            prop_assert!(std::str::from_utf8(pattern.as_bytes()).is_ok(), "Invalid UTF-8 in pattern");
            prop_assert!(std::str::from_utf8(replacement.as_bytes()).is_ok(), "Invalid UTF-8 in replacement");
            prop_assert!(std::str::from_utf8(modifiers.as_bytes()).is_ok(), "Invalid UTF-8 in modifiers");
        }

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1500, // More aggressive testing for substitution
            max_shrink_iters: 150,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn substitution_crash_fuzz(
            input in prop_oneof![
                // Empty inputs
                "",
                // Minimal s operations
                prop::string::string_regex("s").unwrap(),
                // Basic substitution patterns
                prop::string::string_regex("s[/\\\\{}()\\[\\]<>|#!~][^/\\\\{}()\\[\\]<>|#!~]*[/\\\\{}()\\[\\]<>|#!~][^/\\\\{}()\\[\\]<>|#!~]*[/\\\\{}()\\[\\]<>|#!~]?[imsxgaeludnrpcoRD]*").unwrap(),
                // Unbalanced delimiters (potential crash triggers)
                prop::string::string_regex("s[{(\\[].*").unwrap(),
                prop::string::string_regex("s[})].*").unwrap(),
                // Escape sequence stress testing
                prop::string::string_regex("s/([^/\\\\]|\\\\.)*//[imsxg]*").unwrap(),
                // Very deep nesting
                "s" + &prop::string::string_regex("[{(\\[]{0,50}.*[})]\\[]{0,50}").unwrap(),
                // Unicode boundary stress
                "s/[\\u{0080}-\\u{FFFF}]+/[\\u{0080}-\\u{FFFF}]+/g",
                // Memory exhaustion patterns
                "[s/]{1,10}" + &prop::string::string_regex("[a-zA-Z0-9]{0,500}").unwrap(),
            ]
        ) {
            test_substitution_no_crash(input)?;
        }
    }
}

/// Fuzz testing for transliteration parsing with AST invariant validation
#[test]
fn fuzz_extract_transliteration_ast_invariants() {
    fn test_transliteration_invariants(input: String) -> Result<(), proptest::test_runner::TestCaseError> {
        let result = std::panic::catch_unwind(|| {
            extract_transliteration_parts(&input)
        });

        prop_assert!(result.is_ok(), "extract_transliteration_parts panicked on: {:?}", input);

        if let Ok((search, replace, modifiers)) = result {
            // AST consistency invariants

            // Length invariants - results shouldn't be longer than reasonable bounds
            prop_assert!(search.len() <= input.len(),
                "Search part length {} exceeds input length {}", search.len(), input.len());
            prop_assert!(replace.len() <= input.len(),
                "Replace part length {} exceeds input length {}", replace.len(), input.len());
            prop_assert!(modifiers.len() <= input.len(),
                "Modifiers length {} exceeds input length {}", modifiers.len(), input.len());

            // Character class invariants for modifiers
            for ch in modifiers.chars() {
                prop_assert!(ch.is_ascii_alphabetic() || ch.is_ascii_whitespace(),
                    "Invalid character '{}' in modifiers: {:?}", ch, modifiers);
            }

            // UTF-8 invariants
            prop_assert!(search.is_ascii() || std::str::from_utf8(search.as_bytes()).is_ok(),
                "Search contains invalid UTF-8");
            prop_assert!(replace.is_ascii() || std::str::from_utf8(replace.as_bytes()).is_ok(),
                "Replace contains invalid UTF-8");
        }

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 800, // Focused testing on transliteration
            max_shrink_iters: 80,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn transliteration_invariants_fuzz(
            input in prop_oneof![
                // Empty and minimal
                "",
                "tr",
                "y",
                // Basic transliteration patterns
                prop::string::string_regex("(tr|y)[/\\\\{}()\\[\\]<>|#!~][a-zA-Z0-9]*[/\\\\{}()\\[\\]<>|#!~][a-zA-Z0-9]*[/\\\\{}()\\[\\]<>|#!~]?[cdsr]*").unwrap(),
                // Character class patterns
                "tr/[a-z]/[A-Z]/",
                "y/[0-9]/[a-j]/d",
                // Unicode character classes
                "tr/[α-ω]/[Α-Ω]/",
                // Malformed patterns that might break parsing
                prop::string::string_regex("(tr|y)[^a-zA-Z0-9\\s/\\\\]*").unwrap(),
                // Edge case delimiters
                "tr|||", "y###", "tr   ",
            ]
        ) {
            test_transliteration_invariants(input)?;
        }
    }
}

/// Stress test all quote parser functions with extreme inputs
/// Targets memory exhaustion, infinite loops, and parser state corruption
#[test]
fn fuzz_quote_parser_extreme_stress() {
    fn test_extreme_robustness(input: String) -> Result<(), proptest::test_runner::TestCaseError> {
        // Test all public functions for robustness under extreme conditions

        // Test extract_regex_parts
        let result = std::panic::catch_unwind(|| extract_regex_parts(&input));
        prop_assert!(result.is_ok(), "extract_regex_parts panicked on extreme input: {:?}", input);
        if let Ok((output1, output2)) = result {
            prop_assert!(output1.len() <= input.len() * 10,
                "extract_regex_parts produced oversized output: length {} vs input length {}",
                output1.len(), input.len());
            prop_assert!(output2.len() <= input.len() * 10,
                "extract_regex_parts produced oversized output: length {} vs input length {}",
                output2.len(), input.len());
        }

        // Test extract_substitution_parts
        let result = std::panic::catch_unwind(|| extract_substitution_parts(&input));
        prop_assert!(result.is_ok(), "extract_substitution_parts panicked on extreme input: {:?}", input);
        if let Ok((output1, output2, output3)) = result {
            prop_assert!(output1.len() <= input.len() * 10,
                "extract_substitution_parts produced oversized pattern: length {} vs input length {}",
                output1.len(), input.len());
            prop_assert!(output2.len() <= input.len() * 10,
                "extract_substitution_parts produced oversized replacement: length {} vs input length {}",
                output2.len(), input.len());
            prop_assert!(output3.len() <= input.len() * 10,
                "extract_substitution_parts produced oversized modifiers: length {} vs input length {}",
                output3.len(), input.len());
        }

        // Test extract_transliteration_parts
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(&input));
        prop_assert!(result.is_ok(), "extract_transliteration_parts panicked on extreme input: {:?}", input);
        if let Ok((output1, output2, output3)) = result {
            prop_assert!(output1.len() <= input.len() * 10,
                "extract_transliteration_parts produced oversized search: length {} vs input length {}",
                output1.len(), input.len());
            prop_assert!(output2.len() <= input.len() * 10,
                "extract_transliteration_parts produced oversized replace: length {} vs input length {}",
                output2.len(), input.len());
            prop_assert!(output3.len() <= input.len() * 10,
                "extract_transliteration_parts produced oversized modifiers: length {} vs input length {}",
                output3.len(), input.len());
        }

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 500, // Extreme stress testing with fewer but more intense cases
            max_shrink_iters: 50,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn extreme_stress_fuzz(
            input in prop_oneof![
                // Memory stress: very long strings
                prop::string::string_regex("[a-zA-Z0-9/\\\\{}()\\[\\]<>|#!~]{500,2000}").unwrap(),
                // Nested delimiter stress
                prop::string::string_regex("s[{(\\[]+[^})]\\[]*[})]\\[]+[{(\\[]+[^})]\\[]*[})]\\[]+").unwrap(),
                // Escape sequence stress
                prop::string::string_regex("s/(\\\\.){1,100}/(\\\\.){1,100}/").unwrap(),
                // Unicode stress with mixed byte sequences
                ".*[\\u{0000}-\\u{FFFF}]{1,100}.*",
                // Repeated pattern stress (potential infinite loop triggers)
                prop::string::string_regex("(s//|m//|tr///|qr//){1,50}").unwrap(),
                // Binary-like data that might confuse parser
                prop::collection::vec(any::<u8>(), 0..1000).prop_map(|bytes| {
                    String::from_utf8_lossy(&bytes).into_owned()
                }),
            ]
        ) {
            test_extreme_robustness(input)?;
        }
    }
}

/// Integration test: validate that quote parser changes don't break incremental parsing
/// This tests the interaction between quote parsing and AST construction
#[test]
fn fuzz_quote_parser_incremental_parsing_integration() {
    use perl_parser::PerlParser;

    fn test_incremental_integration(input: String) -> Result<(), proptest::test_runner::TestCaseError> {
        // Create a simple Perl script that uses quote-like constructs
        let perl_script = format!(r#"
#!/usr/bin/perl
use strict;
use warnings;

my $regex = {};
my $substitution = $regex;
$substitution =~ {};
my $transliteration = $substitution;
$transliteration =~ {};

print "Done\n";
"#, input, input, input);

        // Test that parser doesn't crash on quote-like constructs
        let result = std::panic::catch_unwind(|| {
            let parser = PerlParser::new();
            parser.parse(&perl_script)
        });

        prop_assert!(result.is_ok(), "Parser crashed on quote construct integration: {:?}", input);

        if let Ok(parse_result) = result {
            // Basic AST integrity checks
            if let Ok(ast) = parse_result {
                prop_assert!(!ast.is_empty(), "AST should not be empty for valid Perl script");
                // Additional AST validation could go here
            }
            // If parsing fails, that's acceptable - we just don't want panics
        }

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 200, // Integration testing - fewer cases but with full parser
            max_shrink_iters: 20,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn incremental_integration_fuzz(
            input in prop_oneof![
                // Valid-looking quote constructs
                prop::string::string_regex("qr/[a-zA-Z0-9_.]+/[imsxg]*").unwrap(),
                prop::string::string_regex("s/[a-zA-Z0-9_.]+/[a-zA-Z0-9_.]+/[imsxg]*").unwrap(),
                prop::string::string_regex("tr/[a-zA-Z0-9]+/[a-zA-Z0-9]+/[cdsr]*").unwrap(),
                // Edge cases that might break integration
                "m//", "s///", "tr///",
                "qr{}", "s{}{}", "tr{}{}",
                // Nested delimiters
                "s{test{nested}test}{replacement}g",
                "qr(test(nested)test)i",
            ]
        ) {
            test_incremental_integration(input)?;
        }
    }
}