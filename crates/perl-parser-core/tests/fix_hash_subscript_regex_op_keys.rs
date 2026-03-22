mod cpan_test_helpers;
use cpan_test_helpers::*;

// Bug A: direct hash subscript with regex-operator name (no arrow)
// e.g. $h{m} = 1  — the lexer was treating `m}` as start of m}...{ regex
#[test]
fn test_hash_subscript_key_m() {
    assert_clean_parse("$h{m} = 1;");
}

#[test]
fn test_hash_subscript_key_s() {
    assert_clean_parse("$h{s} = 1;");
}

#[test]
fn test_hash_subscript_key_q() {
    assert_clean_parse("$h{q} = 1;");
}

#[test]
fn test_hash_subscript_key_qq() {
    assert_clean_parse("$h{qq} = 1;");
}

#[test]
fn test_hash_subscript_key_qw() {
    assert_clean_parse("$h{qw} = 1;");
}

#[test]
fn test_hash_subscript_key_qr() {
    assert_clean_parse("$h{qr} = 1;");
}

#[test]
fn test_hash_subscript_key_qx() {
    assert_clean_parse("$h{qx} = 1;");
}

#[test]
fn test_hash_subscript_key_tr() {
    assert_clean_parse("$h{tr} = 1;");
}

#[test]
fn test_hash_subscript_key_y() {
    assert_clean_parse("$h{y} = 1;");
}

// Bug B: arrow hash subscript with regex-operator name
// e.g. $ref->{m}  — the parser's parse_primary was treating `m` as a quote-op
#[test]
fn test_arrow_hash_subscript_key_m() {
    assert_clean_parse("$ref->{m};");
}

#[test]
fn test_arrow_hash_subscript_key_s() {
    assert_clean_parse("$ref->{s};");
}

#[test]
fn test_arrow_hash_subscript_key_q() {
    assert_clean_parse("$ref->{q};");
}

#[test]
fn test_arrow_hash_subscript_key_tr() {
    assert_clean_parse("$ref->{tr};");
}

// Real-world pattern from Perl::Tidy::Formatter — multiple regex-op keys in sequence
#[test]
fn test_multiple_regex_op_hash_keys() {
    assert_clean_parse(
        "$left_bond_strength{m} = 1;\n\
         $left_bond_strength{q} = 1;\n\
         $left_bond_strength{qq} = 1;\n\
         $left_bond_strength{qw} = 1;\n\
         $left_bond_strength{qr} = 1;\n\
         $left_bond_strength{y} = 1;",
    );
}

// Regex-op key inside a larger expression
#[test]
fn test_regex_op_key_in_condition() {
    assert_clean_parse("if ($h{m} == 1) { print 'yes'; }");
}

// Nested subscript: inner {m} is subscript of $other
#[test]
fn test_nested_hash_subscript_regex_key() {
    assert_clean_parse("my $x = $h{ $other{m} };");
}

// Regression: hash slice with qw (different parse path — must not be affected)
#[test]
fn test_hash_slice_with_qw_unaffected() {
    assert_clean_parse(r#"my @vals = @h{qw(foo bar)};"#);
}

// Regression: regular hash keys still work
#[test]
fn test_regular_hash_key_still_works() {
    assert_clean_parse(r#"my $x = $h{regular_key};"#);
}

// Regression: fat-arrow autoquoting of regex-op name still works (m and s)
#[test]
fn test_fat_arrow_with_regex_op_name() {
    assert_clean_parse(r#"my %h = (m => 1, s => 2);"#);
}

// Bug C (discovered in review): hash SLICE with multiple regex-op keys
// After Bug A is fixed for the first key, subsequent keys in a slice lost
// the after_hash_brace protection because the flag was cleared per-token.
// The depth-tracking fix (hash_brace_depth) and slice-aware
// parse_hash_subscript_key keep all keys in a slice treated as barewords.
#[test]
fn test_hash_slice_two_regex_op_keys() {
    assert_clean_parse(r#"my @v = @h{m, s};"#);
}

#[test]
fn test_hash_slice_three_regex_op_keys() {
    assert_clean_parse(r#"my @v = @h{q, m, s};"#);
}

#[test]
fn test_hash_slice_tr_and_y() {
    assert_clean_parse(r#"my @v = @h{tr, y};"#);
}

// Regression: qw(list) inside a hash subscript must NOT be treated as a bareword
// (it has `(` after it, not `}` or `,`, so the normal qw parse path applies)
#[test]
fn test_hash_subscript_qw_list_unaffected() {
    assert_clean_parse(r#"my @v = %Pkg::Hash{qw(a b)};"#);
}

// Arrow subscript: missing Bug B coverage for y and remaining q* variants
#[test]
fn test_arrow_hash_subscript_key_y() {
    assert_clean_parse("$ref->{y};");
}

#[test]
fn test_arrow_hash_subscript_key_qq() {
    assert_clean_parse("$ref->{qq};");
}

#[test]
fn test_arrow_hash_subscript_key_qr() {
    assert_clean_parse("$ref->{qr};");
}

#[test]
fn test_arrow_hash_subscript_key_qx() {
    assert_clean_parse("$ref->{qx};");
}

#[test]
fn test_arrow_hash_subscript_key_qw() {
    assert_clean_parse("$ref->{qw};");
}
