//! CPAN Pattern Tests: Object-Oriented Patterns

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

#[test]
fn basic_constructor() {
    let code = r#"
sub new {
    my ($class, %args) = @_;
    my $self = bless {}, $class;
    $self->{name} = $args{name};
    return $self;
}
"#;
    assert_clean_parse(code);
}

#[test]
fn constructor_with_defaults() {
    let code = r#"
sub new {
    my ($class, %args) = @_;
    my $self = bless {
        name    => $args{name} || 'unknown',
        verbose => $args{verbose} || 0,
        _cache  => {},
    }, $class;
    return $self;
}
"#;
    assert_clean_parse(code);
}

#[test]
fn method_call_chain() {
    let code = "$obj->method1->method2->method3;";
    assert_clean_parse(code);
    let ast = parse(code);
    // Should be a chain of MethodCall nodes
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].kind.kind_name(), "ExpressionStatement");
    }
}

#[test]
fn class_method_call() {
    let code = "my $obj = Foo::Bar->new(name => 'test', id => 42);";
    assert_clean_parse(code);
}

#[test]
fn isa_check() {
    let code = "if (ref($obj) && $obj->isa('Foo::Bar')) { $obj->do_thing() }";
    assert_clean_parse(code);
}

#[test]
fn can_check() {
    let code = "if ($obj->can('process')) { $obj->process(@args) }";
    assert_clean_parse(code);
}

#[test]
fn autoload_pattern() {
    let code = r#"
sub AUTOLOAD {
    my $self = shift;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';
}
"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let kinds = top_level_kinds(&ast);
    assert!(kinds.contains(&"Subroutine"), "expected Subroutine for AUTOLOAD");
}

#[test]
fn destroy_method() {
    let code = r#"
sub DESTROY {
    my $self = shift;
    close $self->{fh} if $self->{fh};
}
"#;
    assert_clean_parse(code);
}

#[test]
fn multiple_inheritance() {
    let code = r#"
package Child;
use parent qw(Mother Father);
1;
"#;
    assert_clean_parse(code);
}

#[test]
fn package_block_form() {
    let code = r#"
package My::Module {
    use strict;
    use warnings;
    sub new { bless {}, shift }
    1;
}
"#;
    assert_clean_parse(code);
}

#[test]
fn builder_pattern() {
    let code = r#"
my $query = SQL::Builder->new
    ->select('id', 'name')
    ->from('users')
    ->where('active = ?', 1)
    ->order_by('name')
    ->limit(10);
"#;
    assert_clean_parse(code);
}
