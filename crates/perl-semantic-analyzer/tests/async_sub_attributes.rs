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

fn symbol_attrs(table: &SymbolTable, name: &str, kind: SymbolKind) -> Vec<String> {
    table
        .symbols
        .get(name)
        .and_then(|symbols| symbols.iter().find(|symbol| symbol.kind == kind))
        .map(|symbol| symbol.attributes.clone())
        .unwrap_or_default()
}

#[test]
fn named_async_subroutines_propagate_the_async_attribute() {
    let code = r#"
use Future::AsyncAwait;

async sub fetch {
    return await lookup();
}
"#;

    let table = extract_symbols(code);
    let attrs = symbol_attrs(&table, "fetch", SymbolKind::Subroutine);

    assert!(
        attrs.iter().any(|attr| attr == "async"),
        "expected `async` attribute on named async subroutine, got {attrs:?}"
    );
}
