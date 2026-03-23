//! Tests for issue #2895: qw with space before single-quote (or double-quote)
//! delimiter fails inside function calls.
//!
//! Root Cause: Commit 7741fcd8 (#2815) introduced a restriction that only
//! paired delimiters ({, [, (, <) are accepted after whitespace. This was
//! over-broad — it also blocked ' and " which are unambiguous for all
//! quote operators except `s` (where `-s 'filename'` is a filetest).
//!
//! Fix: `crates/perl-lexer/src/lib.rs` — add `is_quote_char` predicate in
//! `try_identifier_or_keyword` Path 2, guarded with `op != "s"`.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Primary failing case: qw with space before single-quote in a call ────────

/// foo(qw 'A B') — the primary regression: qw + space + single-quote inside
/// a function call argument list. Without the fix, the lexer treats `qw` as
/// an identifier and `'A B')` as a single-quoted string that consumes the `)`.
#[test]
fn qw_space_squote_in_call() {
    assert_clean_parse(r#"foo(qw 'A B');"#);
}

/// $_[0]->_cached_tmpdir(qw 'TMPDIR TEMP TMP') — the exact OS2.pm pattern
/// that generates unexpected_rparen_expr cascade failures.
#[test]
fn qw_space_squote_method_call() {
    assert_clean_parse(r#"$_[0]->_cached_tmpdir(qw 'TMPDIR TEMP TMP');"#);
}

/// Two qw with space+single-quote in the same call — ensures both are
/// lexed correctly, not just the first one.
#[test]
fn qw_space_squote_double_in_call() {
    assert_clean_parse(r#"f(qw 'A B', qw 'C D');"#);
}

// ── Statement-level (already works — non-regression baseline) ────────────────

/// my @x = qw 'A B' — at statement level. This worked before; must remain
/// working after the fix.
#[test]
fn qw_space_squote_statement() {
    assert_clean_parse(r#"my @x = qw 'A B';"#);
}

// ── Double-quote delimiter variant ───────────────────────────────────────────

/// my @x = qw "A B" — double-quote delimiter with space. The fix must cover
/// " in addition to '.
#[test]
fn qw_space_dquote_statement() {
    assert_clean_parse("my @x = qw \"A B\";");
}

// ── Newline separator ────────────────────────────────────────────────────────

/// my @x = qw\n'A B' — newline between qw and delimiter. peek_nonspace_and_following
/// already skips newlines; this confirms the fix handles that path too.
#[test]
fn qw_newline_squote_statement() {
    assert_clean_parse("my @x = qw\n'A B';");
}

// ── Other q-family operators with space + single-quote ───────────────────────

/// my $x = q 'hello world' — single-char q operator, not s. Must be allowed.
#[test]
fn q_space_squote_statement() {
    assert_clean_parse(r#"my $x = q 'hello world';"#);
}

/// my $x = qq 'hello $name' — qq operator with space + single-quote delimiter.
#[test]
fn qq_space_squote_statement() {
    assert_clean_parse(r#"my $x = qq 'hello world';"#);
}

/// if ($x =~ m 'foo') { 1 } — m operator with space + single-quote delimiter.
#[test]
fn m_space_squote_in_condition() {
    assert_clean_parse(r#"if ($x =~ m 'foo') { 1 }"#);
}

// ── No-space baseline: must be unchanged by the fix ──────────────────────────

/// foo(qw'A B') — adjacent single-quote, no space. Must continue to work.
#[test]
fn qw_nospace_squote_unchanged() {
    assert_clean_parse(r#"foo(qw'A B');"#);
}

/// foo(qw(A B)) — parenthesis delimiter, no space. Must continue to work.
#[test]
fn qw_parens_unchanged() {
    assert_clean_parse(r#"foo(qw(A B));"#);
}

// ── Other operators: qr and qx with space + single-quote ─────────────────────

/// my $re = qr 'foo' — qr operator with space + single-quote delimiter.
/// is_quote_char covers qr (op != "s" is true), so this must be accepted.
#[test]
fn qr_space_squote_statement() {
    assert_clean_parse(r#"my $re = qr 'foo';"#);
}

/// my $out = qx 'date' — qx operator with space + single-quote delimiter.
/// is_quote_char covers qx (op != "s" is true), so this must be accepted.
#[test]
fn qx_space_squote_statement() {
    assert_clean_parse(r#"my $out = qx 'date';"#);
}

// ── Critical regression guard: -s 'filename' must stay a filetest ────────────

/// if (-s 'tmpfile') { ... } — file-size filetest with string literal.
/// The op != "s" guard in is_quote_char prevents -s 'filename' from being
/// misinterpreted as a substitution operator start.
#[test]
fn s_filetest_with_string_literal_not_subst() {
    assert_clean_parse(r#"if (-s 'tmpfile') { print "has content"; }"#);
}
