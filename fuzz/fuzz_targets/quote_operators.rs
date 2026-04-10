#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_quote::{extract_regex_parts, extract_substitution_parts, extract_transliteration_parts};

const MAX_INPUT_BYTES: usize = 1024;

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

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Raw input against each extractor.
    let _ = extract_regex_parts(&input);
    let _ = extract_substitution_parts(&input);
    let _ = extract_transliteration_parts(&input);

    // Template inputs to drive parser branches for quoted/pair-delimited operators.
    let templates = [
        format!("/{}/", input),
        format!("m{{{}}}", input),
        format!("qr<{0}>ix", input),
        format!("s/{0}/{0}/g", input),
        format!("s{{{0}}}({0})e", input),
        format!("tr[{0}]<{0}>cd", input),
        format!("y|{0}|{0}|r", input),
    ];

    for candidate in &templates {
        let _ = extract_regex_parts(candidate);
        let _ = extract_substitution_parts(candidate);
        let _ = extract_transliteration_parts(candidate);
    }
});
