//! Framework semantic extraction tests for Moo/Moose/Class::Accessor.

use perl_semantic_analyzer::{
    Parser,
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::{must, must_some};

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

#[test]
fn moo_has_qw_attribute_list_generates_symbols_for_each_attribute() {
    let code = r#"
use Moo;
has [qw(first_name last_name)] => (is => 'ro');
"#;

    let table = extract_symbols(code);

    for attr in ["first_name", "last_name"] {
        assert!(has_symbol(&table, attr, SymbolKind::scalar()), "expected attribute `{attr}`");
        assert!(
            has_symbol(&table, attr, SymbolKind::Subroutine),
            "expected generated accessor `{attr}`"
        );
    }
}

#[test]
fn moo_has_generates_builder_predicate_clearer_and_handles_methods() {
    let code = r#"
use Moo;
has 'profile' => (
    is => 'rw',
    builder => 1,
    predicate => 1,
    clearer => 1,
    handles => [qw(full_name timezone)],
);
"#;

    let table = extract_symbols(code);

    for method in
        ["profile", "_build_profile", "has_profile", "clear_profile", "full_name", "timezone"]
    {
        assert!(
            has_symbol(&table, method, SymbolKind::Subroutine),
            "expected generated Moo method `{method}`"
        );
    }
}

#[test]
fn moo_has_handles_hash_generates_delegated_methods() {
    let code = r#"
use Moo;
has 'profile' => (
    is => 'ro',
    handles => {
        full_name => 'name',
        timezone => 'tz',
    },
);
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "full_name", SymbolKind::Subroutine),
        "expected delegated method `full_name`"
    );
    assert!(
        has_symbol(&table, "timezone", SymbolKind::Subroutine),
        "expected delegated method `timezone`"
    );
}

#[test]
fn moo_per_package_scoping_only_enables_has_in_moo_package() {
    let code = r#"
package MooClass;
use Moo;
has 'name' => (is => 'ro');

package PlainClass;
has 'age' => (is => 'ro');
"#;

    let table = extract_symbols(code);

    // MooClass: `use Moo` is active, so `has` synthesis should fire
    assert!(
        has_symbol(&table, "name", SymbolKind::scalar()),
        "expected Moo attribute `name` in MooClass"
    );
    assert!(
        has_symbol(&table, "name", SymbolKind::Subroutine),
        "expected accessor `name` in MooClass"
    );

    // PlainClass: no `use Moo`, so `has` should NOT be synthesised
    assert!(
        !has_symbol(&table, "age", SymbolKind::scalar()),
        "did not expect synthetic attribute `age` in PlainClass (no Moo)"
    );
    assert!(
        !has_symbol(&table, "age", SymbolKind::Subroutine),
        "did not expect synthetic accessor `age` in PlainClass (no Moo)"
    );
}

#[test]
fn moo_package_emits_class_symbol_kind() {
    let code = r#"
package MyApp::User;
use Moo;
has 'name' => (is => 'ro');
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "MyApp::User", SymbolKind::Class),
        "expected SymbolKind::Class for Moo package"
    );
    assert!(
        !has_symbol(&table, "MyApp::User", SymbolKind::Package),
        "Moo package should be upgraded from Package to Class"
    );
}

#[test]
fn moo_role_package_emits_role_symbol_kind() {
    let code = r#"
package MyApp::Printable;
use Moo::Role;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "MyApp::Printable", SymbolKind::Role),
        "expected SymbolKind::Role for Moo::Role package"
    );
    assert!(
        !has_symbol(&table, "MyApp::Printable", SymbolKind::Package),
        "Moo::Role package should be upgraded from Package to Role"
    );
}

#[test]
fn plain_package_keeps_package_symbol_kind() {
    let code = r#"
package MyApp::Utils;
sub helper { 1 }
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "MyApp::Utils", SymbolKind::Package),
        "plain package should remain SymbolKind::Package"
    );
    assert!(
        !has_symbol(&table, "MyApp::Utils", SymbolKind::Class),
        "plain package should NOT be Class"
    );
}

// ---- Method modifier tests (Task #10) ----

fn find_symbol_with_declaration<'a>(
    table: &'a SymbolTable,
    name: &str,
    kind: SymbolKind,
    declaration: &str,
) -> Option<&'a perl_semantic_analyzer::symbol::Symbol> {
    table.symbols.get(name).and_then(|symbols| {
        symbols.iter().find(|s| s.kind == kind && s.declaration.as_deref() == Some(declaration))
    })
}

fn has_reference(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.references.get(name).is_some_and(|refs| refs.iter().any(|r| r.kind == kind))
}

