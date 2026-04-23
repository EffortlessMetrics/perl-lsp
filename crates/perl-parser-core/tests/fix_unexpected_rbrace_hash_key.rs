mod cpan_test_helpers;
use cpan_test_helpers::*;

// Sub-bucket A: keyword as hash subscript key (arrow deref)
#[test]
fn test_arrow_hash_key_not() {
    assert_clean_parse(r#"my $x = $opts->{not};"#);
}

#[test]
fn test_arrow_hash_key_and() {
    assert_clean_parse(r#"my $x = $opts->{and};"#);
}

#[test]
fn test_arrow_hash_key_or() {
    assert_clean_parse(r#"my $x = $opts->{or};"#);
}

#[test]
fn test_arrow_hash_key_xor() {
    assert_clean_parse(r#"my $x = $opts->{xor};"#);
}

#[test]
fn test_arrow_hash_key_do() {
    assert_clean_parse(r#"my $x = $opts->{do};"#);
}

#[test]
fn test_arrow_hash_key_eval() {
    assert_clean_parse(r#"my $x = $opts->{eval};"#);
}

// Assignment through keyword hash key
#[test]
fn test_arrow_hash_key_not_assign() {
    assert_clean_parse(r#"$opts->{not} = \%not_want;"#);
}

// Bare hash subscript (no arrow)
#[test]
fn test_bare_hash_key_not() {
    assert_clean_parse(r#"my $x = $h{not};"#);
}

#[test]
fn test_bare_hash_key_and() {
    assert_clean_parse(r#"my $x = $h{and};"#);
}

#[test]
fn test_bare_hash_key_or() {
    assert_clean_parse(r#"my $x = $h{or};"#);
}

#[test]
fn test_bare_hash_key_xor() {
    assert_clean_parse(r#"my $x = $h{xor};"#);
}

#[test]
fn test_bare_hash_key_do() {
    assert_clean_parse(r#"my $x = $h{do};"#);
}

#[test]
fn test_bare_hash_key_eval() {
    assert_clean_parse(r#"my $x = $h{eval};"#);
}

// Chained deref with keyword key
#[test]
fn test_chained_hash_key_not() {
    assert_clean_parse(r#"my $x = $obj->{opts}->{not};"#);
}

// Keyword key in complex expression
#[test]
fn test_keyword_key_in_condition() {
    assert_clean_parse(r#"if ($opts->{not}) { print "negated" }"#);
}

// Real-world pattern from Exporter::Tiny
#[test]
fn test_exporter_tiny_pattern() {
    assert_clean_parse(r#"my %not_want; $global_opts->{not} = \%not_want;"#);
}

// Edge: keyword followed by expression (not just })
// These should still parse as operators, not identifiers
#[test]
fn test_not_as_operator_in_hash() {
    assert_clean_parse(r#"my $x = $h{not $flag};"#);
}

// Regression: regular hash keys still work
#[test]
fn test_regular_hash_key_still_works() {
    assert_clean_parse(r#"my $x = $opts->{regular_key};"#);
}

// Regression: not as operator still works
#[test]
fn test_not_as_operator_still_works() {
    assert_clean_parse(r#"my $x = not $y;"#);
}
