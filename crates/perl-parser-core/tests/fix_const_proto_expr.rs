//! Tests for issue #2387: unexpected_token_in_expr — constant subs with ()
//! prototype, versioned packages, and Moo/Moose DSL bare calls.
//!
//! Root cause: `bless []` and similar calls where a builtin function is
//! followed by `[` were parsed as array subscript access instead of a
//! function call with an anonymous arrayref argument.

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// bless with an empty anonymous arrayref as first arg.
/// This is the most common OO pattern: `my $self = bless [], $class`.
#[test]
fn test_bless_empty_arrayref() {
    let source = r#"my $g = bless [];"#;
    assert_clean_parse(source);
}

/// bless with anonymous arrayref and explicit package name.
#[test]
fn test_bless_arrayref_with_class() {
    let source = r#"my $g = bless [], 'Foo';"#;
    assert_clean_parse(source);
}

/// bless with arrayref and ref $class || $class idiom — common in Graph.pm
#[test]
fn test_bless_arrayref_ref_or_class() {
    let source = r#"my $g = bless [], ref $class || $class;"#;
    assert_clean_parse(source);
}

/// Full Graph.pm-style pattern: constant sub declarations then bless []
#[test]
fn test_graph_pm_const_subs_with_bless_arrayref() {
    let source = r#"sub _F () { 0 }
sub _G () { 1 }
sub _V () { 2 }

my $class = 'Graph';
my $g = bless [], ref $class || $class;
$g->[ _F ] = 0;
$g->[ _V ] = {};"#;
    assert_clean_parse(source);
}

/// Constant sub with empty () prototype used as expression primary.
#[test]
fn test_const_sub_empty_proto_as_expr_primary() {
    let source = r#"sub _F () { 0 }; my $x = _F + 1;"#;
    assert_clean_parse(source);
}

/// Constant sub used as the sole RHS in assignment.
#[test]
fn test_const_sub_empty_proto_rhs() {
    let source = r#"sub EMPTY () { }; my $y = EMPTY;"#;
    assert_clean_parse(source);
}

/// Versioned package declaration should not cascade errors.
#[test]
fn test_versioned_package_no_cascade() {
    let source = "package App::Cmd::Command::commands 0.340;\nsub execute { return 1 }";
    assert_clean_parse(source);
}

/// Moo/Moose extends without a preceding use Moose.
#[test]
fn test_moo_extends_bare_no_use() {
    let source = r#"extends 'CHI::Driver';"#;
    assert_clean_parse(source);
}

/// CHI::Driver pattern: extends in a package body.
#[test]
fn test_chi_driver_extends_pattern() {
    let source = r#"package CHI::Driver::File;
extends 'CHI::Driver';
sub BUILD { my ($self) = @_; }
1;"#;
    assert_clean_parse(source);
}

/// use Exporter with version number.
#[test]
fn test_use_exporter_with_version() {
    let source = r#"use Exporter 5.57 'import';"#;
    assert_clean_parse(source);
}
