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

    // Test Unicode handling and emoji identifiers that were security-hardened in PR #153
    let unicode_test_patterns = [
        format!("my $🦀_var = {}", input),
        format!("sub λ_{} {{}}", input),
        format!("package 日本語::{}", input),
        format!("use utf8; my $var = '{}'", input),
        format!("my $emoji = '🚀{}'", input),
    ];

    for pattern in &unicode_test_patterns {
        // Test parser with Unicode content - this exercises UTF-16 position mapping
        let mut parser = Parser::new(pattern);
        let _result = parser.parse();
    }

    // Test specifically with Unicode characters that can cause position conversion issues
    if input.chars().any(|c| c > '\u{007F}') {
        // Test with the original input containing Unicode
        let mut parser = Parser::new(&input);
        let _result = parser.parse();

        // Test with combinations that stress UTF-16 boundary handling
        let boundary_tests = [
            format!("# Comment with {}\nmy $var = 1;", input),
            format!("my $var = q{{{}}};", input),
            format!("my $str = \"{}\";", input.replace('"', "\\\"")),
        ];

        for test in &boundary_tests {
            let mut parser = Parser::new(test);
            let _result = parser.parse();
        }
    }

    // Test edge cases that could trigger the UTF-16 security fixes from PR #153
    let position_stress_tests = [
        format!("{}\n{}\n{}", input, input, input), // Multi-line stress
        format!("'{}'", input.repeat(10)),             // Repeated patterns
        input.chars().rev().collect::<String>(),       // Reversed input
    ];

    for test in &position_stress_tests {
        if test.len() <= MAX_INPUT_BYTES {
            let mut parser = Parser::new(test);
            let _result = parser.parse();
        }
    }
});
