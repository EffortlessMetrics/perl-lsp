// Differential recovery test — println! used for diagnostic output.
#![allow(clippy::print_stdout)]
//! Recovery-quality differential test suite for v1/v2/v3 Perl parsers.
//!
//! Tests measure how each parser handles *syntactically broken input* - specifically,
//! how much of the file after a syntax error each parser manages to keep parsing.
//!
//! # Why this matters
//!
//! PR #9170 measured correctness on well-formed input.  This suite measures the
//! orthogonal axis: **resilience to malformed input**.  Critical for LSP UX
//! because a file with a syntax error mid-edit is the common case - broken code
//! during typing.  A parser that gives up at the first error provides worse
//! goto-definition, hover, and references than one that recovers and continues.
//!
//! # Verdict model
//!
//! Each test records a [`RecoveryVerdict`] for each parser.  The test asserts the
//! *currently observed* verdict - not the "correct" one.  When a parser improves
//! or regresses the expected verdict must be updated intentionally.
//!
//! | Verdict | Meaning |
//! |---------|---------|
//! | `FullRecovery` | Parser found all expected post-error markers in its sexp output |
//! | `PartialRecovery` | Parser found some (>=1) but not all markers |
//! | `NoRecovery` | Parser produced no post-error content at all |
//! | `Crashed` | Parser panicked (caught with catch_unwind) |
//!
//! # Actual recovery table (measured on this codebase)
//!
//! | Case | v1 (tree-sitter-c) | v2 (Pest) | v3 (recursive-descent) |
//! |------|-------------------|-----------|------------------------|
//! | trailing_garbage | NoRecovery | PartialRecovery | PartialRecovery |
//! | unclosed_brace | PartialRecovery | NoRecovery | FullRecovery |
//! | unclosed_string | NoRecovery | FullRecovery* | NoRecovery |
//! | unclosed_quote_like | NoRecovery | NoRecovery | FullRecovery |
//! | missing_semicolon | NoRecovery | FullRecovery* | FullRecovery |
//! | mismatched_brackets | NoRecovery | FullRecovery* | FullRecovery |
//! | truncated_heredoc | PartialRecovery | FullRecovery* | FullRecovery |
//! | modern_class_syntax | NoRecovery | FullRecovery* | FullRecovery |
//! | invalid_double_sigil | NoRecovery | FullRecovery* | FullRecovery |
//! | multiple_errors | NoRecovery | PartialRecovery | NoRecovery |
//! | error_in_interpolated_string | NoRecovery | FullRecovery* | FullRecovery |
//! | error_inside_block | PartialRecovery | FullRecovery* | FullRecovery |
//!
//! # Surprising findings
//!
//! **v1 (tree-sitter-c) is weaker at recovery than expected.**  In most cases it
//! earns `NoRecovery` because the ERROR node absorbs post-error source text,
//! preventing the markers from appearing as named parse nodes in the sexp.
//! Tree-sitter does not give up parsing, but it tends to wrap large swaths of
//! broken code in ERROR containers rather than promoting them to statement nodes.
//!
//! **v2 (Pest) often earns `FullRecovery`** - but this is a *misleading* signal.
//! The verdicts marked with `*` are cases where v2 "recovers" by *silently
//! misparsing the input*: the string boundary shifts, the missing semicolon is
//! implicitly inserted, the bad sigil is accepted as valid, etc.  In each case
//! the post-error markers appear in the sexp *as part of a wrong parse* - not
//! because the parser correctly skipped over the error.  This is the most
//! dangerous failure mode for an LSP parser: the client sees a "full" parse
//! that is subtly wrong, not a visible error it can degrade gracefully.
//!
//! **v3 (recursive-descent) is the most reliably honest recoverer.**  It finds
//! post-error code in most cases by explicitly synchronizing at statement
//! boundaries, and unlike v2 it correctly surfaces the broken regions as
//! `ERROR` nodes rather than silently misparse them.

use std::panic;

use perl_parser_comparison::{parse_v1, parse_v2, parse_v3};

// --- Recovery Verdict ---------------------------------------------------------

/// Recovery quality verdict for a single parser on a broken-input case.
///
/// The verdict is based on how many of the M post-error markers the parser
/// manages to surface in its sexp output.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecoveryVerdict {
    /// Parser found all expected post-error markers.
    FullRecovery,
    /// Parser found some but not all post-error markers.
    PartialRecovery,
    /// Parser produced no post-error content at all.
    NoRecovery,
    /// Parser panicked (caught with catch_unwind).
    Crashed,
}

