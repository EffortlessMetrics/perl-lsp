#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 1000;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    if data.is_empty() {
        return std::borrow::Cow::Borrowed("");
    }

    let capped = if data.len() <= MAX_INPUT_BYTES {
        data
    } else {
        &data[..MAX_INPUT_BYTES]
    };

    String::from_utf8_lossy(capped)
}

use perl_parser::Parser;

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Test heredoc patterns that specifically target the boundary fix in cd7a2442
    // This focuses on the off-by-one error that was fixed in parse_heredoc_delimiter
    let heredoc_patterns = [
        // Double-quoted heredoc delimiters (line 5267 fix)
        format!("<<\"{}\"", input),
        format!("<<\"{}\"\nEND\n{}\nEND", input, input),
        format!("<<\"{}\"", input.chars().take(1).collect::<String>()), // Single char - edge case
        "<<\"\"".to_string(), // Empty delimiter - critical edge case
        // Single-quoted heredoc delimiters (line 5270 fix)
        format!("<<'{}'", input),
        format!("<<'{}'\nEND\n{}\nEND", input, input),
        format!("<<'{}'", input.chars().take(1).collect::<String>()), // Single char - edge case
        "<<''".to_string(), // Empty delimiter - critical edge case
        // Bare heredoc delimiters (should be unaffected but test anyway)
        format!("<<{}", input),
        format!("<<{}\nEND\n{}\nEND", input, input),
        // Malformed heredoc constructs that could trigger boundary issues
        "<<\"".to_string(), // Unterminated quote - crash condition
        "<<'".to_string(),   // Unterminated quote - crash condition
        format!("<<\"{}", input), // Missing closing quote
        format!("<<'{}", input), // Missing closing quote
        format!("<<\"{}", input.chars().take(1).collect::<String>()), // Short input, missing quote
    ];

    for pattern in &heredoc_patterns {
        // Test parser with heredoc constructs
        // The boundary fix should prevent crashes on malformed delimiters
        let mut parser = Parser::new(pattern);
        let _result = parser.parse();
    }

    // Test specific edge cases that could trigger the original off-by-one error.
    // Use Unicode-safe single-char extraction so arbitrary fuzz input never panics.
    if let Some(first_char) = input.chars().next() {
        let first = first_char.to_string();
        let edge_cases = [
            format!("<<\"{}\"", first), // Single character
            format!("<<'{}'", first),   // Single character
        ];

        for case in &edge_cases {
            let mut parser = Parser::new(case);
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
        if test.len() <= MAX_INPUT_BYTES {
            let mut parser = Parser::new(test);
            let _result = parser.parse();
        }
    }
});
