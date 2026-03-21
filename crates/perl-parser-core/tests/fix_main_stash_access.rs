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