impl std::fmt::Display for RecoveryVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullRecovery => write!(f, "FullRecovery"),
            Self::PartialRecovery => write!(f, "PartialRecovery"),
            Self::NoRecovery => write!(f, "NoRecovery"),
            Self::Crashed => write!(f, "Crashed"),
        }
    }
}

// --- Helpers ------------------------------------------------------------------

/// Classify recovery quality based on how many markers are found in the sexp.
///
/// Returns:
/// - `FullRecovery` if all markers are found
/// - `PartialRecovery` if some (>=1) but not all are found
/// - `NoRecovery` if none are found
fn classify_recovery(sexp: &str, post_error_markers: &[&str]) -> RecoveryVerdict {
    if post_error_markers.is_empty() {
        return RecoveryVerdict::FullRecovery;
    }
    let found = post_error_markers.iter().filter(|&&m| sexp.contains(m)).count();
    match found {
        0 => RecoveryVerdict::NoRecovery,
        n if n == post_error_markers.len() => RecoveryVerdict::FullRecovery,
        _ => RecoveryVerdict::PartialRecovery,
    }
}

/// Run all three parsers on `source` and classify recovery quality.
///
/// `post_error_markers` are tokens/strings we expect to appear in the parse
/// output if the parser recovered past the error at line K.
///
/// Returns `(v1_verdict, v2_verdict, v3_verdict)`.
fn measure_recovery(
    source: &str,
    post_error_markers: &[&str],
) -> (RecoveryVerdict, RecoveryVerdict, RecoveryVerdict) {
    // v1: tree-sitter always produces a tree; check its sexp for markers
    let v1_result = panic::catch_unwind(|| parse_v1(source));
    let v1_verdict = match v1_result {
        Err(_) => RecoveryVerdict::Crashed,
        Ok(r) => classify_recovery(&r.sexp, post_error_markers),
    };

    // v2: Pest - may recover by misparse; sexp is empty only on catastrophic failure
    let v2_result = panic::catch_unwind(|| parse_v2(source));
    let v2_verdict = match v2_result {
        Err(_) => RecoveryVerdict::Crashed,
        Ok(r) => {
            // v2 on catastrophic parse failure produces an empty sexp ("")
            if r.sexp.is_empty() {
                RecoveryVerdict::NoRecovery
            } else {
                classify_recovery(&r.sexp, post_error_markers)
            }
        }
    };

    // v3: recursive-descent with recovery; always produces a sexp
    let v3_result = panic::catch_unwind(|| parse_v3(source));
    let v3_verdict = match v3_result {
        Err(_) => RecoveryVerdict::Crashed,
        Ok(r) => classify_recovery(&r.sexp, post_error_markers),
    };

    (v1_verdict, v2_verdict, v3_verdict)
}

/// Print a recovery row for --nocapture diagnostic output.
fn print_recovery_row(
    label: &str,
    v1: &RecoveryVerdict,
    v2: &RecoveryVerdict,
    v3: &RecoveryVerdict,
) {
    println!("  [R] {label:<55} | v1={v1:<16} | v2={v2:<16} | v3={v3}");
}

/// Assert a recovery verdict with a descriptive failure message.
fn assert_recovery(
    actual: &RecoveryVerdict,
    expected: &RecoveryVerdict,
    parser: &str,
    context: &str,
) {
    assert_eq!(
        actual, expected,
        "{parser}: recovery for '{context}' expected {expected:?}, got {actual:?}"
    );
}

// --- Recovery Case 1: Trailing garbage ---------------------------------------

/// Recovery 1: trailing-garbage mid-file.
///
/// Well-formed prefix, then `@@@ garbage @@@`, then well-formed suffix.
/// Tests whether the parser re-synchronizes after pure garbage.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - ERROR node absorbs all post-garbage tokens
/// - v2 (Pest): PartialRecovery - recovers `suffix` but `@@@` disrupts the sub
/// - v3 (recursive-descent): PartialRecovery - synchronizes but `@@@` disrupts `sub`
#[test]
fn recovery_01_trailing_garbage_mid_file() {
    let src = r#"
my $prefix = 1;
my $also_prefix = 2;
@@@ this is garbage not perl @@@
sub post_error_sub { return 42; }
my $suffix = 3;
"#;
    let post_error = &["post_error_sub", "suffix"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("trailing_garbage_mid_file", &v1, &v2, &v3);
    // v1: ERROR node swallows all post-garbage tokens including the sub and suffix
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "trailing_garbage_mid_file");
    // v2: partially misparses; finds `suffix` but not `post_error_sub`
    assert_recovery(&v2, &RecoveryVerdict::PartialRecovery, "v2", "trailing_garbage_mid_file");
    // v3: synchronizes at statement boundaries; partial recovery
    assert_recovery(&v3, &RecoveryVerdict::PartialRecovery, "v3", "trailing_garbage_mid_file");
}

