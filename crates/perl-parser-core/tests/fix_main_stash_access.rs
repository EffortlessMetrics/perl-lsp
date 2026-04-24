mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn main_stash_subscript() {
    assert_clean_parse(r#"my $x = $::{foo};"#);
}

#[test]
fn main_stash_exists() {
    assert_clean_parse(r#"exists($::{$pack})"#);
}

#[test]
fn main_stash_for_loop() {
    assert_clean_parse(r#"for ($::{$pack}) { 1; }"#);
}

#[test]
fn main_stash_in_unless() {
    assert_clean_parse(r#"return unless exists($::{$pack});"#);
}

#[test]
fn main_stash_nested() {
    // $::{Foo::}{bar} - nested stash lookup
    assert_clean_parse(r#"my $sym = $::{'Foo::'}{'bar'};"#);
}

// --- regression guard: verify :: addition to COMPOUND_SECOND_CHARS does not
//     break operators that merely contain ':' as a character ---

#[test]
fn ternary_colon_unaffected() {
    // The ':' in a ternary '?:' must not be consumed as part of '::'
    assert_clean_parse(r#"my $x = $flag ? "yes" : "no";"#);
}

#[test]
fn ternary_with_stash_in_condition() {
    // Mix $:: stash access with a surrounding ternary
    assert_clean_parse(r#"my $x = exists($::{$k}) ? $::{$k} : undef;"#);
}

#[test]
fn label_colon_unaffected() {
    // Statement labels use a single ':' (Colon token), not '::' (DoubleColon)
    assert_clean_parse(r#"OUTER: for my $i (1..10) { last OUTER if $i == 5; }"#);
}

#[test]
fn double_colon_in_package_variable_unaffected() {
    // $Foo::bar package-qualified variable still parses correctly
    assert_clean_parse(r#"my $x = $Foo::bar;"#);
}

#[test]
fn main_stash_hash_sigil() {
    // %:: is the main symbol table as a hash — should parse without error
    assert_clean_parse(r#"my %stash = %::;"#);
}

#[test]
fn main_stash_array_sigil() {
    // @:: is the list of symbol names in the main package
    assert_clean_parse(r#"my @syms = @::;"#);
}

#[test]
fn main_stash_symbol_in_parenthesized_condition() {
    // Unicode::Collate pattern: `$::IS_ASCII` in a paren condition.
    assert_clean_parse(r#"my $x = ($::IS_ASCII || $] < 5.008) ? 1 : 0;"#);
}
