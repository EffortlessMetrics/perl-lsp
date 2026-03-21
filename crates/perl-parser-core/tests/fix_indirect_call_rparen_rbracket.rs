mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_indirect_call_last_in_parens() {
    assert_clean_parse(r#"my $x = (send $obj "msg");"#);
}

#[test]
fn test_indirect_call_last_in_arrayref() {
    assert_clean_parse(r#"my $a = [send $obj "msg"];"#);
}

#[test]
fn test_indirect_call_multi_arg_no_regression() {
    // Ensure adding the terminators does not break multi-arg indirect calls
    assert_clean_parse(r#"send $socket $data;"#);
}

#[test]
fn test_known_builtin_still_works_in_parens() {
    // Known builtins were already fixed; verify no regression
    assert_clean_parse(r#"my $x = (delete $h{key});"#);
    assert_clean_parse(r#"my $n = (scalar @arr);"#);
}
