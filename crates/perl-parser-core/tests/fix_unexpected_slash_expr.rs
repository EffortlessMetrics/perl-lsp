/// Tests for issue #2401: unexpected_slash_expr — regex after complex LHS
///
/// Covers cases where `/pattern/` appears:
///   1. After a block statement (e.g. `if (...) { } /re/`)
///   2. After `=~` binding operator on complex LHS (method call result)
///   3. Bare regex at statement start immediately after `{ }` ends a block
mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── After =~ with simple variable (baseline — must stay passing) ────────────

#[test]
fn regex_after_binding_op_simple_var() {
    assert_clean_parse("$str =~ /pattern/;");
}

// ── After =~ with method call result ─────────────────────────────────────────

#[test]
fn regex_after_binding_op_method_call() {
    assert_clean_parse("if ($obj->method() =~ /pattern/) { 1; }");
}

// ── Bare /re/ at statement start following a block statement ─────────────────

#[test]
fn bare_regex_at_stmt_start_after_block() {
    assert_clean_parse("if ($x) { 1; }\n/^prefix/ and do_thing();");
}

// ── Bare /re/ at statement start following a bare brace block ────────────────

#[test]
fn bare_regex_at_stmt_start_after_bare_block() {
    assert_clean_parse("{ 1; }\n/^prefix/ and do_thing();");
}

// ── Full CPAN-style pattern: conditional with binding op ─────────────────────

#[test]
fn regex_in_if_with_binding_op() {
    assert_clean_parse("if ($x =~ /re/) { }");
}

// ── Chained: multiple block stmts followed by bare regex ─────────────────────

#[test]
fn bare_regex_after_multiple_blocks() {
    assert_clean_parse("if ($a) { 1; }\nwhile ($b) { 2; }\n/^prefix/ and do_thing();");
}

// ── Bare /re/ at file start (baseline — must stay passing) ───────────────────

#[test]
fn bare_regex_at_file_start() {
    assert_clean_parse("/^prefix/ and do_thing();");
}

// ── !~ binding operator should also produce clean parse ──────────────────────

#[test]
fn regex_after_not_binding_op() {
    assert_clean_parse("if ($str !~ /bad/) { 1; }");
}

#[test]
fn bare_regex_rhs_of_word_and_in_grep_block() {
    assert_clean_parse(
        r#"grep {
    substr($_, -2, 2, '') eq '::'
    and /$RE_IDENTIFIER/o
} keys %{"${name}::"};"#,
    );
}

#[test]
fn bare_regex_after_unary_not_in_grep_block() {
    assert_clean_parse(r#"grep { ! /^\_/ } @methodlist;"#);
}

#[test]
fn bare_regex_rhs_of_word_and_in_nested_condition() {
    assert_clean_parse(
        r#"grep {
    ($opts{include_main} and /^\Q$basename\E\.orig\.tar\.$opts{extension}$/) or
    ($opts{include_supplementary} and /^\Q$basename\E\.orig-[[:alnum:]-]+\.tar\.$opts{extension}$/)
} readdir($dir_dh);"#,
    );
}
