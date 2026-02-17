//! Framework semantic extraction tests for Moo/Moose/Class::Accessor.

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

#[test]
fn moo_has_emits_attribute_and_accessor_symbols() {
    let code = r#"
package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');

sub greet {
    my $self = shift;
    return $self->name;
}
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "name", SymbolKind::scalar()),
        "expected Moo attribute `name` scalar symbol"
    );
    assert!(
        has_symbol(&table, "name", SymbolKind::Subroutine),
        "expected default accessor method symbol for `name`"
    );

    let references = table.references.get("name");
    assert!(
        references.is_some_and(|refs| refs.iter().any(|r| r.kind == SymbolKind::Subroutine)),
        "expected method-call reference for `$self->name`"
    );
}

#[test]
fn moo_has_custom_reader_writer_symbols() {
    let code = r#"
use Moo;
has 'name' => (reader => 'get_name', writer => 'set_name');
"#;

    let table = extract_symbols(code);

    assert!(has_symbol(&table, "name", SymbolKind::scalar()), "expected attribute symbol `name`");
    assert!(
        has_symbol(&table, "get_name", SymbolKind::Subroutine),
        "expected reader accessor symbol"
    );
    assert!(
        has_symbol(&table, "set_name", SymbolKind::Subroutine),
        "expected writer accessor symbol"
    );
}

#[test]
fn class_accessor_generates_method_symbols() {
    let code = r#"
package Example::Accessor;
use parent 'Class::Accessor';
__PACKAGE__->mk_accessors(qw(foo bar));
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "foo", SymbolKind::Subroutine),
        "expected generated Class::Accessor method `foo`"
    );
    assert!(
        has_symbol(&table, "bar", SymbolKind::Subroutine),
        "expected generated Class::Accessor method `bar`"
    );
}

#[test]
fn plain_has_without_framework_is_not_treated_as_attribute() {
    let code = r#"
sub has { return 1; }
has 'name' => (is => 'ro');
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "name", SymbolKind::scalar()),
        "did not expect synthetic attribute without Moo/Moose context"
    );
    assert!(
        !has_symbol(&table, "name", SymbolKind::Subroutine),
        "did not expect synthetic accessor without Moo/Moose context"
    );
}
