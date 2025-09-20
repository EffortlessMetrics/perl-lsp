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

        // Test dual indexing patterns that were enhanced in PR #153
        let package_patterns = [
            format!("Package::{}", input),
            format!("{}::function", input),
            format!("{}::{}", input, input),
            format!("my $pkg = {}; $pkg::method()", input),
        ];

        for pattern in &package_patterns {
            // Test parser with dual indexing patterns
            let mut parser = Parser::new(pattern);
            let _result = parser.parse();
        }

        // Test file path completion patterns that could cause path traversal
        let path_patterns = [
            format!("use ../../../{}", input),
            format!("require '{}'", input.replace('/', "\\").replace("\\", "/")),
            format!("do '{}'", input),
            format!("use lib '{}'", input),
        ];

        for pattern in &path_patterns {
            // Test parser with potentially malicious path constructs
            let mut parser = Parser::new(pattern);
            let _result = parser.parse();
        }

        // Test workspace navigation edge cases
        let navigation_patterns = [
            format!("sub {}::method {{}}", input),
            format!("package {}; sub method {{}}", input),
            format!("use {}::{{}}", input),
        ];

        for pattern in &navigation_patterns {
            let mut parser = Parser::new(pattern);
            let _result = parser.parse();
        }
    }
});