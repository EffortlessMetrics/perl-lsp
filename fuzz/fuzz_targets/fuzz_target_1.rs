#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::{format_with_trivia, Parser, SymbolExtractor, TriviaPreservingParser};

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

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let source = input.as_ref();

    let snippets = [
        source.to_string(),
        format!("package {}::Pkg; sub handler {{ {} }}", truncate_chars(source, 12), source),
        format!("use strict;\n# fuzz\nmy $value = q{{{}}};\n", source),
        format!("sub {} {{\n    return {}\n}}", truncate_chars(source, 8), source),
        format!("my @items = map {{ {} }} grep {{ {} }} @ARGV;", source, source),
    ];

    for snippet in &snippets {
        let mut parser = Parser::new(snippet);
        let result = parser.parse();

        let trivia_tree = TriviaPreservingParser::new(snippet.clone()).parse();
        let _formatted = format_with_trivia(&trivia_tree);

        if let Ok(ast) = &result {
            let extractor = SymbolExtractor::new_with_source(snippet);
            let symbol_table = extractor.extract(ast);
            let _symbol_count = symbol_table.symbols.len();
            let _reference_count = symbol_table.references.len();
        }

    }
});
