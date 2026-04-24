//! Semantic overlay query smoke tests for the tree-sitter facade.

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn definition_query_resolves_symbol_at_offset() {
    let source = "my $value = 1;\n$value += 1;\n";
    let tree = parse(source);
    let offset = must_some(source.find("$value += 1"));
    let definition = must_some(tree.definition_at_offset(offset));

    assert_eq!(definition.name, "value");
    assert!(definition.start_byte < definition.end_byte);
}

#[test]
fn visible_imports_query_returns_in_scope_use_statements() {
    let source = "use strict;\nuse List::Util qw(first);\nmy $x = first(@vals);\n";
    let tree = parse(source);
    let offset = must_some(source.find("first(@vals)"));
    let imports = tree.visible_imports_at_offset(offset);

    assert!(imports.iter().any(|entry| entry.module == "strict"));
    assert!(imports.iter().any(|entry| {
        entry.module == "List::Util" && entry.symbols.iter().any(|symbol| symbol == "first")
    }));
}

#[test]
fn pragma_state_query_reflects_effective_state() {
    let source = "use strict;\nno strict 'subs';\nfoo;\n";
    let tree = parse(source);
    let offset = must_some(source.find("foo;"));
    let state = tree.effective_pragma_state_at_offset(offset);

    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}
