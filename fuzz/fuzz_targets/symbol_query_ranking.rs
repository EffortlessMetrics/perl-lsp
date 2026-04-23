#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_lsp_rs_core::providers::symbol_query::{compare_names_by_query, matches_query};

const MAX_INPUT_BYTES: usize = 2048;
const MAX_TOKENS: usize = 64;
const MAX_TOKEN_CHARS: usize = 64;

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

fn split_tokens(input: &str) -> Vec<String> {
    input
        .split(['\n', '\0', '|', ',', ';'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.chars().take(MAX_TOKEN_CHARS).collect::<String>())
        .take(MAX_TOKENS)
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let mut tokens = split_tokens(&input);

    if tokens.is_empty() {
        tokens.push("symbol".to_string());
    }

    for query in &tokens {
        for name in &tokens {
            let _ = matches_query(name, query);
        }
    }

    for query in &tokens {
        let mut sorted = tokens.clone();
        sorted.sort_by(|a, b| compare_names_by_query(a, b, query));
        sorted.dedup();

        let mut reversed = sorted.clone();
        reversed.reverse();
        reversed.sort_by(|a, b| compare_names_by_query(a, b, query));
    }
});
