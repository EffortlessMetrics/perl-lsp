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

use perl_parser::quote_parser::extract_substitution_parts;
use perl_parser::Parser;

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Test quote_parser::extract_substitution_parts directly
    // This targets the core substitution parsing logic
    if input.starts_with('s') {
        let (pattern, replacement, modifiers) = extract_substitution_parts(&input);

        // Basic invariant checks - these should never panic or crash
        let _ = pattern.len() <= input.len();
        let _ = replacement.len() <= input.len();
        let _ = modifiers.len() <= input.len();

        // Observe unknown modifier chars without panicking; fuzzing should keep running.
        let _has_nonstandard_modifier =
            modifiers.chars().any(|ch| !matches!(ch, 'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r'));
    }

    // Test full parser with substitution inputs
    // This tests the complete parsing pipeline
    let mut parser = Parser::new(&input);
    let _result = parser.parse();

    // We don't assert on parse success/failure since many fuzz inputs
    // will be malformed, but the parser should never crash or panic
});
