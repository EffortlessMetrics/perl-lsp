//! RIPR seam-closure tests for `LocalSymbolTable::scan_subs`.
//!
//! Each test drives a specific predicate boundary in `scan_subs` with an
//! exact-count oracle (`assert_eq!(st.len(), N)`) so that any mutation of
//! the predicate causes the oracle to fail.
//!
//! Seams targeted (by line in `crates/perl-lexer/src/symbol_table.rs`):
//!
//! | Line | Expression |
//! |------|-----------|
//! | 54   | `while i < len {` |
//! | 58   | `while i < len && bytes[i] != b'\n' {` |
//! | 67   | `while i < len {` (single-quote inner) |
//! | 68   | `if bytes[i] == b'\\' && i + 1 < len {` (single-quote escape) |
//! | 70   | `} else if bytes[i] == b'\'' {` (close single-quote) |
//! | 82   | `while i < len {` (double-quote inner) |
//! | 83   | `if bytes[i] == b'\\' && i + 1 < len {` (double-quote escape) |
//! | 85   | `} else if bytes[i] == b'"' {` (close double-quote) |
//! | 95   | `b's' if i + 3 <= len && bytes[i..i + 3] == *b"sub"` |
//! | 97   | `let prev_ident = i > 0 && is_ident_byte(bytes[i - 1]);` |
//! | 99   | `let next_ident = i + 3 < len && is_ident_byte(bytes[i + 3]);` |
//! | 104  | `while j < len && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {` |
//! | 108  | `if j < len && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'_') {` |

use perl_lexer::LocalSymbolTable;

// ---------------------------------------------------------------------------
// Line 54: `while i < len` — outer loop boundary
// ---------------------------------------------------------------------------

/// Empty input: the loop body never executes, table must be empty.
/// Mutating `while i < len` to `while i < len + 1` would panic.
/// Mutating to `while i < len - 1` would skip the last char; harmless for
/// empty but the exact-0 oracle is the discriminator for the `i < len` guard.
#[test]
fn seam_54_outer_loop_empty_input_exact_zero() {
    let st = LocalSymbolTable::scan_subs("");
    assert_eq!(st.len(), 0, "empty input must produce exactly 0 known subs (outer loop seam)");
}

/// Single-byte input: the loop runs exactly once, no sub found.
#[test]
fn seam_54_outer_loop_single_byte_exact_zero() {
    let st = LocalSymbolTable::scan_subs("x");
    assert_eq!(st.len(), 0, "single non-sub byte must produce exactly 0 known subs");
}

/// Input exactly 3 bytes "sub": boundary i+3 == len with nothing after; no name follows.
#[test]
fn seam_54_outer_loop_truncated_sub_keyword_exact_zero() {
    // "sub" with no following name — word boundary passes, but no ident after,
    // so j ends at len with j < len false; no insertion happens.
    let st = LocalSymbolTable::scan_subs("sub");
    assert_eq!(
        st.len(),
        0,
        "bare 'sub' with no name must produce exactly 0 subs (outer loop / sub-parse seam)"
    );
}

// ---------------------------------------------------------------------------
// Line 58: `while i < len && bytes[i] != b'\n'` — comment EOF boundary
// ---------------------------------------------------------------------------

/// Comment that runs to EOF (no trailing newline): loop exits when i == len,
/// not when hitting b'\n'. If the `i < len` guard were removed the loop would
/// access bytes[len] (out of bounds / panic).  The oracle: exactly 0 subs.
#[test]
fn seam_58_comment_to_eof_no_newline_exact_zero() {
    let src = "# sub never { }";
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        0,
        "sub inside comment running to EOF must not be registered (comment-EOF boundary seam)"
    );
}

/// Comment terminated by newline, followed by a real sub: the `!= b'\n'` guard
/// must stop the skip at the newline so the subsequent sub IS found.
/// Mutating `!= b'\n'` to always-skip would produce 0, not 1.
#[test]
fn seam_58_comment_newline_terminates_skip_exact_one() {
    let src = "# sub fake { }\nsub real { }";
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub should be found after comment newline (comment-newline boundary seam)"
    );
    assert!(st.is_known_sub("real"));
}

/// Two comments, each terminated by newline, then a real sub.
/// Exact count distinguishes correct multi-comment handling.
#[test]
fn seam_58_two_comments_then_sub_exact_one() {
    let src = "# comment one\n# comment two\nsub real2 { }";
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(st.len(), 1, "exactly 1 sub after two comments (comment-newline boundary seam)");
    assert!(st.is_known_sub("real2"));
}

