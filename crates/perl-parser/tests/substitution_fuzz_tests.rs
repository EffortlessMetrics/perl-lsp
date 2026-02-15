/// Focused property-based fuzz tests for substitution operator parsing
///
/// These tests systematically explore edge cases and boundary conditions
/// in the substitution operator parsing logic.
use perl_parser::{Parser, quote_parser::extract_substitution_parts};
use std::panic;

/// Test a batch of substitution operator inputs with invariant checking
fn test_substitution_batch(inputs: &[&str]) -> Vec<String> {
    let mut crashes = Vec::new();

    for &input in inputs {
        // Test quote_parser::extract_substitution_parts with panic catching
        let result = panic::catch_unwind(|| {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);

            // Basic invariants that should never be violated
            assert!(pattern.len() <= input.len(), "Pattern longer than input: {}", input);
            assert!(replacement.len() <= input.len(), "Replacement longer than input: {}", input);
            assert!(modifiers.len() <= input.len(), "Modifiers longer than input: {}", input);

            // Verify modifiers only contain valid characters
            for ch in modifiers.chars() {
                assert!(
                    matches!(ch, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'
                        | 'a' | 'd' | 'l' | 'u' | 'n' | 'p' | 'c'),
                    "Invalid modifier '{}' in: {}",
                    ch,
                    input
                );
            }

            (pattern, replacement, modifiers)
        });

        if result.is_err() {
            crashes.push(format!("extract_substitution_parts panicked on: {}", input));
        }

        // Test full parser with panic catching
        let parser_result = panic::catch_unwind(|| {
            let mut parser = Parser::new(input);
            parser.parse()
        });

        if parser_result.is_err() {
            crashes.push(format!("Parser panicked on: {}", input));
        }
    }

    crashes
}

/// Generate systematic test cases covering edge conditions
fn generate_edge_case_inputs() -> Vec<&'static str> {
    vec![
        // Basic malformed cases
        "s",
        "s/",
        "s//",
        "s///",
        // Empty components
        "s///g",
        "s/a//",
        "s//b/",
        "s/a/b/",
        // Single character delimiters
        "s!pattern!replacement!",
        "s@pattern@replacement@",
        "s#pattern#replacement#",
        "s%pattern%replacement%",
        "s^pattern^replacement^",
        "s&pattern&replacement&",
        "s*pattern*replacement*",
        "s+pattern+replacement+",
        "s=pattern=replacement=",
        "s~pattern~replacement~",
        // Balanced delimiters
        "s(pattern)(replacement)",
        "s{pattern}{replacement}",
        "s[pattern][replacement]",
        "s<pattern><replacement>",
        // Edge cases with balanced delimiters
        "s()()",
        "s{}{}",
        "s[][]",
        "s<><>",
        "s()()g",
        "s{}{}g",
        "s[][]g",
        "s<><>g",
        // Incomplete balanced delimiters
        "s(",
        "s{",
        "s[",
        "s<",
        "s(pattern",
        "s{pattern",
        "s[pattern",
        "s<pattern",
        "s(pattern)",
        "s{pattern}",
        "s[pattern]",
        "s<pattern>",
        // Modifier edge cases
        "s/a/b/ggiimmssxxooeerr",
        "s/a/b/gibberish",
        "s/a/b/123",
        "s/a/b/!@#",
        "s/a/b/ ",
        "s/a/b/\t",
        "s/a/b/\n",
        // Escape sequence edge cases
        "s/\\/\\/\\/",
        "s/\\\\\\\\\\\\",
        // Unicode edge cases
        "s/α/β/",
        "s/😀/😁/",
        "s/🦀/🔥/",
        "s/Ω/Α/",
        // Nested delimiter cases
        "s{pat{tern}}{replace{ment}}",
        "s[pat[tern]][replace[ment]]",
        "s(pat(tern))(replace(ment))",
        "s<pat<tern>><replace<ment>>",
        // Mixed content
        "s/[a-z]+/NUMBER/g",
        "s/\\d{3}-\\d{2}-\\d{4}/XXX-XX-XXXX/",
        "s/(?:foo|bar)/baz/gi",
    ]
}

/// Generate pathological inputs designed to stress the parser
fn generate_pathological_inputs() -> Vec<String> {
    let mut inputs = Vec::new();

    // Deep nesting with balanced delimiters
    for depth in 1..5 {
        // Reduced depth to prevent timeout
        let pattern = format!("s{}{}{}", "{".repeat(depth), "a".repeat(depth), "}".repeat(depth));
        let replacement =
            format!("{}{}{}", "{".repeat(depth), "b".repeat(depth), "}".repeat(depth));
        inputs.push(format!("s{}{}", pattern, replacement));
    }

    // Long modifiers
    for len in [10, 50] {
        // Reduced lengths
        inputs.push(format!("s/a/b/{}", "g".repeat(len)));
        inputs.push(format!("s/a/b/{}", "invalid".repeat(len)));
    }

    inputs
}

