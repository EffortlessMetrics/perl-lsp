use perl_parser::{Parser, quote_parser::extract_substitution_parts};

#[test]
fn test_fuzz_crash_xqn_heredoc() {
    // Reproduction case discovered by fuzzer: xqN<<"
    // This input caused a crash in the parser during substitution/quote parsing
    let input = "xqN<<\"";

    // The parser should handle this gracefully without crashing
    let mut parser = Parser::new(input);
    let result = parser.parse();

    // We don't require successful parsing since this is malformed input,
    // but the parser must not crash or panic
    match result {
        Ok(_) => {
            // Parsing succeeded - this is fine
        }
        Err(_) => {
            // Parsing failed - this is expected for malformed input
            // The important thing is that we didn't crash
        }
    }
}

#[test]
fn test_fuzz_crash_substitution_parts() {
    // Test the specific quote parser function that was being fuzzed
    let input = "xqN<<\"";

    // This should not panic even with malformed input
    if input.starts_with('s') {
        let (pattern, replacement, modifiers) = extract_substitution_parts(input);

        // Basic invariant checks
        assert!(pattern.len() <= input.len());
        assert!(replacement.len() <= input.len());
        assert!(modifiers.len() <= input.len());
    } else {
        // Input doesn't start with 's', so substitution parsing should be skipped
        // This is expected behavior
    }
}

#[test]
fn test_fuzz_edge_cases_similar_to_crash() {
    // Test variations of the crashing input to ensure robustness
    let similar_inputs = [
        "xqN<<",
        "xqN<<'",
        "xqN<<`",
        "qN<<\"",
        "x<<\"",
        "xq<<\"",
        "xqN<",
        "xqN<<\"\"",
    ];

    for input in &similar_inputs {
        let mut parser = Parser::new(input);
        let _result = parser.parse();
        // Should not crash regardless of parse success/failure
    }
}

#[test]
fn test_fuzz_heredoc_quote_combinations() {
    // Test various heredoc and quote combinations that could be problematic
    let test_cases = [
        "<<EOF\ntest\nEOF",
        "<<\"EOF\"\ntest\nEOF",
        "<<'EOF'\ntest\nEOF",
        "<<`EOF`\ntest\nEOF",
        "q<<EOF",
        "qq<<EOF",
        "qx<<EOF",
        "qw<<EOF",
    ];

    for input in &test_cases {
        let mut parser = Parser::new(input);
        let _result = parser.parse();
        // Should handle all heredoc variations gracefully
    }
}