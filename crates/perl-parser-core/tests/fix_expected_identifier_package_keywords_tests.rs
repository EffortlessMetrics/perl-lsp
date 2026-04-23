mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #2150: expected_identifier — keyword tokens as package names
// Corpus files: if.pm, mro.pm, Class/C3/next.pm

// === Core bug: keyword as first token in package name ===

#[test]
fn test_package_keyword_if() {
    // Source: /usr/share/perl/5.38/if.pm line 1
    let source = r#"package if;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_keyword_next() {
    // Source: mro.pm line 24-25
    let source = r#"package next;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_keyword_next_after_comment() {
    // Source: mro.pm — package name on next line after comment
    let source = "package # hide me from PAUSE\n    next;";
    assert_clean_parse(source);
}

// === Additional keyword-as-package-name patterns ===

#[test]
fn test_package_keyword_unless() {
    let source = r#"package unless;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_keyword_while() {
    let source = r#"package while;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_keyword_for() {
    let source = r#"package for;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_keyword_do() {
    let source = r#"package do;"#;
    assert_clean_parse(source);
}

// === Keyword after :: separator ===

#[test]
fn test_package_compound_keyword_suffix() {
    // e.g., Foo::if — keyword after ::
    let source = r#"package Foo::if;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_compound_keyword_prefix() {
    // e.g., next::Foo — keyword before ::
    let source = r#"package next::Foo;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_maybe_next() {
    // Source: mro.pm — maybe::next with comment
    let source = "package # hide me from PAUSE\n    maybe::next;";
    assert_clean_parse(source);
}

#[test]
fn test_package_maybe_next_simple() {
    // maybe::next without comment
    let source = r#"package maybe::next;"#;
    assert_clean_parse(source);
}

// === Regression: normal packages still work ===

#[test]
fn test_package_normal_still_works() {
    let source = r#"package Normal::Package;"#;
    assert_clean_parse(source);
}

#[test]
fn test_package_simple_identifier() {
    let source = r#"package Foo;"#;
    assert_clean_parse(source);
}

// === Full file patterns from corpus ===

#[test]
fn test_if_pm_full() {
    let source = r#"package if;
use strict;
our $VERSION = '0.0610';

sub work {
  my $method = shift() ? 'import' : 'unimport';
  return unless shift;
  my $p = $_[0];
  require $p;
}

sub import   { shift; unshift @_, 1; goto &work }
sub unimport { shift; unshift @_, 0; goto &work }

1;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mro_pm_packages() {
    let source = r#"package mro;
use strict;
use warnings;

our $VERSION = '1.28';

sub import {
    mro::set_mro(scalar(caller), $_[1]) if $_[1];
}

package # hide me from PAUSE
    next;

sub can { mro::_nextcan($_[0], 0) }

sub method {
    my $method = mro::_nextcan($_[0], 1);
    goto &$method;
}

package # hide me from PAUSE
    maybe::next;

sub method {
    my $method = mro::_nextcan($_[0], 0);
    goto &$method if defined $method;
    return;
}

1;
"#;
    assert_clean_parse(source);
}
