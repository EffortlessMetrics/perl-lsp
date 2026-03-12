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
});
