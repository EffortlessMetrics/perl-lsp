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

        // Test heredoc patterns that specifically target the boundary fix in cd7a2442
        // This focuses on the off-by-one error that was fixed in parse_heredoc_delimiter
        let heredoc_patterns = [
            // Double-quoted heredoc delimiters (line 5267 fix)
            format!("<<\"{}\"", input),
            format!("<<\"{}\"\nEND\n{}\nEND", input, input),
            format!("<<\"{}\"", input.chars().take(1).collect::<String>()), // Single char - edge case
            format!("<<\"\""), // Empty delimiter - critical edge case

            // Single-quoted heredoc delimiters (line 5270 fix)
            format!("<<'{}'", input),
            format!("<<'{}'\nEND\n{}\nEND", input, input),
            format!("<<'{}'", input.chars().take(1).collect::<String>()), // Single char - edge case
            format!("<<''"), // Empty delimiter - critical edge case

            // Bare heredoc delimiters (should be unaffected but test anyway)
            format!("<<{}", input),
            format!("<<{}\nEND\n{}\nEND", input, input),

            // Malformed heredoc constructs that could trigger boundary issues
            "<<\"".to_string(), // Unterminated quote - crash condition
            "<<'".to_string(), // Unterminated quote - crash condition
            format!("<<\"{}", input), // Missing closing quote
            format!("<<'{}", input), // Missing closing quote
            format!("<<\"{}", input.chars().take(1).collect::<String>()), // Short input, missing quote
        ];

        for pattern in &heredoc_patterns {
            // Test parser with heredoc constructs
            // The boundary fix should prevent crashes on malformed delimiters
            let mut parser = Parser::new(pattern);
            let _result = parser.parse();

            // We don't assert on parse success/failure since many fuzz inputs
            // will be malformed, but the parser should never crash or panic
        }

        // Test specific edge cases that could trigger the original off-by-one error
        if input.len() > 0 {
            let edge_cases = [
                format!("<<\"{}\"", &input[..1.min(input.len())]), // Single character
                format!("<<'{}'", &input[..1.min(input.len())]), // Single character
            ];

            for case in &edge_cases {
                let mut parser = Parser::new(&case);
                let _result = parser.parse();
            }
        }

        // Test combinations with other Perl constructs to ensure heredoc parsing
        // doesn't break when integrated with complex syntax
        let integration_tests = [
            format!("my $var = <<\"{}\";\n{}\nEOF", input, input),
            format!("print <<'{}';\n{}\nEOF", input, input),
            format!("my @array = (<<\"{}\", 'other');\n{}\nEOF", input, input),
        ];

        for test in &integration_tests {
            if test.len() <= 1000 {
                let mut parser = Parser::new(&test);
                let _result = parser.parse();
            }
        }
    }
});