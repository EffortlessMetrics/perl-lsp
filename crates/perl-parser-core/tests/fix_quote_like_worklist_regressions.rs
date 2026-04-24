//! Focused corpus-style regressions for quote-like, regex, and heredoc seams.
//! These cases mirror worklist categories that historically appeared in sweep buckets.

mod cpan_test_helpers;
use cpan_test_helpers::{assert_clean_parse, assert_has_error};

#[test]
fn subst_unusual_delimiter_pairs_clean_parse() {
    assert_clean_parse(r#"$x =~ s<foo><bar>g;"#);
    assert_clean_parse(r#"$x =~ s{foo}[bar]r;"#);
    assert_clean_parse(r#"$x =~ s!foo!bar!;"#);
}

#[test]
fn qr_paired_delimiters_clean_parse() {
    assert_clean_parse(r#"my $re1 = qr<foo\s+bar>i;"#);
    assert_clean_parse(r#"my $re2 = qr{(?:a|b){2}}x;"#);
}

#[test]
fn transliteration_paired_and_unpaired_clean_parse() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ tr[abc][xyz]d;"#);
    assert_clean_parse(r#"$x =~ y(abc)(xyz)c;"#);
}

#[test]
fn qw_comment_and_space_sensitive_forms_clean_parse() {
    assert_clean_parse("my @x = qw(foo # comment\n bar);");
    assert_clean_parse(r#"my @y = qw 'A B';"#);
    assert_clean_parse(r#"my @z = qw\n'X Y';"#);
}

#[test]
fn slash_ambiguity_after_builtins_and_operators_clean_parse() {
    assert_clean_parse(r#"my $n = time / 60;"#);
    assert_clean_parse(r#"grep /foo/, @list;"#);
    assert_clean_parse(r#"my $x = $a // $b;"#);
}

#[test]
fn heredoc_adjacent_to_phase_keyword_and_quote_like_clean_parse() {
    assert_clean_parse(
        "BEGIN { my $x = <<'TAG'; }\nbody\nTAG\n",
    );
    assert_clean_parse(
        "my $re = qr/foo/; my $doc = <<END;\nhello\nEND\n",
    );
}

#[test]
fn malformed_quote_like_inputs_remain_recoverable_with_specific_diagnostics() {
    assert_has_error(r#"$x =~ s/foo/bar/z;"#, "Invalid substitution modifier");
    assert_has_error(r#"$x =~ s<foo>;"#, "Missing replacement in substitution");
}
