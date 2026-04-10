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

use perl_parser::quote_parser::{
    extract_regex_parts, extract_substitution_parts, extract_substitution_parts_strict,
    extract_transliteration_parts,
};
use perl_parser::Parser;

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    let quote_patterns = [
        format!("m/{}/", input),
        format!("m{{{}}}", input),
        format!("qr/{}/", input),
        format!("s/{0}/{0}/g", input),
        format!("s{{{0}}}{{{0}}}gi", input),
        format!("tr/{0}/{0}/", input),
        format!("y{{{0}}}{{{0}}}", input),
    ];

    for pattern in &quote_patterns {
        let _ = extract_regex_parts(pattern);
        let _ = extract_substitution_parts(pattern);
        let _ = extract_substitution_parts_strict(pattern);
        let _ = extract_transliteration_parts(pattern);

        let mut parser = Parser::new(pattern);
        let _ = parser.parse();
    }

    let malformed_patterns = [
        format!("m/{}", input),
        format!("s/{}/", input),
        format!("s/{}/{}/q", input, input),
        format!("tr/{}/", input),
        format!("y{{{}}}", input),
    ];

    for pattern in &malformed_patterns {
        let _ = extract_substitution_parts_strict(pattern);

        let mut parser = Parser::new(pattern);
        let _ = parser.parse();
    }
});
