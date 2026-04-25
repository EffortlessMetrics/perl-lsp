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

// ── Non-arrow hash subscript with quote-op keys ──────────────────────────────
//
// $h{s} (no arrow) was previously broken — the lexer treated `s` as a quote
// operator because after_arrow was false. Fixed by subsequent lexer work
// (after_var_subscript tracking in PR #2833/#2844), which correctly suppresses
// quote-operator recognition inside hash subscript braces even without `->`.

/// $h{s} — bare hash subscript (no arrow) with quote-op key.
/// Previously broken: lexer treated `s` as substitution operator.
/// Fixed by after_var_subscript lexer tracking.
#[test]
fn test_non_arrow_hash_key_s() {
    let source = r#"my $x = $h{s};"#;
    assert_clean_parse(source);
}

/// $h{q} — bare hash subscript with 'q' key.
/// Previously broken: lexer treated `q` as quote operator.
/// Fixed by after_var_subscript lexer tracking.
#[test]
fn test_non_arrow_hash_key_q() {
    let source = r#"my $x = $h{q};"#;
    assert_clean_parse(source);
}

// ── Regression: space before paren delimiter (all four paired chars covered) ─

/// s (pattern) (replacement) — space before paren delimiter.
/// Parens are one of the four paired delimiters ({, [, (, <) that are allowed
/// after whitespace, but this case was not previously tested.
#[test]
fn test_subst_space_before_paren() {
    let source = r#"$x =~ s (foo) (bar);"#;
    assert_clean_parse(source);
}

