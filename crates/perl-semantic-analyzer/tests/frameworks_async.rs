//! Framework semantic extraction tests for IO::Async.

use perl_semantic_analyzer::{
    Parser,
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::must;

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == kind))
}

fn symbol_attrs(table: &SymbolTable, name: &str, kind: SymbolKind) -> Vec<String> {
    table
        .symbols
        .get(name)
        .and_then(|symbols| symbols.iter().find(|symbol| symbol.kind == kind))
        .map(|symbol| symbol.attributes.clone())
        .unwrap_or_default()
}

#[test]
fn io_async_use_synthesizes_class_symbols_for_common_namespaces() {
    let code = r#"
use IO::Async;

my $loop = IO::Async::Loop->new;
my $stream = IO::Async::Stream->new;
my $handle = IO::Async::Handle->new;
"#;

    let table = extract_symbols(code);

    for name in ["IO::Async::Loop", "IO::Async::Stream", "IO::Async::Handle"] {
        assert!(
            has_symbol(&table, name, SymbolKind::Class),
            "expected synthetic IO::Async class symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Class);
        assert!(
            attrs.iter().any(|attr| attr == "framework=IO::Async"),
            "expected `framework=IO::Async` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn io_async_namespace_import_enables_symbol_synthesis() {
    let code = r#"
use IO::Async::Loop;

my $loop = IO::Async::Loop->new;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "IO::Async::Loop", SymbolKind::Class),
        "expected namespace import to enable IO::Async class synthesis"
    );
}

#[test]
fn io_async_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $loop = IO::Async::Loop->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "IO::Async::Loop", SymbolKind::Class),
        "did not expect IO::Async class synthesis without `use IO::Async`"
    );
}