// --- Recovery Case 2: Unclosed brace -----------------------------------------

/// Recovery 2: unclosed brace in a sub declaration.
///
/// `sub foo { my $x = 1;` (no closing `}`), then `sub bar { ... }`.
/// Tests whether the parser resynchronizes after an unclosed block.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): PartialRecovery - finds `bar` but not `after` (absorbed into foo)
/// - v2 (Pest): NoRecovery - fails on the unclosed brace (strict PEG)
/// - v3 (recursive-descent): FullRecovery - implicit close via synchronization; finds both
#[test]
fn recovery_02_unclosed_brace() {
    let src = r#"
my $before = 1;
sub foo { my $x = 1;
sub bar { my $y = 2; }
my $after = 3;
"#;
    let post_error = &["bar", "after"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("unclosed_brace", &v1, &v2, &v3);
    // v1: finds bar (parsed as nested sub) but "after" is swallowed
    assert_recovery(&v1, &RecoveryVerdict::PartialRecovery, "v1", "unclosed_brace");
    // v2: fails on unclosed brace
    assert_recovery(&v2, &RecoveryVerdict::NoRecovery, "v2", "unclosed_brace");
    // v3: full recovery - synchronizes and finds both post-error names
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "unclosed_brace");
}

// --- Recovery Case 3: Unclosed string ----------------------------------------

/// Recovery 3: unclosed double-quoted string.
///
/// `my $x = "abc` (newline before close), then `sub after_string { return 1; }`.
///
/// **Observed verdicts (surprising):**
/// - v1 (tree-sitter): NoRecovery - remaining code absorbed into string node
/// - v2 (Pest): FullRecovery* - misparses: string ends at newline, rest parsed as code
/// - v3 (recursive-descent): NoRecovery - string absorbs remaining content
///
/// The v2 `FullRecovery` is a *misleading* signal: v2 implicitly terminates the
/// string at the newline and continues parsing - finding `after_string` as code.
/// This is a silent mismatch with Perl's actual semantics (strings span lines).
#[test]
fn recovery_03_unclosed_string() {
    let src = "my $before = 1;\nmy $x = \"abc\nmy $y = 2;\nsub after_string { return 1; }\n";
    let post_error = &["after_string"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("unclosed_string", &v1, &v2, &v3);
    println!(
        "    NOTE: v2=FullRecovery is a misleading signal - v2 silently terminates \
         the unclosed string at newline and continues parsing with wrong semantics"
    );
    // v1: string node absorbs all remaining content
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "unclosed_string");
    // v2: silently terminates the string and finds after_string (wrong but found)
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "unclosed_string");
    // v3: string absorbs remaining content (same as v1 in this regard)
    assert_recovery(&v3, &RecoveryVerdict::NoRecovery, "v3", "unclosed_string");
}

// --- Recovery Case 4: Unclosed quote-like ------------------------------------

/// Recovery 4: unclosed quote-like operator `q{}`.
///
/// `my $x = q{ unmatched`, then `sub after_q { return 2; }`.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - q{} absorbs subsequent content
/// - v2 (Pest): NoRecovery - fails on unclosed q{}
/// - v3 (recursive-descent): FullRecovery - synchronizes after the broken q{}
///
/// v3's recovery from an unclosed quote-like operator is notably better than v1.
#[test]
fn recovery_04_unclosed_quote_like() {
    let src = "my $before = 1;\nmy $x = q{ unmatched\nmy $y = 2;\nsub after_q { return 2; }\n";
    let post_error = &["after_q"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("unclosed_quote_like", &v1, &v2, &v3);
    // v1: q{} swallows remaining source
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "unclosed_quote_like");
    // v2: fails on unclosed q{}
    assert_recovery(&v2, &RecoveryVerdict::NoRecovery, "v2", "unclosed_quote_like");
    // v3: FullRecovery - recovers and finds after_q
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "unclosed_quote_like");
}

