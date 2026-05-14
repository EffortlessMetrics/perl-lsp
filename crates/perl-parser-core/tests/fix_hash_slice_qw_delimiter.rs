mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn hash_slice_qw_slash_delimiter() {
    assert_clean_parse("my @v = @hash{qw/a b c/};");
}

#[test]
fn hash_slice_qw_slash_delimiter_simple() {
    assert_clean_parse("my @v = @h{qw/x y/};");
}

#[test]
fn hash_slice_qw_slash_delimiter_keeps_following_block_statement() {
    let source = "sub verify { my ($got, $exists) = @params{qw/got exists/}; return 0; }";
    let sexp = parse(source).to_sexp();
    assert!(
        sexp.contains("(return (number 0))"),
        "expected return statement after hash-slice qw expression for source:\n{source}\n\nsexp:\n{sexp}",
    );
}

#[test]
fn delete_hash_slice_qw_slash_delimiter() {
    assert_clean_parse("delete @hash{qw/a b/};");
}

#[test]
fn hash_slice_qw_pipe_delimiter() {
    assert_clean_parse("my @v = @hash{qw|a b c|};");
}

#[test]
fn hash_slice_qw_in_assignment() {
    assert_clean_parse("my %copy; @copy{qw/foo bar/} = @orig{qw/foo bar/};");
}

#[test]
fn hash_slice_qw_parens_still_works() {
    assert_clean_parse("my @v = @hash{qw(a b c)};");
}

#[test]
fn hash_slice_string_keys_still_works() {
    assert_clean_parse(r#"my @v = @hash{'foo', 'bar'};"#);
}

#[test]
fn hash_slice_qw_single_word() {
    assert_clean_parse("my @v = @hash{qw/only/};");
}

#[test]
fn hash_slice_qw_bareword_key_still_works() {
    assert_clean_parse("my @v = @hash{qw, other};");
}

#[test]
fn delete_hash_slice_qw_pipe_delimiter() {
    assert_clean_parse("delete @h{qw|x y z|};");
}
