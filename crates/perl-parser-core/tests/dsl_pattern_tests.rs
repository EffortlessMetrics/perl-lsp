//! DSL pattern tests for Moose/Moo/Dancer2/modern Perl
//!
//! These tests exercise the parser against real-world Perl DSL patterns
//! from the most popular CPAN modules.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Moose patterns ===

#[test]
fn moose_has_with_quoted_name() {
    assert_clean_parse(r#"use Moose; has 'name' => (is => 'ro', isa => 'Str', required => 1);"#);
}

#[test]
fn moose_extends_and_with() {
    assert_clean_parse(r#"use Moose; extends 'Animal'; with 'Role::Printable';"#);
}

#[test]
fn moose_before_modifier() {
    assert_clean_parse(r#"use Moose; before 'greet' => sub { warn 'about to greet' };"#);
}

#[test]
fn moose_after_modifier() {
    assert_clean_parse(r#"use Moose; after 'greet' => sub { warn 'greeted' };"#);
}

#[test]
fn moose_around_modifier() {
    assert_clean_parse(
        r#"use Moose; around 'greet' => sub { my ($orig, $self, @args) = @_; $self->$orig(@args) };"#,
    );
}

#[test]
fn moose_no_moose_make_immutable() {
    assert_clean_parse(r#"no Moose; __PACKAGE__->meta->make_immutable;"#);
}

// === Moo patterns ===

#[test]
fn moo_has_bare_name() {
    assert_clean_parse(r#"use Moo; has name => (is => 'ro', required => 1);"#);
}

#[test]
fn moo_has_with_default_sub() {
    assert_clean_parse(r#"use Moo; has items => (is => 'rw', default => sub { [] });"#);
}

#[test]
fn moo_role_requires() {
    assert_clean_parse(r#"use Moo::Role; requires 'name';"#);
}

// === DBIx::Class patterns ===

#[test]
fn dbic_has_many() {
    assert_clean_parse(
        r#"__PACKAGE__->has_many('books', 'MyApp::Schema::Result::Book', 'author_id');"#,
    );
}

#[test]
fn dbic_belongs_to() {
    assert_clean_parse(r#"__PACKAGE__->belongs_to('author', 'MyApp::Schema::Result::Author');"#);
}

#[test]
fn dbic_add_columns() {
    assert_clean_parse(r#"__PACKAGE__->add_columns(qw/id name email/);"#);
}

#[test]
fn dbic_set_primary_key() {
    assert_clean_parse(r#"__PACKAGE__->set_primary_key('id');"#);
}

// === Dancer2/Mojolicious route DSL ===

#[test]
fn dancer_get_route() {
    assert_clean_parse(r#"get '/hello' => sub { return 'Hello World' };"#);
}

#[test]
fn dancer_post_route() {
    assert_clean_parse(r#"post '/api/data' => sub { my $data = request->body; return $data };"#);
}

#[test]
fn dancer_any_route() {
    assert_clean_parse(r#"any ['get', 'post'] => '/form' => sub { template 'form' };"#);
}

// === Try::Tiny ===

#[test]
fn try_catch_finally() {
    assert_clean_parse(r#"try { die 'oops' } catch { warn "caught: $_" } finally { cleanup() };"#);
}

// === Modern Perl (v5.36+ signatures) ===

#[test]
fn sub_with_signature() {
    assert_clean_parse(r#"sub add($x, $y) { return $x + $y }"#);
}

#[test]
fn sub_with_default_param() {
    assert_clean_parse(r#"sub greet($name = 'World') { say "Hello, $name" }"#);
}

// === v5.38 class syntax ===

#[test]
fn class_with_fields_and_method() {
    assert_clean_parse(
        r#"use feature 'class'; class Point { field $x :param :reader; field $y :param :reader; method magnitude() { sqrt($x**2 + $y**2) } }"#,
    );
}

// === Postfix dereference (v5.20+) ===

#[test]
fn postfix_deref_array() {
    assert_clean_parse(r#"my @items = $ref->@*;"#);
}

#[test]
fn postfix_deref_hash_slice() {
    assert_clean_parse(r#"my %subset = $href->%{qw(a b c)};"#);
}
