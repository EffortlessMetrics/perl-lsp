//! Hash/block literal disambiguation tests.
//!
//! Perl uses `{}` for both anonymous hash references and code blocks.
//! The parser must distinguish between them based on context. This file
//! covers hash literals, hash references, block expressions, and the
//! ambiguous edges where the two overlap.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Hash literals ──────────────────────────────────────────────────

#[test]
fn hash_literal_fat_comma_pairs() {
    assert_clean_parse("my %h = (a => 1, b => 2);");
}

#[test]
fn hash_ref_fat_comma_pairs() {
    assert_clean_parse("my $href = { a => 1, b => 2 };");
}

#[test]
fn empty_hash_ref() {
    assert_clean_parse("my $href = {};");
}

#[test]
fn hash_ref_assignment_via_arrow() {
    assert_clean_parse("$obj->{config} = { debug => 1, level => 2 };");
}

// ── Nested hashes ──────────────────────────────────────────────────

#[test]
fn nested_hash_ref() {
    assert_clean_parse("my $h = { a => { b => 1 } };");
}

#[test]
fn hash_with_nested_hash_ref_value() {
    assert_clean_parse("my %h = (key => { nested => 'val' });");
}

// ── Block expressions ──────────────────────────────────────────────

#[test]
fn do_block() {
    assert_clean_parse("do { my $x = 1; $x + 2 };");
}

#[test]
fn eval_block() {
    assert_clean_parse("eval { die 'test' };");
}

#[test]
fn sort_block() {
    assert_clean_parse("sort { $a <=> $b } @list;");
}

// ── Ambiguous cases ────────────────────────────────────────────────

#[test]
fn block_with_statement_inside() {
    // Braces after assignment with a statement inside: code block, not hash.
    assert_clean_parse("my $r = { print 'hello' };");
}

#[test]
fn unary_plus_forces_hash_ref() {
    // Leading `+` disambiguates in favour of an anonymous hash reference.
    assert_clean_parse("+{ key => 'val' };");
}