// ---------------------------------------------------------------------------
// Lines 67–70: single-quote string inner loop + escape + close predicates
// ---------------------------------------------------------------------------

/// Single-quoted string with an escaped backslash `\\` followed by an
/// escaped quote `\'` inside.  The backslash-escape branch (line 68)
/// must skip 2 bytes; the close-quote branch (line 70) must fire on the
/// ACTUAL closing `'`, not the escaped one.
///
/// If line 68 (`i + 1 < len`) mutates to skip only 1 byte, the inner `'`
/// prematurely closes the string and `sub inner` becomes visible.
#[test]
fn seam_67_68_70_single_quote_escaped_quote_hides_sub_exact_zero() {
    // 'it\'s a trap sub hidden { }' — the \' is an escape, not string close.
    // After the real closing ', "sub real" appears.
    let src = r#"my $x = 'it\'s a trap sub hidden { }'; sub real { }"#;
    let st = LocalSymbolTable::scan_subs(src);
    // "hidden" must NOT appear; "real" must appear; count must be exactly 1.
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub: escaped single-quote must not close the string (seam 68/70)"
    );
    assert!(st.is_known_sub("real"), "'real' must be registered");
    assert!(!st.is_known_sub("hidden"), "'hidden' inside single-quote must be suppressed");
}

/// Single-quoted string terminated by a `\\` at end-of-string (backslash
/// followed immediately by closing quote — `'\\'`).  The `i + 1 < len`
/// guard in line 68 ensures we don't step past the buffer end.
#[test]
fn seam_68_single_quote_backslash_at_string_end_exact_one() {
    // String '\' followed by real sub. The \\ in the string is `\` + `'`
    // escape pair, closing cleanly, so "real" is found.
    // Using raw string to write: my $x = '\\'; sub real { }
    let src = "my $x = '\\\\'; sub real { }";
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub after single-quoted string ending in \\\\ (seam 68 i+1<len guard)"
    );
    assert!(st.is_known_sub("real"));
}

/// Single-quoted string with content but no backslash: the `else { i += 1 }`
/// branch must advance past ordinary chars.  Exact count ensures the string
/// content is not treated as code.
#[test]
fn seam_70_single_quote_plain_content_hides_sub_exact_one() {
    let src = "my $x = 'sub hidden { }'; sub visible { }";
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub: plain single-quoted string must hide inner 'sub' (seam 70 close)"
    );
    assert!(st.is_known_sub("visible"));
    assert!(!st.is_known_sub("hidden"));
}

// ---------------------------------------------------------------------------
// Lines 82–85: double-quote string inner loop + escape + close predicates
// ---------------------------------------------------------------------------

/// Double-quoted string with an escaped double-quote `\"` inside.
/// The escape branch (line 83) must skip 2 bytes so the inner `"` does NOT
/// prematurely close the string, keeping `sub hidden` invisible.
#[test]
fn seam_82_83_85_double_quote_escaped_quote_hides_sub_exact_one() {
    // "say \"sub hidden { }\"" — the \" are escapes, not string close.
    let src = r#"my $s = "say \"sub hidden { }\""; sub real { }"#;
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub: escaped double-quote must not close string (seam 83/85)"
    );
    assert!(st.is_known_sub("real"));
    assert!(!st.is_known_sub("hidden"), "'hidden' inside double-quote must be suppressed");
}

/// Double-quoted string ending with `\\` (escaped backslash, then real close).
/// Guard `i + 1 < len` on line 83 must hold.
#[test]
fn seam_83_double_quote_backslash_at_string_end_exact_one() {
    // "foo\\" followed by real sub
    let src = r#"my $x = "foo\\"; sub real { }"#;
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub after double-quoted string ending in \\\\ (seam 83 i+1<len guard)"
    );
    assert!(st.is_known_sub("real"));
}

/// Ordinary double-quoted content (no backslash): the `else { i += 1 }` branch.
#[test]
fn seam_85_double_quote_plain_content_hides_sub_exact_one() {
    let src = r#"my $x = "sub hidden { }"; sub visible { }"#;
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        1,
        "exactly 1 sub: plain double-quoted string must hide inner 'sub' (seam 85 close)"
    );
    assert!(st.is_known_sub("visible"));
    assert!(!st.is_known_sub("hidden"));
}

