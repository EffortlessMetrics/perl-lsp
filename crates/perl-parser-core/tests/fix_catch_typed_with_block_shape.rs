mod cpan_test_helpers;
use cpan_test_helpers::assert_clean_parse;

#[test]
fn catch_typed_with_parses_cleanly() {
    assert_clean_parse(
        r#"try { $out = $search->command([qw(rev-parse --is-bare-repository --git-dir)], STDERR => 0); } catch Git::Error::Command with { throw Error::Simple('fatal'); };"#,
    );
}

#[test]
fn catch_typed_without_with_parses_cleanly() {
    assert_clean_parse(r#"try { die 'x' } catch Error::Simple { warn 'handled' };"#);
}