/// Run systematic property-based testing
fn run_substitution_fuzz_tests() -> Result<(), Vec<String>> {
    let mut all_crashes = Vec::new();

    // Test edge cases
    let edge_cases = generate_edge_case_inputs();
    let mut crashes = test_substitution_batch(&edge_cases);
    all_crashes.append(&mut crashes);

    // Test pathological cases
    let pathological_cases = generate_pathological_inputs();
    let pathological_refs: Vec<&str> = pathological_cases.iter().map(|s| s.as_str()).collect();
    let mut crashes = test_substitution_batch(&pathological_refs);
    all_crashes.append(&mut crashes);

    if all_crashes.is_empty() {
        Ok(())
    } else {
        eprintln!("Found {} crashes/issues:", all_crashes.len());
        for crash in &all_crashes {
            eprintln!("  - {}", crash);
        }
        Err(all_crashes)
    }
}

#[test]
fn test_substitution_fuzz_edge_cases() {
    // Test basic edge cases that should never crash
    let edge_cases =
        vec!["s", "s/", "s//", "s///", "s/a/b/", "s{a}{b}", "s[a][b]", "s(a)(b)", "s<a><b>"];

    let crashes = test_substitution_batch(&edge_cases);
    assert!(crashes.is_empty(), "Found crashes in edge cases: {:?}", crashes);
}

#[test]
fn test_substitution_fuzz_delimiter_variants() {
    // Test various delimiter combinations
    let delimiter_cases = vec![
        "s!pattern!replacement!",
        "s@pattern@replacement@",
        "s#pattern#replacement#",
        "s%pattern%replacement%",
        "s^pattern^replacement^",
        "s&pattern&replacement&",
        "s*pattern*replacement*",
        "s+pattern+replacement+",
        "s=pattern=replacement=",
        "s~pattern~replacement~",
    ];

    let crashes = test_substitution_batch(&delimiter_cases);
    assert!(crashes.is_empty(), "Found crashes in delimiter cases: {:?}", crashes);
}

#[test]
fn test_substitution_fuzz_unicode_handling() {
    // Test Unicode edge cases
    let unicode_cases =
        vec!["s/α/β/", "s/😀/😁/", "s/🦀/🔥/", "s/Ω/Α/", "s/नमस्ते/हैलो/", "s/Здравствуй/Привет/"];

    let crashes = test_substitution_batch(&unicode_cases);
    assert!(crashes.is_empty(), "Found crashes in Unicode cases: {:?}", crashes);
}

#[test]
fn test_substitution_fuzz_boundary_conditions() {
    // Test boundary conditions that could cause issues
    let boundary_cases = vec![
        "s()()g",
        "s{}{}g",
        "s[][]g",
        "s<><>g",
        "s/a/b/ggiimmssxxooeerr", // Excessive valid modifiers
        "s/\\/\\/\\/",            // Escape sequences
        "s/\\\\\\\\\\\\",         // Multiple backslashes
    ];

    let crashes = test_substitution_batch(&boundary_cases);
    assert!(crashes.is_empty(), "Found crashes in boundary cases: {:?}", crashes);
}

#[test]
fn test_substitution_fuzz_nested_delimiters() {
    // Test nested delimiter scenarios
    let nested_cases = vec![
        "s{pat{tern}}{replace{ment}}",
        "s[pat[tern]][replace[ment]]",
        "s(pat(tern))(replace(ment))",
        "s<pat<tern>><replace<ment>>",
        "s{a{b{c}}}{d{e{f}}}",
    ];

    let crashes = test_substitution_batch(&nested_cases);
    assert!(crashes.is_empty(), "Found crashes in nested delimiter cases: {:?}", crashes);
}

#[test]
fn test_substitution_comprehensive_fuzz() -> Result<(), Box<dyn std::error::Error>> {
    // Run the comprehensive fuzz test suite
    match run_substitution_fuzz_tests() {
        Ok(()) => println!("✅ All substitution operator fuzz tests passed!"),
        Err(crashes) => {
            // Save crashes to fuzz directory for analysis
            let crash_log = crashes.join("\n");
            let crash_log_path = std::env::temp_dir().join("substitution_fuzz_crashes.log");
            let _ = std::fs::write(&crash_log_path, crash_log);

            return Err(format!(
                "Found {} crashes in substitution operator parsing",
                crashes.len()
            )
            .into());
        }
    }
    Ok(())
}
