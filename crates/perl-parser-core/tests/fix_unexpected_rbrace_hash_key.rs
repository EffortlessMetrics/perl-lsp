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

#[test]
fn test_bare_hash_key_cmp() {
    assert_clean_parse(r#"my %opt; $opt{cmp} ||= '=';"#);
}

#[test]
fn test_arrow_hash_key_cmp() {
    assert_clean_parse(r#"my $x = $opts->{cmp};"#);
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

// Real-world parser-corpus patterns: keyword-like builtins and modern keywords
// are valid unquoted hash keys when a hash subscript delimiter follows them.
#[test]
fn test_bare_hash_key_tie() {
    assert_clean_parse(r#"@{$bits{tie}}{3,2,1,0} = ($bf[4], $bf[4], $bf[4], $bf[4]);"#);
}

#[test]
fn test_bare_hash_key_untie() {
    assert_clean_parse(r#"$bits{untie}{0} = $bf[0];"#);
}

#[test]
fn test_arrow_hash_key_defer_assign() {
    assert_clean_parse(r#"$self->{defer} = 1;"#);
}

#[test]
fn test_arrow_hash_key_defer_condition() {
    assert_clean_parse(r#"if ($self->{autodeferring} && $self->{defer}) { $self->{defer} = 0; }"#);
}

#[test]
fn test_hash_slice_keyword_keys() {
    assert_clean_parse(r#"my @vals = @h{defer, try, eval, tie, untie};"#);
}
