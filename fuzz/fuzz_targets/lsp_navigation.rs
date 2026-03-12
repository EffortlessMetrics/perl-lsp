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
});
