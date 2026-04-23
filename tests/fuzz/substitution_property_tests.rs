/// Focused property-based tests for substitution operator parsing
///
/// These tests systematically explore edge cases and boundary conditions
/// in the substitution operator parsing logic without requiring full fuzz infrastructure.

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
                assert!(matches!(ch, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'),
                       "Invalid modifier '{}' in: {}", ch, input);
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
        "s()",
        "s{}",
        "s[]",
        "s<>",
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
        "s/\\x41/\\x42/",
        "s/\\n/\\t/",

        // Unicode edge cases
        "s/α/β/",
        "s/😀/😁/",
        "s/🦀/🔥/",
        "s/Ω/Α/",

        // Very long inputs
        &"s/".repeat(100),
        &format!("s/{}/{}/", "a".repeat(100), "b".repeat(100)),
        &format!("s/pattern/replacement/{}", "g".repeat(50)),

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
    for depth in 1..10 {
        let pattern = format!("s{}{}{}", "{".repeat(depth), "a".repeat(depth), "}".repeat(depth));
        let replacement = format!("{}{}{}", "{".repeat(depth), "b".repeat(depth), "}".repeat(depth));
        inputs.push(format!("s{}{}", pattern, replacement));
    }

    // Extremely long modifiers
    for len in [10, 50, 100, 500] {
        inputs.push(format!("s/a/b/{}", "g".repeat(len)));
        inputs.push(format!("s/a/b/{}", "invalid".repeat(len)));
    }

    // Binary data simulation
    for byte in [0u8, 1, 127, 128, 255] {
        if let Ok(s) = std::str::from_utf8(&[b's', b'/', byte, b'/', byte, b'/']) {
            inputs.push(s.to_string());
        }
    }

    inputs
}

/// Run systematic property-based testing
pub fn run_substitution_fuzz_tests() -> Result<(), Vec<String>> {
    println!("Running substitution operator property-based tests...");

    let mut all_crashes = Vec::new();

    // Test edge cases
    println!("Testing edge cases...");
    let edge_cases = generate_edge_case_inputs();
    let mut crashes = test_substitution_batch(&edge_cases);
    all_crashes.append(&mut crashes);

    // Test pathological cases
    println!("Testing pathological cases...");
    let pathological_cases = generate_pathological_inputs();
    let pathological_refs: Vec<&str> = pathological_cases.iter().map(|s| s.as_str()).collect();
    let mut crashes = test_substitution_batch(&pathological_refs);
    all_crashes.append(&mut crashes);

    if all_crashes.is_empty() {
        println!("✅ All substitution operator property tests passed!");
        Ok(())
    } else {
        println!("❌ Found {} crashes/issues:", all_crashes.len());
        for crash in &all_crashes {
            println!("  - {}", crash);
        }
        Err(all_crashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitution_property_based() {
        run_substitution_fuzz_tests().expect("Property-based tests should pass");
    }
}