#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::{format_with_trivia, Parser, SymbolExtractor, TriviaPreservingParser};

const MAX_INPUT_BYTES: usize = 1000;
const MAX_IDENTIFIER_CHARS: usize = 24;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    if data.is_empty() {
        return std::borrow::Cow::Borrowed("");
    }

    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };

    String::from_utf8_lossy(capped)
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn sanitize_identifier(input: &str) -> String {
    let mut identifier = String::new();

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            identifier.push(ch);
        }

        if identifier.len() >= MAX_IDENTIFIER_CHARS {
            break;
        }
    }

    if identifier.is_empty() {
        identifier.push('_');
    }

    if identifier.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        identifier.insert(0, '_');
    }

    identifier
}

fn parse_snippet(snippet: &str) {
    let mut parser = Parser::new(snippet);
    let result = parser.parse();

    let trivia_tree = TriviaPreservingParser::new(snippet.to_string()).parse();
    let _formatted = format_with_trivia(&trivia_tree);

    if let Ok(ast) = &result {
        let extractor = SymbolExtractor::new_with_source(snippet);
        let symbol_table = extractor.extract(ast);
        let _ = symbol_table.symbols.len();
        let _ = symbol_table.references.len();
    }
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let source = input.as_ref();

    let ident = sanitize_identifier(source);
    let short_source = truncate_chars(source, 96);

    let snippets = [
        source.to_string(),
        format!("package {}::Pkg; sub handler {{ {} }}", ident, source),
        format!("use strict;\n# fuzz\nmy $value = q{{{}}};\n", source),
        format!("sub {} {{\n    my ($x) = @_;\n    return {}\n}}", ident, short_source),
        format!("my @items = map {{ {} }} grep {{ {} }} @ARGV;", short_source, short_source),
        format!(
            "my $re = qr/{{{}}}/; $text =~ s/{{{}}}/{} /gr;",
            short_source, short_source, ident
        ),
        format!("my $doc = <<'{}';\n{}\n{}\nprint $doc;", ident, short_source, ident),
        format!("BEGIN {{ package {}::Boot; our $V = '{}'; }} END {{ 1; }}", ident, short_source),
        format!("=head1 {}\n\n{}\n\n=cut\nsub {} {{ return 1; }}", ident, short_source, ident),
    ];

    for snippet in &snippets {
        parse_snippet(snippet);
    }

    if let Some((left, right)) = source.split_once('\n') {
        let stitched = format!(
            "package {}::Split;\nsub left {{ {} }}\nsub right {{ {} }}",
            ident,
            truncate_chars(left, 80),
            truncate_chars(right, 80)
        );
        parse_snippet(&stitched);
    }
});
