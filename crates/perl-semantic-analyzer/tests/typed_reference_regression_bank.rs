use perl_semantic_analyzer::{
    Parser,
    symbol::{SymbolExtractor, SymbolKind, SymbolReference, SymbolTable},
};
use perl_tdd_support::must;

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn refs_for<'a>(table: &'a SymbolTable, name: &str) -> &'a [SymbolReference] {
    table.references.get(name).map(Vec::as_slice).unwrap_or(&[])
}

#[test]
fn typed_reference_baseline_sub_definition_vs_call_current_api_collapses_to_kind() {
    let table = extract_symbols("sub foo { 1 } foo();");

    let defs = table.find_symbol("foo", 0, SymbolKind::Subroutine);
    assert!(!defs.is_empty(), "expected a subroutine definition for foo");

    let refs = refs_for(&table, "foo");
    assert!(refs.iter().any(|r| r.kind == SymbolKind::Subroutine));
}

#[test]
fn typed_reference_baseline_variable_read_vs_write_flag_is_currently_missing() {
    let table = extract_symbols("my $x = 1; $x = $x + 1;");

    let refs = refs_for(&table, "x");
    assert!(!refs.is_empty(), "expected references to $x");
    assert!(
        refs.iter().all(|r| !r.is_write),
        "current baseline: write sites are not typed; all refs keep is_write=false"
    );
}

#[test]
fn typed_reference_baseline_import_export_inheritance_and_role_edges_not_recorded_as_refs() {
    let code = r#"
package Child;
use parent 'Base';
use Exporter 'import';
our @EXPORT_OK = qw(foo);
use Module qw(foo);
with 'Role::Named';
"#;
    let table = extract_symbols(code);

    assert!(refs_for(&table, "Base").is_empty());
    assert!(refs_for(&table, "Module").is_empty());
    assert!(refs_for(&table, "Role::Named").is_empty());
    assert!(!table.find_symbol("EXPORT_OK", 0, SymbolKind::array()).is_empty());
}

#[test]
fn typed_reference_baseline_generated_accessor_symbol_exists_but_edge_provenance_is_untyped() {
    let code = r#"
package MyClass;
use Moo;
has 'name' => (is => 'rw');
"#;
    let table = extract_symbols(code);

    assert!(!table.find_symbol("name", 0, SymbolKind::Subroutine).is_empty());
}

#[test]
fn typed_reference_baseline_sub_ref_forms_dynamic_and_alias_boundaries_are_untyped() {
    let code = r#"
sub foo { 1 }
my $code = \&foo;
&foo();
goto &foo;
*alias = \&foo;
eval "foo()";
"#;
    let table = extract_symbols(code);

    assert!(!table.find_symbol("foo", 0, SymbolKind::Subroutine).is_empty());
    let refs = refs_for(&table, "foo");
    assert!(refs.iter().any(|r| r.kind == SymbolKind::Subroutine));
}
