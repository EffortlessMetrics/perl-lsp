mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #2394: typeglob assignment with sub bodies fail to parse cleanly.
// Patterns from MOOSE, CLASS-ACCESSOR, CATALYST corpus files.

#[test]
fn test_typeglob_assign_sub_simple() {
    // *method = sub { ... };  — from Moose::Meta::Class
    let source = r#"*foo = sub { 1 };"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_reference() {
    // *foo = \&other;  — glob aliasing via reference
    let source = r#"*foo = \&other;"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_prototype() {
    // *GLOB = sub () { ... };  — with empty prototype, from Catalyst
    let source = r#"*GLOB = sub () { 1 };"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_dynamic_simple() {
    // *{$name} = sub { ... };  — dynamic name
    let source = r#"*{$name} = sub { return $_[0] };"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_dynamic_concat() {
    // *{$pkg . '::' . $name} = sub { ... };  — from Class::Accessor
    let source = r#"*{$pkg . '::' . $name} = sub { $_[0] };"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_qualified() {
    // *{'UNIVERSAL::isa'} = sub () { ... };  — string key, from Catalyst
    let source = r#"*{'UNIVERSAL::isa'} = sub () { ... };"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_body_multiline() {
    // Multi-statement sub body
    let source = r#"
*install = sub {
    my ($self, $name) = @_;
    $self->{$name} = 1;
    return $self;
};
"#;
    assert_clean_parse(source);
}

#[test]
fn test_typeglob_assign_sub_qualified_name() {
    // *Foo::bar = sub { ... };  — qualified typeglob
    let source = r#"*Foo::bar = sub { my $x = 1; $x };"#;
    assert_clean_parse(source);
}