// ---------------------------------------------------------------------------
// Line 95: `b's' if i + 3 <= len && bytes[i..i + 3] == *b"sub"` boundary
// ---------------------------------------------------------------------------

/// Input "su" (2 bytes): `i + 3 = 3 > len = 2` so the slice guard fails.
/// Exact 0 ensures the guard is enforced.
#[test]
fn seam_95_truncated_su_does_not_panic_exact_zero() {
    let st = LocalSymbolTable::scan_subs("su");
    assert_eq!(
        st.len(),
        0,
        "'su' (incomplete keyword) must produce 0 subs — seam 95 i+3<=len guard"
    );
}

/// "sub" at end of input with i+3 == len exactly (boundary case).
#[test]
fn seam_95_sub_keyword_at_exact_end_of_input_exact_zero() {
    // "sub" with nothing after — word boundary OK but no ident follows.
    let st = LocalSymbolTable::scan_subs("sub");
    assert_eq!(st.len(), 0, "bare 'sub' at EOF must produce 0 (seam 95 boundary + no-ident guard)");
}

// ---------------------------------------------------------------------------
// Line 97: `let prev_ident = i > 0 && is_ident_byte(bytes[i - 1]);`
// ---------------------------------------------------------------------------

/// "xsub foo" — prev char is 'x' (ident), so `prev_ident = true` and the
/// `sub` is NOT parsed as a keyword.  Exact 0 discriminates the guard.
#[test]
fn seam_97_prev_ident_blocks_sub_keyword_exact_zero() {
    let st = LocalSymbolTable::scan_subs("xsub foo { }");
    assert_eq!(st.len(), 0, "'xsub' must not be treated as keyword — seam 97 prev_ident guard");
}

/// "1sub foo" — prev char is '1' (ident byte via is_ident_byte), blocked.
#[test]
fn seam_97_prev_digit_blocks_sub_keyword_exact_zero() {
    let st = LocalSymbolTable::scan_subs("1sub foo { }");
    assert_eq!(
        st.len(),
        0,
        "'1sub' must not be treated as keyword — seam 97 prev_ident includes digits"
    );
}

/// Sub at start of input: `i = 0` so `i > 0` is false, prev_ident = false.
/// A real sub IS found.  Discriminates the `i > 0` part.
#[test]
fn seam_97_sub_at_start_of_input_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub start_of_file { }");
    assert_eq!(st.len(), 1, "sub at position 0 must be found — seam 97 i>0 guard");
    assert!(st.is_known_sub("start_of_file"));
}

// ---------------------------------------------------------------------------
// Line 99: `let next_ident = i + 3 < len && is_ident_byte(bytes[i + 3]);`
// ---------------------------------------------------------------------------

/// "subx foo" — char immediately after "sub" is 'x' (ident), next_ident = true,
/// keyword not matched.  Exact 0 discriminates.
#[test]
fn seam_99_next_ident_blocks_sub_keyword_exact_zero() {
    let st = LocalSymbolTable::scan_subs("subx foo { }");
    assert_eq!(st.len(), 0, "'subx' must not be treated as keyword — seam 99 next_ident guard");
}

/// "sub_foo" — 'sub' followed immediately by '_' (ident): next_ident = true.
#[test]
fn seam_99_next_underscore_blocks_sub_keyword_exact_zero() {
    let st = LocalSymbolTable::scan_subs("sub_foo { }");
    assert_eq!(
        st.len(),
        0,
        "'sub_foo' not a keyword invocation — seam 99 next_ident guard for '_'"
    );
}

/// "sub foo" — 'sub' followed by space (not ident): next_ident = false, match.
/// This is the positive case for the same boundary.
#[test]
fn seam_99_next_space_allows_sub_keyword_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub foo { }");
    assert_eq!(st.len(), 1, "'sub foo' must register foo — seam 99 next_ident=false allows match");
    assert!(st.is_known_sub("foo"));
}

// ---------------------------------------------------------------------------
// Line 104: whitespace-skip loop `matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n')`
// ---------------------------------------------------------------------------

/// Sub followed by a tab before name: the whitespace-skip must handle `\t`.
#[test]
fn seam_104_tab_after_sub_is_skipped_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub\tfoo_tab { }");
    assert_eq!(st.len(), 1, "sub followed by tab must still register the name (seam 104 tab skip)");
    assert!(st.is_known_sub("foo_tab"));
}

