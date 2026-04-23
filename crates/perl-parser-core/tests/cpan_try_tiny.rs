//! CPAN Pattern Tests: Try::Tiny

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn try_catch() {
    let code = r#"try { dangerous_op() } catch { warn "caught: $_" };"#;
    assert_clean_parse(code);
}

#[test]
fn try_catch_finally() {
    let code = r#"try { dangerous_op() } catch { warn "caught: $_" } finally { cleanup() };"#;
    assert_clean_parse(code);
}

#[test]
fn try_finally_no_catch() {
    let code = "try { work() } finally { cleanup() };";
    assert_clean_parse(code);
}

#[test]
fn try_catch_assigned_to_variable() {
    let code = r#"
my $result = try {
    might_fail();
} catch {
    warn "caught: $_";
    undef;
};
"#;
    assert_clean_parse(code);
}

#[test]
fn try_catch_finally_multiline() {
    let code = r#"
use Try::Tiny;
my $result = try {
    might_fail();
} catch {
    warn "caught: $_";
    undef;
} finally {
    cleanup();
};
"#;
    assert_clean_parse(code);
}

#[test]
fn nested_try_catch() {
    let code = r#"
try {
    try {
        inner_op();
    } catch {
        warn "inner: $_";
    };
} catch {
    warn "outer: $_";
};
"#;
    assert_clean_parse(code);
}
