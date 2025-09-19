use perl_parser::quote_parser::extract_substitution_parts;
use perl_parser::Parser;
use std::panic;

/// Bounded fuzz testing for substitution operator parsing without requiring nightly Rust
/// This implements property-based testing to identify crash conditions and edge cases
#[test]
fn test_substitution_parsing_fuzz() {
    let fuzz_inputs = generate_fuzz_inputs();
    let mut crashers = Vec::new();
    let mut tested_count = 0;

    for input in fuzz_inputs {
        tested_count += 1;

        // Test direct substitution parsing
        let result = panic::catch_unwind(|| {
            if input.starts_with('s') {
                let (pattern, replacement, modifiers) = extract_substitution_parts(&input);

                // Basic invariant checks
                assert!(pattern.len() <= input.len());
                assert!(replacement.len() <= input.len());
                assert!(modifiers.len() <= input.len());

                // Verify modifiers only contain valid characters
                for ch in modifiers.chars() {
                    assert!(matches!(ch, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'));
                }
            }
        });

        if result.is_err() {
            crashers.push(format!("extract_substitution_parts crasher: {}", input));
        }

        // Test full parser
        let parser_result = panic::catch_unwind(|| {
            let mut parser = Parser::new(&input);
            let _result = parser.parse();
        });

        if parser_result.is_err() {
            crashers.push(format!("Parser crasher: {}", input));
        }
    }

    if !crashers.is_empty() {
        eprintln!("Found {} crashers out of {} inputs:", crashers.len(), tested_count);
        for crasher in &crashers {
            eprintln!("  {}", crasher);
        }
        panic!("Fuzz testing found crashers - see output above");
    }

    println!("Fuzz testing completed: {} inputs tested, no crashers found", tested_count);
}

/// Generate edge-case inputs targeting substitution operator parsing vulnerabilities
fn generate_fuzz_inputs() -> Vec<String> {
    let mut inputs = Vec::new();

    // Basic substitution forms
    inputs.extend(vec![
        "s///".to_string(),
        "s/a/b/".to_string(),
        "s{}{}".to_string(),
        "s[][]".to_string(),
        "s<><>".to_string(),
        "s()()".to_string(),
    ]);

    // Edge case delimiters
    inputs.extend(vec![
        "s'pattern'replacement'".to_string(),
        "s#pattern#replacement#".to_string(),
        "s|pattern|replacement|".to_string(),
        "s!pattern!replacement!".to_string(),
        "s@pattern@replacement@".to_string(),
    ]);

    // Malformed inputs that could cause crashes
    inputs.extend(vec![
        "s".to_string(),
        "s/".to_string(),
        "s//".to_string(),
        "s/pattern".to_string(),
        "s/pattern/".to_string(),
        "s{".to_string(),
        "s{pattern".to_string(),
        "s{pattern}".to_string(),
        "s{pattern}{".to_string(),
    ]);

    // Unicode edge cases
    inputs.extend(vec![
        "s/café/tea/".to_string(),
        "s{🦀}{⚡}".to_string(),
        "s/\u{0000}/test/".to_string(),
        "s/\u{FFFF}/test/".to_string(),
    ]);

    // Nested delimiters and escaping
    inputs.extend(vec![
        "s{{}}{replacement}".to_string(),
        "s{pattern{nested}}{replacement}".to_string(),
        "s/pattern\\/with\\/slashes/replacement/".to_string(),
        "s{pattern\\}with\\}braces}{replacement}".to_string(),
    ]);

    // Empty components
    inputs.extend(vec![
        "s///g".to_string(),
        "s{}{}g".to_string(),
        "s/pattern//".to_string(),
        "s//replacement/".to_string(),
    ]);

    // Invalid modifiers
    inputs.extend(vec![
        "s/pattern/replacement/xyz".to_string(),
        "s/pattern/replacement/123".to_string(),
        "s/pattern/replacement/g1i2m3".to_string(),
    ]);

    // Memory exhaustion potential
    inputs.extend(vec![
        format!("s/{}/replacement/", "a".repeat(1000)),
        format!("s/pattern/{}/", "b".repeat(1000)),
        format!("s/pattern/replacement/{}", "gimsx".repeat(100)),
    ]);

    // Deeply nested structures
    let mut nested = "s{".to_string();
    for _ in 0..100 {
        nested.push_str("a{");
    }
    for _ in 0..100 {
        nested.push('}');
    }
    nested.push_str("}{replacement}");
    inputs.push(nested);

    // Pathological cases
    inputs.extend(vec![
        "s/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\//replacement/".to_string(),
        "s{{{{{{{{{{{{{{{{{{{{{{}}}}}}}}}}}}}}}}}}}}}}}}{replacement}".to_string(),
        "s/pattern/\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\/".to_string(),
    ]);

    inputs
}

#[test]
fn test_minimal_repro_cases() {
    // These are minimal cases that should work correctly
    let safe_cases = vec![
        ("s/a/b/", ("a", "b", "")),
        ("s{pattern}{replacement}", ("pattern", "replacement", "")),
        ("s///g", ("", "", "g")),
        ("s/test/result/gi", ("test", "result", "gi")),
    ];

    for (input, expected) in safe_cases {
        let (pattern, replacement, modifiers) = extract_substitution_parts(input);
        assert_eq!((pattern.as_str(), replacement.as_str(), modifiers.as_str()), expected,
                   "Failed for input: {}", input);
    }
}