#[test]
fn moo_around_modifier_emits_subroutine_symbol() {
    let code = r#"
package MyApp::User;
use Moo;
around 'name' => sub { };
"#;

    let table = extract_symbols(code);

    let sym = find_symbol_with_declaration(&table, "name", SymbolKind::Subroutine, "around");
    assert!(sym.is_some(), "expected Subroutine symbol with declaration='around' for `name`");
}

#[test]
fn moo_before_modifier_emits_subroutine_symbol() {
    let code = r#"
package MyApp::User;
use Moo;
before 'validate' => sub { };
"#;

    let table = extract_symbols(code);

    let sym = find_symbol_with_declaration(&table, "validate", SymbolKind::Subroutine, "before");
    assert!(sym.is_some(), "expected Subroutine symbol with declaration='before' for `validate`");
}

#[test]
fn moo_after_modifier_emits_subroutine_symbol() {
    let code = r#"
package MyApp::User;
use Moo;
after 'cleanup' => sub { };
"#;

    let table = extract_symbols(code);

    let sym = find_symbol_with_declaration(&table, "cleanup", SymbolKind::Subroutine, "after");
    assert!(sym.is_some(), "expected Subroutine symbol with declaration='after' for `cleanup`");
}

#[test]
fn moose_override_modifier_emits_subroutine_symbol() {
    let code = r#"
package MyApp::User;
use Moose;
override 'render' => sub { };
"#;

    let table = extract_symbols(code);

    let sym = find_symbol_with_declaration(&table, "render", SymbolKind::Subroutine, "override");
    assert!(sym.is_some(), "expected Subroutine symbol with declaration='override' for `render`");
}

#[test]
fn moose_augment_modifier_emits_subroutine_symbol() {
    let code = r#"
package MyApp::User;
use Moose;
augment 'render' => sub { };
"#;

    let table = extract_symbols(code);

    let sym = find_symbol_with_declaration(&table, "render", SymbolKind::Subroutine, "augment");
    assert!(sym.is_some(), "expected Subroutine symbol with declaration='augment' for `render`");
}

#[test]
fn moo_modifier_not_emitted_without_framework() {
    let code = r#"
package Plain;
around 'name' => sub { };
"#;

    let table = extract_symbols(code);

    assert!(
        find_symbol_with_declaration(&table, "name", SymbolKind::Subroutine, "around").is_none(),
        "should not emit modifier symbol without Moo/Moose"
    );
}

#[test]
fn moo_extends_emits_class_reference() {
    let code = r#"
package MyApp::Admin;
use Moo;
extends 'MyApp::User';
"#;

    let table = extract_symbols(code);

    assert!(
        has_reference(&table, "MyApp::User", SymbolKind::Class),
        "expected Class reference for `extends 'MyApp::User'`"
    );
}

#[test]
fn moo_with_emits_role_reference() {
    let code = r#"
package MyApp::User;
use Moo;
with 'MyApp::Printable';
"#;

    let table = extract_symbols(code);

    assert!(
        has_reference(&table, "MyApp::Printable", SymbolKind::Role),
        "expected Role reference for `with 'MyApp::Printable'`"
    );
}

#[test]
fn moo_extends_with_not_emitted_without_framework() {
    let code = r#"
package Plain;
extends 'Parent';
with 'SomeRole';
"#;

    let table = extract_symbols(code);

    assert!(
        !has_reference(&table, "Parent", SymbolKind::Class),
        "should not emit extends reference without Moo"
    );
    assert!(
        !has_reference(&table, "SomeRole", SymbolKind::Role),
        "should not emit with reference without Moo"
    );
}

#[test]
fn moo_has_multiple_attributes_list() {
    let code = r#"
use Moose;
has 'first_name', 'last_name' => (is => 'ro');
"#;

    let table = extract_symbols(code);

    for attr in ["first_name", "last_name"] {
        assert!(has_symbol(&table, attr, SymbolKind::scalar()), "expected attribute `{attr}`");
        assert!(
            has_symbol(&table, attr, SymbolKind::Subroutine),
            "expected generated accessor `{attr}`"
        );
    }
}

#[test]
fn moo_role_requires_emits_subroutine_symbol() {
    let code = r#"
package MyApp::Role;
use Moo::Role;
requires 'some_method', 'another_method';
"#;

    let table = extract_symbols(code);

    let some_method =
        find_symbol_with_declaration(&table, "some_method", SymbolKind::Subroutine, "requires");
    assert!(
        some_method.is_some(),
        "expected Subroutine symbol with declaration='requires' for `some_method`"
    );
    assert!(must_some(some_method).attributes.contains(&"requires=true".to_string()));

    let another_method =
        find_symbol_with_declaration(&table, "another_method", SymbolKind::Subroutine, "requires");
    assert!(
        another_method.is_some(),
        "expected Subroutine symbol with declaration='requires' for `another_method`"
    );
}
