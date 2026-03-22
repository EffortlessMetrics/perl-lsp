//! Tests for issue #2732: substitution/transliteration modifier parsing fixes
//!
//! Root Cause 1A: Path 2 in lexer uses current_char() instead of peek_nonspace()
//!   for s/tr/y/m detection — fails when whitespace precedes the delimiter.
//!
//! Root Cause 1B: after_arrow flag persists across statement boundaries (;, ), })
//!   causing s/// on the next statement to be treated as an identifier.
//!
//! Root Cause 2: is_quote_delim() rejects control characters via .is_control(),
//!   but Perl allows any non-alphanumeric, non-whitespace delimiter (e.g. BEL \x07).
//!
//! Root Cause 3: parse_hash_subscript_key() doesn't handle quote-operator
//!   identifiers (s, m, tr, y, q, qq, qw, qr, qx) as bareword hash keys.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Root Cause 1A: whitespace before delimiter ──────────────────────────────

/// s { pattern } { replacement }g — space before opening brace
#[test]
fn test_subst_space_before_brace() {
    assert_clean_parse("s { foo } { bar }g;");
}

/// s [pattern] [replacement]ex — space before bracket (MakeMaker.pm pattern)
#[test]
fn test_subst_space_before_bracket() {
    let source = r#"$value =~ s [^~(\w*)] [$1]ex;"#;
    assert_clean_parse(source);
}

/// s < pattern > <replacement> — space before angle bracket (Data/Dumper.pm pattern)
#[test]
fn test_subst_space_before_angle() {
    let source = r#"$x =~ s <foo> <bar>;"#;
    assert_clean_parse(source);
}

/// s\n{pattern}{replacement}g — newline before delimiter (diagnostics.pm pattern)
#[test]
fn test_subst_newline_before_delimiter() {
    assert_clean_parse("s\n{foo}\n{bar}g;");
}

// ── Root Cause 1B: after_arrow persists across statement boundary ────────────

/// s/// on next statement after ->() call — after_arrow must be cleared by ;
/// Reproduces Filter/Simple.pm: $transform->(@_); s/$extractor/.../g;
#[test]
fn test_subst_after_arrow_call_statement() {
    let source = r#"$transform->(@_); s/foo/bar/g;"#;
    assert_clean_parse(source);
}

/// after_arrow cleared by ) — method call inside expression
#[test]
fn test_subst_after_method_call_paren() {
    let source = r#"my $x = $obj->method(); s/foo/bar/;"#;
    assert_clean_parse(source);
}

/// after_arrow cleared by } — block following method call
#[test]
fn test_subst_after_arrow_in_block() {
    let source = r#"if ($obj->thing) { s/foo/bar/; }"#;
    assert_clean_parse(source);
}

// ── Root Cause 2: control character delimiter ────────────────────────────────

/// s\x07pattern\x07replacement\x07 — BEL as delimiter (perl5db.pl pattern)
#[test]
fn test_subst_bel_delimiter() {
    // BEL character (\x07) as substitution delimiter
    assert_clean_parse("s\x07foo\x07bar\x07;");
}

// ── Root Cause 3: hash key 's' and other quote-op identifiers ───────────────

/// $_->{s} — 's' as bare hash subscript key (Biber/Config.pm pattern)
#[test]
fn test_hash_key_s() {
    let source = r#"my $x = $_->{s};"#;
    assert_clean_parse(source);
}

/// $_->{m} — 'm' as bare hash subscript key via arrow
#[test]
fn test_hash_key_m() {
    let source = r#"my $x = $_->{m};"#;
    assert_clean_parse(source);
}

/// $_->{tr} — 'tr' as bare hash subscript key via arrow
#[test]
fn test_hash_key_tr() {
    let source = r#"my $x = $_->{tr};"#;
    assert_clean_parse(source);
}

/// $_->{y} — 'y' as bare hash subscript key via arrow
#[test]
fn test_hash_key_y() {
    let source = r#"my $x = $_->{y};"#;
    assert_clean_parse(source);
}

/// Hash slice with quote-op keys in fat-arrow list context — s/m/tr/y before => must autoquote.
/// Regression: with peek_nonspace(), s before => was being treated as substitution operator.
#[test]
fn test_hash_quote_op_fat_arrow() {
    let source = r#"my %h = (s => 1, m => 2, tr => 3, y => 4);"#;
    assert_clean_parse(source);
}

// ── Regression guards: existing adjacent-delimiter forms must still work ─────

/// s/foo/bar/g — adjacent slash delimiter unaffected
#[test]
fn test_subst_adjacent_slash_regression() {
    assert_clean_parse("s/foo/bar/g;");
}

/// s{foo}{bar}g — adjacent brace delimiter unaffected
#[test]
fn test_subst_adjacent_brace_regression() {
    assert_clean_parse("s{foo}{bar}g;");
}

/// $obj->s — method named 's' must NOT be treated as substitution operator
#[test]
fn test_arrow_method_named_s_regression() {
    let source = r#"$obj->s("arg");"#;
    assert_clean_parse(source);
}

// ── Root Cause 3 extended: q-family hash keys ────────────────────────────────

/// $_->{q} — 'q' as bare hash subscript key via arrow
#[test]
fn test_hash_key_q() {
    let source = r#"my $x = $_->{q};"#;
    assert_clean_parse(source);
}

/// $_->{qq} — 'qq' as bare hash subscript key via arrow
#[test]
fn test_hash_key_qq() {
    let source = r#"my $x = $_->{qq};"#;
    assert_clean_parse(source);
}

/// $_->{qw} — 'qw' as bare hash subscript key via arrow
#[test]
fn test_hash_key_qw() {
    let source = r#"my $x = $_->{qw};"#;
    assert_clean_parse(source);
}

// ── Regression guard: chained hash access with arrow ────────────────────────

/// $h->{outer}->{inner} — chained hash access must not break after_arrow clearing on }
/// If clearing after_arrow on } breaks chained access, this will parse with errors.
#[test]
fn test_chained_hash_access_regression() {
    let source = r#"my $x = $h->{outer}->{inner};"#;
    assert_clean_parse(source);
}

/// $h->{s}->{m} — chained access with quote-op keys at each level
#[test]
fn test_chained_hash_access_quote_op_keys() {
    let source = r#"my $x = $h->{s}->{m};"#;
    assert_clean_parse(source);
}