/// Sub followed by `\r\n` (Windows line ending): both CR and LF must be skipped.
#[test]
fn seam_104_crlf_after_sub_is_skipped_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub\r\nfoo_crlf { }");
    assert_eq!(
        st.len(),
        1,
        "sub followed by CRLF must still register the name (seam 104 CR+LF skip)"
    );
    assert!(st.is_known_sub("foo_crlf"));
}

/// Multiple spaces and a newline before the name: all whitespace classes.
#[test]
fn seam_104_mixed_whitespace_after_sub_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub  \t\n foo_ws { }");
    assert_eq!(
        st.len(),
        1,
        "sub with mixed whitespace must register name (seam 104 whitespace loop)"
    );
    assert!(st.is_known_sub("foo_ws"));
}

// ---------------------------------------------------------------------------
// Line 108: `if j < len && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'_')`
// ---------------------------------------------------------------------------

/// "sub 1bad" — first char after whitespace is '1' (digit, not alpha or `_`):
/// the ident-start guard (line 108) rejects it.  Exact 0 discriminates.
#[test]
fn seam_108_digit_start_rejects_ident_exact_zero() {
    let st = LocalSymbolTable::scan_subs("sub 1bad { }");
    assert_eq!(
        st.len(),
        0,
        "sub name starting with digit must be rejected (seam 108 ident-start guard)"
    );
}

/// "sub _ok" — first char is '_': the `bytes[j] == b'_'` branch of line 108
/// must accept it.  Exact 1 discriminates.
#[test]
fn seam_108_underscore_start_accepted_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub _ok { }");
    assert_eq!(
        st.len(),
        1,
        "sub name starting with _ must be accepted (seam 108 underscore branch)"
    );
    assert!(st.is_known_sub("_ok"));
}

/// "sub A" — uppercase alpha: `is_ascii_alphabetic()` must accept it.
#[test]
fn seam_108_uppercase_alpha_accepted_exact_one() {
    let st = LocalSymbolTable::scan_subs("sub Upper { }");
    assert_eq!(
        st.len(),
        1,
        "sub name starting with uppercase must be accepted (seam 108 alpha branch)"
    );
    assert!(st.is_known_sub("Upper"));
}

/// "sub " at EOF (no name follows): `j < len` fails, no insertion.
#[test]
fn seam_108_no_ident_after_whitespace_exact_zero() {
    let st = LocalSymbolTable::scan_subs("sub ");
    assert_eq!(st.len(), 0, "'sub ' with no name must produce 0 (seam 108 j<len guard)");
}

// ---------------------------------------------------------------------------
// Combined end-to-end boundary test (production caller path)
// ---------------------------------------------------------------------------

/// Drive `scan_subs` with input that exercises every predicate branch at once
/// in a realistic snippet, then assert the exact count.
///
/// This is the call-observation test the ripr artifact labels as "production
/// caller": `scan_subs` is the production scanning function wired into
/// `LexerConfig` via `parser_context.rs:73` and `lib.rs:1948`.
#[test]
fn seam_all_branches_realistic_snippet_exact_count() {
    let src = concat!(
        "# sub comment_sub { }\n",       // comment — skipped
        "my $q = 'sub str_sub { }';\n",  // single-quote — skipped
        "my $d = \"sub dq_sub { }\";\n", // double-quote — skipped
        "xsub not_kw { }\n",             // prev-ident guard — not a keyword
        "sub_not_kw { }\n",              // next-ident guard — not a keyword
        "sub real_alpha { }\n",          // plain sub — found
        "sub\t_under_tab { }\n",         // tab + underscore-start — found
        "sub\r\ncrlf_sub { }\n",         // CRLF whitespace — found
    );
    let st = LocalSymbolTable::scan_subs(src);
    assert_eq!(
        st.len(),
        3,
        "exactly 3 subs must be registered from the combined snippet (all-branch seam test)"
    );
    assert!(st.is_known_sub("real_alpha"));
    assert!(st.is_known_sub("_under_tab"));
    assert!(st.is_known_sub("crlf_sub"));
    assert!(!st.is_known_sub("comment_sub"));
    assert!(!st.is_known_sub("str_sub"));
    assert!(!st.is_known_sub("dq_sub"));
}
