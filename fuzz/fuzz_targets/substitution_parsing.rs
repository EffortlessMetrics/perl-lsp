#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::quote_parser::extract_substitution_parts;
use perl_parser::Parser;

fuzz_target!(|data: &[u8]| {
    // Convert fuzz input to string, handling invalid UTF-8 gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // Skip overly long inputs to prevent timeout
        if input.len() > 1000 {
            return;
        }

        // Test quote_parser::extract_substitution_parts directly
        // This targets the core substitution parsing logic
        if input.starts_with('s') {
            let (pattern, replacement, modifiers) = extract_substitution_parts(input);

            // Basic invariant checks - these should never panic or crash
            assert!(pattern.len() <= input.len());
            assert!(replacement.len() <= input.len());
            assert!(modifiers.len() <= input.len());

            // Verify modifiers only contain valid characters
            for ch in modifiers.chars() {
                assert!(matches!(ch, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'));
            }
        }

        // Test full parser with substitution inputs
        // This tests the complete parsing pipeline
        let mut parser = Parser::new(input);
        let _result = parser.parse();

        // We don't assert on parse success/failure since many fuzz inputs
        // will be malformed, but the parser should never crash or panic
    }
});