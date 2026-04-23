mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #2184: parse_no had a restrictive import item parser that
// rejected valid Perl. The fix applies the same depth-tracking slurp loop
// that parse_use uses to parse parenthesised argument lists.

#[test]
fn test_no_module_with_scalar_var_in_parens() {
    // `no MyModule ($var, 0)` — sigil-prefixed variable rejected by old code
    let source = r#"no MyModule ($var, 0);"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_overload_with_backslash_ref() {
    // `no overload ('==' => \&func)` — backslash ref rejected by old code
    let source = r#"no overload ('==' => \&func);"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_overload_multiple_ops() {
    let source = r#"no overload '+', '-', '*';"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_feature_nested_parens() {
    // Nested parens inside the arg list must be depth-tracked
    let source = r#"no feature ('say', 'state');"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_warnings_single_string() {
    let source = r#"no warnings 'all';"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_strict_refs() {
    let source = r#"no strict 'refs';"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_warnings_multiple_args_in_parens() {
    let source = r#"no warnings ('experimental', 'uninitialized');"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_overload_fat_arrow_with_coderef() {
    // coderef value after fat arrow should be slurped without error
    let source = r#"no overload '""' => \&to_string;"#;
    assert_clean_parse(source);
}
