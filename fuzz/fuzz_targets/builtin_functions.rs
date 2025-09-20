#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::Parser;

fuzz_target!(|data: &[u8]| {
    // Convert fuzz input to string, handling invalid UTF-8 gracefully
    if let Ok(input) = std::str::from_utf8(data) {
        // Skip overly long inputs to prevent timeout
        if input.len() > 1000 {
            return;
        }

        // Focus on builtin function edge cases
        // Test map/grep/sort with {} blocks that PR #153 enhanced
        let builtin_prefixes = ["map{", "grep{", "sort{", "map {", "grep {", "sort {"];

        for prefix in &builtin_prefixes {
            let test_input = format!("{}{}", prefix, input);

            // Test parser with builtin function constructs
            let mut parser = Parser::new(&test_input);
            let _result = parser.parse();

            // We don't assert on parse success/failure since many fuzz inputs
            // will be malformed, but the parser should never crash or panic
        }

        // Test empty block edge cases that were specifically enhanced in PR #153
        let empty_block_tests = [
            format!("map{{{}}}", input),
            format!("grep{{{}}}", input),
            format!("sort{{{}}}", input),
            format!("map{{}}{}", input),
            format!("grep{{}}{}", input),
            format!("sort{{}}{}", input),
        ];

        for test_case in &empty_block_tests {
            let mut parser = Parser::new(test_case);
            let _result = parser.parse();
        }
    }
});