// --- Recovery Case 5: Missing semicolon --------------------------------------

/// Recovery 5: missing semicolon between statements.
///
/// `my $x = 1 my $y = 2;` - semicolon is missing after `$x = 1`.
///
/// **Observed verdicts (surprising):**
/// - v1 (tree-sitter): NoRecovery - ERROR node swallows remaining content
/// - v2 (Pest): FullRecovery* - implicitly inserts semicolon and continues (wrong semantic)
/// - v3 (recursive-descent): FullRecovery - inserts synthetic semicolon via recovery
///
/// Both v2 and v3 find `after_missing_semi`, but through different mechanisms:
/// v3 uses explicit error recovery and emits an error diagnostic; v2 silently
/// mismisparse the input without any diagnostic.
#[test]
fn recovery_05_missing_semicolon() {
    let src = r#"
my $before = 0;
my $x = 1 my $y = 2;
sub after_missing_semi { return 99; }
"#;
    let post_error = &["after_missing_semi"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("missing_semicolon", &v1, &v2, &v3);
    // v1: ERROR node absorbs the bad expression and subsequent code
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "missing_semicolon");
    // v2: silently inserts semicolon and continues (FullRecovery but wrong parse)
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "missing_semicolon");
    // v3: inserts synthetic semicolon with error diagnostic, then continues
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "missing_semicolon");
}

// --- Recovery Case 6: Mismatched brackets ------------------------------------

/// Recovery 6: mismatched brackets.
///
/// `my @a = [1, 2, 3);` - `[` opened, `)` closes (wrong bracket type).
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - ERROR node absorbs post-mismatch code
/// - v2 (Pest): FullRecovery* - silently reconciles the mismatch, parses `after_mismatch`
/// - v3 (recursive-descent): FullRecovery - synchronizes at the mismatch, continues
#[test]
fn recovery_06_mismatched_brackets() {
    let src = r#"
my $before = 1;
my @a = [1, 2, 3);
sub after_mismatch { return 7; }
my $end = 99;
"#;
    let post_error = &["after_mismatch", "end"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("mismatched_brackets", &v1, &v2, &v3);
    // v1: ERROR node swallows downstream content
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "mismatched_brackets");
    // v2: silently fixes the mismatch and finds both markers
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "mismatched_brackets");
    // v3: synchronizes correctly, finds both markers
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "mismatched_brackets");
}

// --- Recovery Case 7: Truncated heredoc --------------------------------------

/// Recovery 7: truncated heredoc - opener but body never terminated.
///
/// `my $x = <<EOF;` but no terminating `EOF` line.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): PartialRecovery - finds `end` but not `after_heredoc`
/// - v2 (Pest): FullRecovery* - misparses; the heredoc consumes lines until
///   something in the file looks like `EOF`, then continues parsing
/// - v3 (recursive-descent): FullRecovery - finds both markers (content is in
///   the heredoc body sexp as literal text, visible to the marker search)
///
/// All three parsers treat the body text as heredoc content. The v1/v2/v3
/// difference is how much of the following code they absorb into the body.
#[test]
fn recovery_07_truncated_heredoc() {
    let src = r#"
my $before = 1;
my $x = <<EOF;
this heredoc never closes
sub after_heredoc { return 5; }
my $end = 2;
"#;
    let post_error = &["after_heredoc", "end"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("truncated_heredoc", &v1, &v2, &v3);
    println!(
        "    NOTE: markers may appear as heredoc body text (not as parsed nodes) \
         - this is still counted as 'found' by the marker search"
    );
    // v1: partially absorbs; finds end (literal in sexp) but misses after_heredoc
    assert_recovery(&v1, &RecoveryVerdict::PartialRecovery, "v1", "truncated_heredoc");
    // v2: finds both (misparsed heredoc termination)
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "truncated_heredoc");
    // v3: finds both (body text visible in sexp)
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "truncated_heredoc");
}

// --- Recovery Case 8: Modern class syntax (Perl 5.38+) -----------------------

