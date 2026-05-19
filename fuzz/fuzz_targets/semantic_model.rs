#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::{Parser, SemanticModel, SourceLocation};

const MAX_INPUT_BYTES: usize = 2048;
const MAX_QUERY_POSITIONS: usize = 64;
const MAX_SNIPPET_CHARS: usize = 256;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    if data.is_empty() {
        return std::borrow::Cow::Borrowed("");
    }

    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };

    String::from_utf8_lossy(capped)
}

fn safe_snippet(input: &str) -> String {
    input.chars().take(MAX_SNIPPET_CHARS).filter(|ch| *ch != '\0').collect()
}

fn query_model(model: &SemanticModel, source: &str) {
    let symbol_table = model.symbol_table();

    let mut positions = source
        .char_indices()
        .map(|(offset, _)| offset)
        .take(MAX_QUERY_POSITIONS)
        .collect::<Vec<_>>();
    positions.push(source.len());

    for position in positions {
        let location = SourceLocation { start: position, end: position };
        let _ = model.hover_info_at(location);
        let _ = model.definition_at(position);

        for symbols in symbol_table.symbols.values() {
            for symbol in symbols.iter().take(4) {
                let _ = symbol_table.find_references(symbol);
            }
        }
    }

    for name in symbol_table.symbols.keys().take(16) {
        let _ = model.resolve_inherited_method_location(name, "new");
        let _ = model.resolve_inherited_method_location(name, "BUILD");
        let _ = model.parent_chain(name);
    }

    let _ = model.tokens();
    let _ = model.export_metadata();
    let _ = model.package_edges();
    let _ = model.generated_members();
}

fn parse_and_query(source: &str) {
    let mut parser = Parser::new(source);
    let Ok(ast) = parser.parse() else { return };

    let model = SemanticModel::build(&ast, source);
    query_model(&model, source);
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let snippet = safe_snippet(&input);

    let variants = [
        snippet.clone(),
        format!("package Fuzz::Pkg;\nuse strict;\nuse warnings;\nmy $value = q{{{snippet}}};\n$value;\n"),
        format!("package Parent; sub new {{ bless {{}}, shift }}\npackage Child; our @ISA = qw(Parent); sub method {{ my ($self) = @_; return $self->new({snippet}); }}\n"),
        format!("package RoleUser;\nuse Moo;\nhas field => (is => 'rw');\nsub run {{ my $self = shift; return $self->field({snippet}); }}\n"),
        format!("use Exporter 'import';\nour @EXPORT_OK = qw(fuzzed);\nsub fuzzed {{ return q{{{snippet}}}; }}\n"),
    ];

    for source in &variants {
        parse_and_query(source);
    }
});