/// Mixed delimiter substitution: paired pattern + slash replacement.
/// Perl accepts `s{foo}/bar/`, and this must not be misclassified as
/// missing replacement.
#[test]
fn test_subst_mixed_paired_and_slash_delims() {
    assert_clean_parse(r#"$x =~ s{foo}/bar/;"#);
}

/// Mixed delimiter substitution with all other paired-open delimiter variants.
/// The fix must work for `[`, `(`, and `<` paired openers, not just `{`.
#[test]
fn test_subst_mixed_bracket_and_slash_delims() {
    assert_clean_parse(r#"$x =~ s[foo]/bar/;"#);
}

#[test]
fn test_subst_mixed_paren_and_slash_delims() {
    assert_clean_parse(r#"$x =~ s(foo)/bar/;"#);
}

#[test]
fn test_subst_mixed_angle_and_slash_delims() {
    assert_clean_parse(r#"$x =~ s<foo>/bar/;"#);
}

/// Mixed delimiter substitution with a non-slash non-paired replacement.
/// Perl also accepts `s{foo}!bar!` (any non-alphanumeric, non-whitespace char).
#[test]
fn test_subst_mixed_paired_and_pipe_delims() {
    assert_clean_parse(r#"$x =~ s{foo}|bar|;"#);
}

/// Missing replacement after paired pattern delimiter must still error.
/// `s{foo}` with no second part is malformed.
#[test]
fn test_subst_paired_pattern_missing_replacement_errors() {
    assert_has_error(r#"$x =~ s{foo};"#, "Missing");
}

/// Mixed delimiter transliteration: paired search + slash replacement.
/// Perl accepts `tr{abc}/xyz/`.
#[test]
fn test_transliteration_mixed_paired_and_slash_delims() {
    assert_clean_parse(r#"$x =~ tr{abc}/xyz/;"#);
}

/// Mixed delimiter transliteration for bracket and paren paired openers.
#[test]
fn test_transliteration_mixed_bracket_and_slash_delims() {
    assert_clean_parse(r#"$x =~ tr[abc]/xyz/;"#);
}

// ── Regression: non-paired delimiters after whitespace must be rejected ───────

/// -s $bs — file-size test, not substitution. The original XSLoader.pm regression.
/// `s` here is a bareword file-test operator; `$bs` is its argument.
/// With whitespace before `$`, the lexer must NOT treat `$` as a delimiter.
#[test]
fn test_file_size_test_not_subst() {
    let source = r#"goto \&XSLoader::bootstrap_inherit if not -f $file or -s $bs;"#;
    assert_clean_parse(source);
}

/// -s $bs in assignment context — variant of the core regression
#[test]
fn test_file_size_test_in_condition() {
    let source = r#"if (-s $file) { print "has content"; }"#;
    assert_clean_parse(source);
}

// ── Regression: comma-delimited substitution (XSLoader.pm) ──────────────────

/// XSLoader.pm line 43: s,[\\/][^\\/]+$,, — comma as substitution delimiter.
/// This is a non-standard but valid Perl delimiter. Must parse cleanly.
#[test]
fn test_comma_delimited_subst_xsloader() {
    let source = r#"$modlibname =~ s,[\\/][^\\/]+$,, while $c--;"#;
    assert_clean_parse(source);
}

/// Diagnostic preservation: invalid substitution modifiers should remain
/// reported as errors, even after delimiter-path fixes.
#[test]
fn test_invalid_substitution_modifier_still_errors() {
    assert_has_error(r#"$x =~ s{foo}{bar}z;"#, "Invalid substitution modifier");
}

/// Diagnostic preservation: unterminated replacement still reports the
/// missing-closing-delimiter error.
#[test]
fn test_missing_substitution_closing_delimiter_still_errors() {
    assert_has_error(r#"$x =~ s/foo/;"#, "Missing closing delimiter in substitution");
}

/// XSLoader.pm full content — must parse cleanly (corpus gate regression test).
#[test]
fn test_xsloader_pm_full() {
    let source = r#"use strict;
no strict 'refs';

package XSLoader;

our $VERSION = "0.32";

package DynaLoader;

boot_DynaLoader('DynaLoader') if defined(&boot_DynaLoader) &&
                                !defined(&dl_error);
package XSLoader;

sub load {
    package DynaLoader;

    my ($caller, $modlibname) = caller();
    my $module = $caller;

    if (@_) {
        $module = $_[0];
    } else {
        $_[0] = $module;
    }

    my $boots = "$module\::bootstrap";
    goto &$boots if defined &$boots;

    goto \&XSLoader::bootstrap_inherit unless $module and defined &dl_load_file;

    my @modparts = split(/::/,$module);
    my $modfname = $modparts[-1];
    my $modfname_orig = $modfname;

    my $modpname = join('/',@modparts);
    my $c = () = split(/::/,$caller,-1);
    $modlibname =~ s,[\\/][^\\/]+$,, while $c--;
    if ($modlibname !~ m{^/}) {
        FOUND: {
            for (@INC) {
                if ($_ eq $modlibname) {
                    last FOUND;
                }
            }
            goto \&XSLoader::bootstrap_inherit;
        }
    }
    my $file = "$modlibname/auto/$modpname/$modfname.so";

    my $bs = "$modlibname/auto/$modpname/$modfname_orig.bs";

    goto \&XSLoader::bootstrap_inherit if not -f $file or -s $bs;

    my $bootname = "boot_$module";
    $bootname =~ s/\W/_/g;
    @DynaLoader::dl_require_symbols = ($bootname);

    my $boot_symbol_ref;

    my $libref = dl_load_file($file, 0) or do {
        require Carp;
        Carp::croak("Can't load '$file' for module $module: " . dl_error());
    };
    push(@DynaLoader::dl_librefs,$libref);

    $boot_symbol_ref = dl_find_symbol($libref, $bootname) or do {
        require Carp;
        Carp::croak("Can't find '$bootname' symbol in $file\n");
    };

    push(@DynaLoader::dl_modules, $module);

  boot:
    my $xs = dl_install_xsub($boots, $boot_symbol_ref, $file);

    push(@DynaLoader::dl_shared_objects, $file);
    return &$xs(@_);
}

sub bootstrap_inherit {
    require DynaLoader;
    goto \&DynaLoader::bootstrap_inherit;
}

1;
"#;
    assert_clean_parse(source);
}

// ── Regression guard for #2895 fix: -s 'filename' must remain a filetest ─────

/// -s 'config.txt' — file-size filetest with a string literal argument.
/// After the #2895 fix (which allows ' and " after whitespace for non-s operators),
/// the `op != "s"` guard must keep -s 'filename' as a filetest, NOT a substitution.
#[test]
fn test_file_size_test_with_string_literal() {
    assert_clean_parse(r#"if (-s 'config.txt') { 1 }"#);
}