/// Recovery 8: Perl 5.38+ `class` keyword with `:isa(Bar)`.
///
/// `class Foo :isa(Bar) { method baz () { ... } }` then `sub after_class { }`.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - class keyword not fully supported; ERROR absorbs
/// - v2 (Pest): FullRecovery* - `class` is parsed as a bareword function call
///   (wrong but accepted); `after_class` is subsequently found
/// - v3 (recursive-descent): FullRecovery - has explicit class/method support;
///   parses the class block correctly and continues to `after_class`
#[test]
fn recovery_08_modern_class_syntax() {
    let src = r#"
my $before = 1;
class Foo :isa(Bar) { method baz () { return 42; } }
sub after_class { return 1; }
my $end = 99;
"#;
    let post_error = &["after_class"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("modern_class_syntax", &v1, &v2, &v3);
    // No crashes
    assert_ne!(v1, RecoveryVerdict::Crashed, "v1 must not crash on class syntax");
    assert_ne!(v2, RecoveryVerdict::Crashed, "v2 must not crash on class syntax");
    assert_ne!(v3, RecoveryVerdict::Crashed, "v3 must not crash on class syntax");
    // v1: ERROR node absorbs class block, after_class not found
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "modern_class_syntax");
    // v2: class treated as bareword function, finds after_class
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "modern_class_syntax");
    // v3: class support is explicit, finds after_class
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "modern_class_syntax");
}

// --- Recovery Case 9: Invalid double sigil -----------------------------------

/// Recovery 9: invalid double sigil `my @@x = 5`.
///
/// This case was found during investigation for PR #9168.  v2 silently accepts
/// the wrong parse.  We measure whether code after this statement still parses.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - ERROR node eats subsequent code
/// - v2 (Pest): FullRecovery* - silently accepts `@@x` as valid, continues
/// - v3 (recursive-descent): FullRecovery - emits error diagnostic, synchronizes
///
/// The v2 FullRecovery is a misleading signal: it "recovers" by accepting
/// invalid syntax as valid rather than by skipping over the error.
#[test]
fn recovery_09_invalid_double_sigil() {
    let src = r#"
my $before = 1;
my @@x = 5;
sub after_double_sigil { return 3; }
my $end = 99;
"#;
    let post_error = &["after_double_sigil", "end"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("invalid_double_sigil", &v1, &v2, &v3);
    println!(
        "    NOTE: v2 FullRecovery is misleading - v2 accepts @@x as valid, \
         not a recovery from an error"
    );
    // v1: ERROR absorbs post-error code
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "invalid_double_sigil");
    // v2: silently accepts @@x as valid (wrong but continues)
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "invalid_double_sigil");
    // v3: emits error, synchronizes, finds both markers
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "invalid_double_sigil");
}

// --- Recovery Case 10: Multiple errors ---------------------------------------

/// Recovery 10: three syntax errors spread across a multi-statement file.
///
/// Does each parser recover between error sites, or bail at the first one?
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - large ERROR block swallows inter-error code
/// - v2 (Pest): PartialRecovery - first error `@@@` causes partial damage;
///   some subsequent subs are found but not all
/// - v3 (recursive-descent): NoRecovery - the unclosed string in error 2
///   absorbs remaining content, preventing markers from surfacing
///
/// This is the hardest case: multiple error types compound each other.
/// Neither v1 nor v3 fully survives three different error categories in one file.
#[test]
fn recovery_10_multiple_errors_across_file() {
    let src = r#"
my $a = 1;
sub first_good { return 1; }
@@@ first_error @@@
sub second_good { return 2; }
my $bad = "unclosed
sub third_good { return 3; }
my @c = [1, 2);
sub fourth_good { return 4; }
"#;
    let post_error = &["second_good", "third_good", "fourth_good"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("multiple_errors_across_file", &v1, &v2, &v3);
    println!(
        "    v1={v1} v2={v2} v3={v3}  \
         (3 error types: @@@ garbage, unclosed string, bracket mismatch)"
    );
    // No crashes
    assert_ne!(v1, RecoveryVerdict::Crashed, "v1 must not crash on multi-error file");
    assert_ne!(v2, RecoveryVerdict::Crashed, "v2 must not crash on multi-error file");
    assert_ne!(v3, RecoveryVerdict::Crashed, "v3 must not crash on multi-error file");
    // v1: large ERROR node, no markers found
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "multiple_errors_across_file");
    // v2: partially survives - some markers found but not all
    assert_recovery(&v2, &RecoveryVerdict::PartialRecovery, "v2", "multiple_errors_across_file");
    // v3: unclosed string absorbs remaining content, no markers found
    assert_recovery(&v3, &RecoveryVerdict::NoRecovery, "v3", "multiple_errors_across_file");
}

