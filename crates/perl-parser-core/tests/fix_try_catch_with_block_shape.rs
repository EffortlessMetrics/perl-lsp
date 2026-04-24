mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;

#[test]
fn catch_type_with_block_parses_clean() {
    assert_clean_parse(
        r#"
try { 1; }
catch Git::Error::Command with {
    1;
}
"#,
    );
}

#[test]
fn catch_type_with_block_and_finally_parses_clean() {
    assert_clean_parse(
        r#"
try { 1; }
catch My::Error with {
    1;
}
finally {
    1;
}
"#,
    );
}
