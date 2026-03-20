mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_begin_then_if() {
    // Minimal reproducer for unclosed_brace_semicolon
    assert_clean_parse(r#"BEGIN { 1; } if (1) { print "ok"; }"#);
}

#[test]
fn test_check_then_if() {
    // Does CHECK { } have the same problem?
    assert_clean_parse(r#"CHECK { 1; } if (1) { print "ok"; }"#);
}

#[test]
fn test_end_then_if() {
    assert_clean_parse(r#"END { 1; } if (1) { print "ok"; }"#);
}

#[test]
fn test_init_then_if() {
    assert_clean_parse(r#"INIT { 1; } if (1) { print "ok"; }"#);
}

#[test]
fn test_unitcheck_then_if() {
    assert_clean_parse(r#"UNITCHECK { 1; } if (1) { print "ok"; }"#);
}

#[test]
fn test_begin_then_while() {
    assert_clean_parse(r#"BEGIN { 1; } while (1) { last; }"#);
}

#[test]
fn test_begin_then_sub() {
    assert_clean_parse(r#"BEGIN { 1; } sub foo { 1; }"#);
}