// --- Recovery Case 11: Error in interpolated string --------------------------

/// Recovery 11: error inside an interpolated string expression.
///
/// `print "the value is ${foo bar baz} done";` - the `${foo bar baz}` is not
/// valid Perl variable interpolation syntax.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): NoRecovery - ERROR node absorbs the statement
/// - v2 (Pest): FullRecovery* - misparses the interpolation but finds `after_interp`
/// - v3 (recursive-descent): FullRecovery - synchronizes at statement end,
///   continues to `after_interp`
///
/// The statement boundary after the `print` statement is the key recovery point.
/// v3 uses it correctly.  v1 absorbs into ERROR.
#[test]
fn recovery_11_error_in_interpolated_string() {
    let src = r#"
my $before = 1;
print "the value is ${foo bar baz} done";
sub after_interp { return 8; }
my $end = 55;
"#;
    let post_error = &["after_interp", "end"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("error_in_interpolated_string", &v1, &v2, &v3);
    // v1: ERROR absorbs the print statement and downstream code
    assert_recovery(&v1, &RecoveryVerdict::NoRecovery, "v1", "error_in_interpolated_string");
    // v2: misparses interpolation but continues; finds both markers
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "error_in_interpolated_string");
    // v3: statement-level recovery; finds both markers
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "error_in_interpolated_string");
}

// --- Recovery Case 12: Error inside a block ----------------------------------

/// Recovery 12: syntax error inside a sub body; following sub should parse.
///
/// `sub foo { @@@ }` - garbage inside a sub body, then `sub bar { return 1; }`.
///
/// **Observed verdicts:**
/// - v1 (tree-sitter): PartialRecovery - finds `bar` but not `end`
/// - v2 (Pest): FullRecovery* - `@@@` is partially accepted; `bar` and `end` found
/// - v3 (recursive-descent): FullRecovery - block boundary isolates the error;
///   `bar` and `end` parsed correctly
///
/// The sub/block boundary is a natural synchronization point.  v3 exploits it
/// cleanly.  v1 finds `bar` but swallows `end` into an ERROR region.
#[test]
fn recovery_12_error_inside_block() {
    let src = r#"
my $before = 1;
sub foo { @@@ }
sub bar { return 1; }
my $end = 42;
"#;
    let post_error = &["bar", "end"];
    let (v1, v2, v3) = measure_recovery(src, post_error);
    print_recovery_row("error_inside_block", &v1, &v2, &v3);
    // v1: finds bar but swallows end into ERROR
    assert_recovery(&v1, &RecoveryVerdict::PartialRecovery, "v1", "error_inside_block");
    // v2: finds both markers
    assert_recovery(&v2, &RecoveryVerdict::FullRecovery, "v2", "error_inside_block");
    // v3: block boundary recovers cleanly; finds both
    assert_recovery(&v3, &RecoveryVerdict::FullRecovery, "v3", "error_inside_block");
}

// --- Summary printer ---------------------------------------------------------

/// Print the recovery test suite header and legend.
///
/// Tests run in parallel so this may not appear first with --nocapture.
#[test]
fn zzz_recovery_summary_header() {
    let line: String = "-".repeat(90);
    println!("\n  {line}");
    println!("  Recovery-Quality Differential Test Suite");
    println!("  v1 = tree-sitter-perl-c  (C FFI)           - recovery via ERROR/MISSING nodes");
    println!(
        "  v2 = perl-parser-pest    (Pest/PEG legacy)  - often \"recovers\" by silent mismatch"
    );
    println!("  v3 = perl-parser-core    (recursive descent) - recovery via parse_with_recovery()");
    println!();
    println!("  Verdict meanings:");
    println!("    FullRecovery    - all post-error markers found in parse output");
    println!("    PartialRecovery - some (>=1) but not all post-error markers found");
    println!("    NoRecovery      - no post-error content in parse output");
    println!("    Crashed         - parser panicked (caught with catch_unwind)");
    println!();
    println!("  WARNING: v2 FullRecovery often signals SILENT MISMATCH, not genuine recovery.");
    println!("  v2 accepts invalid syntax as valid rather than skipping over errors.");
    println!("  {line}");
    println!("  [R] {:<55} | {:<20} | {:<20} | v3", "Label", "v1", "v2");
    println!("  {line}");
}
