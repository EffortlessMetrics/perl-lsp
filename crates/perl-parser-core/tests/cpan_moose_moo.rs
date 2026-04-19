//! CPAN Pattern Tests: Moose / Moo

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn has_attribute_with_quoted_name() {
    let code = "has 'name' => (is => 'ro', isa => 'Str', default => sub { 'unknown' });";
    assert_clean_parse(code);
}

#[test]
fn has_attribute_with_bare_name() {
    let code = "has name => (is => 'ro', required => 1);";
    assert_clean_parse(code);
}

#[test]
fn has_attribute_rw_with_builder() {
    let code = "has 'cache' => (is => 'rw', lazy => 1, builder => '_build_cache');";
    assert_clean_parse(code);
}

#[test]
fn has_attribute_with_type_coercion() {
    let code = "has 'count' => (is => 'ro', isa => 'Int', coerce => 1, default => 0);";
    assert_clean_parse(code);
}

#[test]
fn has_arrayref_attribute() {
    let code = "has 'items' => (is => 'ro', isa => 'ArrayRef[Str]', default => sub { [] });";
    assert_clean_parse(code);
}

#[test]
fn with_single_role() {
    let code = "with 'Some::Role';";
    assert_clean_parse(code);
}

#[test]
fn with_multiple_roles() {
    let code = "with 'Role::One', 'Role::Two', 'Role::Three';";
    assert_clean_parse(code);
}

#[test]
fn extends_single_class() {
    let code = "extends 'Base::Class';";
    assert_clean_parse(code);
}

#[test]
fn extends_multiple_classes() {
    let code = "extends 'Base::One', 'Base::Two';";
    assert_clean_parse(code);
}

#[test]
fn around_modifier() {
    let code =
        r#"around 'method' => sub { my $orig = shift; my $self = shift; $self->$orig(@_) };"#;
    assert_clean_parse(code);
}

#[test]
fn before_modifier() {
    let code = "before 'save' => sub { my $self = shift; $self->validate };";
    assert_clean_parse(code);
}

#[test]
fn after_modifier() {
    let code = "after 'load' => sub { my $self = shift; $self->_post_load };";
    assert_clean_parse(code);
}

#[test]
fn override_modifier() {
    let code = "override 'render' => sub { my $self = shift; return super() . ' extra' };";
    assert_clean_parse(code);
}

#[test]
fn augment_modifier() {
    let code = "augment 'render' => sub { return ' more stuff' };";
    assert_clean_parse(code);
}

#[test]
fn full_moose_class() {
    let code = r#"
package Animal;
use Moose;

has 'name' => (is => 'ro', isa => 'Str', required => 1);
has 'age'  => (is => 'rw', isa => 'Int', default => 0);

sub speak {
    my $self = shift;
    return "My name is " . $self->name;
}

around 'speak' => sub {
    my $orig = shift;
    my $self = shift;
    return uc($self->$orig(@_));
};

__PACKAGE__->meta->make_immutable;
1;
"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let kinds = top_level_kinds(&ast);
    // Should contain Package, Use, Subroutine, and ExpressionStatements
    // (has/around/make_immutable/1; are all expression statements)
    assert!(
        kinds.contains(&"Package"),
        "expected Package node, got: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"Use"),
        "expected Use node, got: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"Subroutine"),
        "expected Subroutine node, got: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"ExpressionStatement"),
        "expected ExpressionStatement nodes for has/around/1;, got: {:?}",
        kinds
    );
}

#[test]
fn moo_class_with_types() {
    let code = r#"
package Dog;
use Moo;
use Types::Standard qw(Str Int);

has name => (is => 'ro', isa => Str, required => 1);
has age  => (is => 'rw', isa => Int, default => sub { 0 });

sub bark {
    my $self = shift;
    return "Woof! I'm " . $self->name;
}

1;
"#;
    assert_clean_parse(code);
}

#[test]
fn make_immutable_chain() {
    let code = "__PACKAGE__->meta->make_immutable;";
    assert_clean_parse(code);
    let ast = parse(code);
    let kinds = top_level_kinds(&ast);
    assert!(
        kinds.contains(&"ExpressionStatement"),
        "expected ExpressionStatement for chained method call, got: {:?}",
        kinds
    );
}
