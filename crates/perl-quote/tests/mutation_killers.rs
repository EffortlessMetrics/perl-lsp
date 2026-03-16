//! Targeted mutation-killing tests for `perl-quote`.
//!
//! Each test is labeled with the mutant it kills and why the test
//! distinguishes the mutant from correct behavior.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict,
};

// ──────────────────────────────────────────────────────────────
// MUTANT: lib.rs:14:23 replace > with >= in extract_regex_parts
//
// Current:  text.len() > 1   → bare "m" (len==1) does NOT match m-prefix path
// Mutant:   text.len() >= 1  → bare "m" (len==1) WOULD match m-prefix path
//
// With current code: content = "m", delimiter = 'm', pattern = "mm", body = ""
// With mutant:       content = "",  no delimiter, returns ("", "", "")
// ──────────────────────────────────────────────────────────────

#[test]
fn test_regex_bare_m_single_char_pattern_includes_delimiter()
-> Result<(), Box<dyn std::error::Error>> {
    // "m" alone: NOT treated as m// prefix (len == 1, not > 1)
    // The entire string becomes the content, so delimiter = 'm', closing = 'm'
    // Pattern wraps the (empty) body in delimiters: "mm"
    let (pat, body, mods) = extract_regex_parts("m");
    // Mutant would return ("", "", "") — this assertion kills it
    assert_eq!(pat, "mm", "bare 'm' should use 'm' as delimiter giving pattern 'mm'");
    assert_eq!(body, "", "body inside mm is empty");
    assert_eq!(mods, "", "no modifiers");
    Ok(())
}

#[test]
fn test_regex_m_with_alphabetic_second_char_treated_as_literal()
-> Result<(), Box<dyn std::error::Error>> {
    // "ms" — 'm' followed by alphabetic 's': NOT a regex prefix (second char is alphabetic)
    // So content = "ms", delimiter = 'm', closing = 'm', body = "s", pattern = "msm"
    let (pat, body, _mods) = extract_regex_parts("ms");
    // If mutant changed > to >= AND the alphabetic check still holds for "ms",
    // this would still pass — but for bare "m" (len==1) the mutant produces different output
    assert_eq!(body, "s", "content after stripping first delimiter 'm'");
    assert_eq!(pat, "msm", "pattern wraps body in 'm' delimiters");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// MUTANT: lib.rs:86:8 delete ! in extract_substitution_parts_strict
//
// Current:  if !is_paired && !pattern_closed { return Err(MissingClosingDelimiter) }
// Mutant:   if  is_paired && !pattern_closed { ... }  (same as the paired check on line 91)
//
// With mutant: non-paired unclosed substitutions do NOT return MissingClosingDelimiter
// ──────────────────────────────────────────────────────────────

#[test]
fn test_strict_subst_non_paired_unclosed_returns_missing_closing_delimiter() {
    // "s/foo" — non-paired delimiter '/', pattern "foo" has no closing '/'
    // Must return MissingClosingDelimiter
    // With mutant (deletes !): is_paired=false so `is_paired && !pattern_closed` is false,
    // error is NOT triggered — mutant would proceed and fail differently or succeed
    let result = extract_substitution_parts_strict("s/foo");
    assert_eq!(
        result,
        Err(SubstitutionError::MissingClosingDelimiter),
        "non-paired unclosed pattern must return MissingClosingDelimiter"
    );
}

#[test]
fn test_strict_subst_non_paired_fully_closed_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the non-error path still works so we don't accidentally over-constrain
    let result = extract_substitution_parts_strict("s/foo/bar/");
    assert!(result.is_ok(), "complete non-paired substitution should succeed");
    let (pat, repl, _mods) = result.map_err(|e| format!("{:?}", e))?;
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// MUTANT: lib.rs:119:17 delete match arm '\\' in extract_substitution_parts_strict
//
// Current:  '\\' arm: push '\\', set escaped=true → next char is literal
// Mutant:   '\\' falls to `_ => body.push(ch)` — escape flag never set
//           so s/foo/bar\// would see '/' after '\' as the closing delimiter
// ──────────────────────────────────────────────────────────────

#[test]
fn test_strict_subst_escaped_delimiter_in_replacement_body()
-> Result<(), Box<dyn std::error::Error>> {
    // s/foo/bar\//  → pattern="foo", replacement="bar\/" (backslash-escaped '/')
    // The final '/' is the actual closing delimiter
    let result = extract_substitution_parts_strict(r"s/foo/bar\//");
    assert!(result.is_ok(), "escaped delimiter in replacement should not prematurely close it");
    let (pat, repl, _mods) = result.map_err(|e| format!("{:?}", e))?;
    assert_eq!(pat, "foo");
    // With mutant (no escape handling), '\' does not protect '/', so '/' terminates early
    // and replacement would be "bar\" instead of "bar\/"
    assert_eq!(repl, r"bar\/", "backslash should escape the delimiter in replacement");
    Ok(())
}

#[test]
fn test_strict_subst_double_backslash_in_replacement() -> Result<(), Box<dyn std::error::Error>> {
    // s/foo/bar\\/  → pattern="foo", replacement="bar\\" (double backslash = literal backslash)
    // After the double-backslash, escaped resets to false, so '/' properly terminates
    let result = extract_substitution_parts_strict(r"s/foo/bar\\/");
    assert!(result.is_ok(), "double backslash in replacement should work");
    let (pat, repl, _mods) = result.map_err(|e| format!("{:?}", e))?;
    assert_eq!(pat, "foo");
    assert_eq!(repl, r"bar\\", "double backslash is a literal backslash in replacement");
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// MUTANT: lib.rs:251:42 replace || with && in extract_substitution_parts
//
// Current:  if delimiter.is_ascii_alphanumeric() || delimiter.is_whitespace()
// Mutant:   if delimiter.is_ascii_alphanumeric() && delimiter.is_whitespace()
//
// Whitespace chars are not alphanumeric → mutant: `false && true` = false
// Current: `false || true` = true → takes the alphanumeric/whitespace branch
//
// Perl allows: s foo bar  (space delimited, unusual but valid)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_subst_whitespace_delimiter_triggers_paired_delimiter_branch() {
    // s<SPACE>foo<SPACE>bar<SPACE>  — space delimiter
    // With current code: ' '.is_ascii_alphanumeric() || ' '.is_whitespace() = true
    //   → tries split_on_last_paired_delimiter, which fails (no paired delimiters found)
    //   → returns ("", "", "")
    // With mutant: ' '.is_ascii_alphanumeric() && ' '.is_whitespace() = false
    //   → falls through to the normal non-paired delimiter path
    //   → parses differently
    // The key: both paths must agree on some observable output
    let (pat, repl, _mods) = extract_substitution_parts("s foo bar ");
    // Current code: space delimiter hits alphanumeric||whitespace branch → split_on_last_paired
    // which finds no paired delimiters in "s foo bar " → returns ("", "", "")
    // Mutant: space goes through normal non-paired path → would parse differently
    // We assert on the current behavior to kill the mutant
    assert_eq!(
        (pat.as_str(), repl.as_str()),
        ("", ""),
        "whitespace delimiter triggers alphanumeric/whitespace branch returning empty"
    );
}

#[test]
fn test_subst_alphanumeric_delimiter_a_returns_empty() {
    // s a b a  — alphanumeric delimiter, clearly invalid in practice
    // Both current AND mutant handle this the same (alphanumeric is true for &&)
    // This is a sanity check that alphanumeric alone is handled
    let (pat, repl, _mods) = extract_substitution_parts("saabag");
    // split_on_last_paired_delimiter won't find paired delimiters → ("", "", "")
    assert_eq!(pat, "");
    assert_eq!(repl, "");
}

#[test]
fn test_subst_space_vs_exclamation_delimiter_differ() {
    // Exclamation is non-alphanumeric, non-whitespace → normal path
    let (pat_exc, repl_exc, _) = extract_substitution_parts("s!foo!bar!");
    // Space is whitespace → alphanumeric||whitespace branch
    let (pat_sp, repl_sp, _) = extract_substitution_parts("s foo bar ");

    // With current code the paths differ in behavior:
    // '!' uses normal path → pat="foo", repl="bar"
    assert_eq!(pat_exc, "foo");
    assert_eq!(repl_exc, "bar");
    // ' ' uses whitespace branch → ("", "")
    assert_eq!(pat_sp, "");
    assert_eq!(repl_sp, "");
    // This test kills mutant 4: if || becomes &&, space would take normal path
    // and produce "foo","bar" instead — the assertion above would fail
}

// ──────────────────────────────────────────────────────────────
// MUTANTS: lib.rs:277 — logic for `else if !is_paired && !pattern_closed`
//
// Current: else if !is_paired && !pattern_closed
//   → fallback to split_unclosed_substitution_pattern when non-paired AND unclosed
// Mutant 5 (replace && with ||): triggers even when IS paired OR IS closed
// Mutant 6 (delete ! on is_paired): becomes `is_paired && !pattern_closed`
//           → triggers for paired+unclosed but NOT for non-paired+unclosed
// Mutant 7 (delete ! on pattern_closed): becomes `!is_paired && pattern_closed`
//           → triggers for non-paired+closed but NOT for non-paired+unclosed
// ──────────────────────────────────────────────────────────────

#[test]
fn test_subst_lenient_non_paired_unclosed_uses_fallback() {
    // Lenient version of a non-paired unclosed substitution
    // "s/foo" — non-paired '/', pattern unclosed (no rest after pattern)
    // With current code:
    //   branch 1: `!is_paired && !rest1.is_empty()` — rest1 IS empty (no replacement found) → false
    //   branch 2: `!is_paired && !pattern_closed` — !false && !false = false if closed
    //             but for unclosed: rest1="", pattern_closed=false
    //   Actually for "s/foo": extract_delimited_content_strict("s/foo" minus 's' = "/foo")
    //   → opens on '/', reads "foo", EOF without closing '/' → body="foo", rest="", found_closing=false
    //   So pattern="foo", rest1="", pattern_closed=false
    //   branch 1: !is_paired && !rest1.is_empty() → !false && false = false (rest1 IS empty)
    //   branch 2: !is_paired && !pattern_closed → true && true = TRUE → fallback path
    //   split_unclosed_substitution_pattern("foo") → None (no paired delimiters in "foo")
    //   → (String::new(), Cow::Borrowed(rest1=""))
    let (_pat, _repl, _mods) = extract_substitution_parts("s/foo");
    // With mutant 6 (is_paired instead of !is_paired): is_paired=false → false&&true=false
    //   → goes to `else` branch → (String::new(), Cow::Borrowed(""))
    // Wait — both paths seem to return ("foo", "") here. Let me reconsider.
    //
    // The key distinction: branch 2 might transform `pattern` via fallback.
    // For "s/foo", fallback finds no paired delimiter → pattern stays "foo", repl=""
    // Without branch 2 (mutant 6): else branch → pattern="foo", repl=""
    // Same output! This mutant may be equivalent here.
    //
    // Use a case where fallback DOES transform things: "s/foo{bar}baz"
    // pattern = "foo{bar}baz" (unclosed slash), pattern_closed=false
    // split_unclosed_substitution_pattern("foo{bar}baz"):
    //   finds '{' at idx=3, calls extract_delimited_content_strict("{bar}baz", '{', '}')
    //   returns ("bar", "baz", true)
    //   leading = "foo"
    //   returns Some(("foo", "bar", "baz"))
    // So: pattern="foo", repl="bar", modifiers="baz"
    let (pat2, repl2, mods2) = extract_substitution_parts("s/foo{bar}baz");
    // With mutant 6 (is_paired replaces !is_paired): branch 2 becomes `is_paired && !pattern_closed`
    //   is_paired = false for '/' delimiter → branch 2 NOT taken → else branch
    //   → repl = ""
    // Fallback: pattern="foo", repl="bar", rest="baz"
    // extract_substitution_modifiers("baz") filters: 'b' invalid, 'a' valid, 'z' invalid → "a"
    assert_eq!(pat2, "foo", "fallback splits pattern at paired delimiter");
    assert_eq!(repl2, "bar", "fallback extracts replacement from paired delimiter in pattern");
    assert_eq!(mods2, "a", "fallback modifiers filtered: only 'a' is valid from 'baz'");
}

#[test]
fn test_subst_lenient_non_paired_closed_with_rest_uses_unpaired_body() {
    // Normal closed non-paired: "s/foo/bar/g"
    // branch 1: !is_paired && !rest1.is_empty() → true && true = TRUE → normal path
    // branch 2 is NOT reached
    // With mutant 7 (delete ! on pattern_closed): branch 2 = `!is_paired && pattern_closed`
    //   For normal case: pattern_closed=true → would evaluate branch 2 INSTEAD of else
    //   But wait — branch 1 is checked FIRST. If branch 1 matches, branch 2 is never reached.
    //   So mutant 7 only matters when branch 1 is false.
    // Branch 1 false: is_paired=true OR rest1.is_empty()
    // Case: non-paired, rest1="" (no replacement), pattern_closed=true
    //   → "s/foo/"  — non-paired, closed, empty replacement
    //   branch 1: !is_paired && !rest1.is_empty() → true && false = false
    //   branch 2 (current): !is_paired && !pattern_closed → true && false = false
    //   else: → ("", "")
    //   branch 2 (mutant 7): !is_paired && pattern_closed → true && true = TRUE
    //   → fallback to split_unclosed_substitution_pattern on "foo")
    //   → None → ("", Cow::Borrowed(""))
    //   These produce same output! Still equivalent.
    //
    // The real distinction is for "s/foo/bar/g" — verify the normal path works
    let (pat, repl, mods) = extract_substitution_parts("s/foo/bar/g");
    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
}

#[test]
fn test_subst_lenient_with_and_mutation_in_branch_condition() {
    // Additional case targeting mutant 5 (replace && with || at line 277)
    // Current: `if !is_paired && !rest1.is_empty()`
    // Mutant 5 is actually at line 273 (checking the correct line):
    //   line 273: `if !is_paired && !rest1.is_empty()`
    //   line 277: `else if !is_paired && !pattern_closed`
    // Actually looking at lib.rs:277:26 (replace && with ||)
    //   lib.rs:277:15 (delete !)
    //   lib.rs:277:29 (delete !)
    //
    // For mutant lib.rs:277:26 (replace && with ||):
    //   Current: `!is_paired && !pattern_closed` → triggers only for non-paired+unclosed
    //   Mutant:  `!is_paired || !pattern_closed` → triggers if EITHER non-paired OR unclosed
    //   This would incorrectly trigger for paired+unclosed cases
    //
    // Test: paired substitution that IS closed should NOT trigger the fallback
    let (pat, repl, mods) = extract_substitution_parts("s{foo}{bar}g");
    // Paired, closed. Current: branch 2 NOT taken (is_paired=true, so !is_paired=false)
    // Mutant (||): !is_paired || !pattern_closed = false || false = false → also not taken
    // Hmm. Need paired+closed case where mutant would fire:
    // is_paired=false (non-paired) AND pattern_closed=true (closed):
    //   Current branch 2: !false && !true = true && false = false → not taken
    //   Mutant branch 2:  !false || !true = true || false = true → TAKEN
    //   → calls split_unclosed_substitution_pattern on pattern
    //   For "s/foo/bar/g": rest1 = "/bar/g" which is non-empty so branch 1 fires first
    // Need: non-paired, pattern_closed=true, rest1="" (empty rest)
    //   → "s/foo/" (non-paired, pattern "foo" is closed, rest is "")
    let (pat2, repl2, mods2) = extract_substitution_parts("s/foo/");
    // Current behavior:
    //   branch 1: !is_paired && !rest1.is_empty() → true && false = false (rest1="" after closing /)
    //   Hmm wait: for "s/foo/", extract_delimited_content_strict("/foo/") returns:
    //     opens on '/', reads "foo", hits closing '/' → body="foo", rest="", found_closing=true
    //   So rest1="", pattern_closed=true
    //   branch 1: true && false = false
    //   branch 2 (current): !true && !true = false → not taken
    //   Wait: !is_paired = !false = true; !pattern_closed = !true = false → true && false = false
    //   So branch 2 not taken → else → ("", Cow::Borrowed(""))
    //   pattern="foo", repl="", mods=""
    //
    //   With mutant (||): !is_paired || !pattern_closed = true || false = TRUE → branch 2 taken!
    //   split_unclosed_substitution_pattern("foo") → None (no paired delimiters)
    //   → (String::new(), Cow::Borrowed("")) → pattern="foo", repl="", mods=""
    //   SAME result! Equivalent in this case.
    assert_eq!(pat2, "foo");
    assert_eq!(repl2, "");
    assert_eq!(mods2, "");
    // The key case where mutant 5 (||) diverges:
    // Need split_unclosed_substitution_pattern to return SOMETHING different from else branch
    // "s/foo{bar}/" — non-paired '/', pattern="foo{bar}" (with paired inside), pattern_closed=true
    //   rest1 = "" (after closing '/')
    //   branch 1: true && false = false
    //   branch 2 (current): !is_paired && !pattern_closed = true && false = false → NOT taken
    //   else: repl="", mods=""
    //   With mutant (||): true || false = TRUE → taken
    //   split_unclosed_substitution_pattern("foo{bar}") → Some(("foo", "bar", ""))
    //   → pattern="foo", repl="bar"
    //   DIFFERENT from current behavior!
    let (pat3, repl3, mods3) = extract_substitution_parts("s/foo{bar}/");
    // Current: repl="" (else branch), mods=""
    // Mutant: repl="bar" (fallback branch)
    assert_eq!(pat3, "foo{bar}", "pattern includes nested braces when slash-closed");
    assert_eq!(repl3, "", "replacement is empty (no text after closing delimiter)");
    assert_eq!(mods3, "", "no modifiers");

    assert_eq!(pat, "foo");
    assert_eq!(repl, "bar");
    assert_eq!(mods, "g");
}

// ──────────────────────────────────────────────────────────────
// Additional boundary tests for completeness
// ──────────────────────────────────────────────────────────────

#[test]
fn test_regex_m_length_boundary_exact_two_chars() -> Result<(), Box<dyn std::error::Error>> {
    // "m/" — length 2, second char is '/' (non-alphabetic) → m-prefix branch IS taken
    // content = "/", delimiter = '/', body = "", pattern = "//"
    let (pat, body, mods) = extract_regex_parts("m/");
    assert_eq!(pat, "//", "m/ with empty body has pattern '//'");
    assert_eq!(body, "", "empty body");
    assert_eq!(mods, "", "no modifiers");
    Ok(())
}

#[test]
fn test_regex_m_with_content_uses_m_as_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // "m/hello/" — m-prefix branch, content = "/hello/", delimiter='/', body="hello"
    let (pat, body, mods) = extract_regex_parts("m/hello/");
    assert_eq!(pat, "/hello/");
    assert_eq!(body, "hello");
    assert_eq!(mods, "");
    Ok(())
}

#[test]
fn test_strict_subst_paired_unclosed_returns_error() {
    // Paired unclosed: s{foo (no closing brace, no replacement)
    let result = extract_substitution_parts_strict("s{foo");
    assert_eq!(
        result,
        Err(SubstitutionError::MissingClosingDelimiter),
        "paired unclosed pattern returns MissingClosingDelimiter"
    );
}

#[test]
fn test_strict_subst_backslash_before_closing_delimiter_in_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    // s/pat/re\nplace/ — backslash before 'n' (not the delimiter) should work fine
    let result = extract_substitution_parts_strict(r"s/pat/re\nplace/");
    assert!(result.is_ok());
    let (pat, repl, _) = result.map_err(|e| format!("{:?}", e))?;
    assert_eq!(pat, "pat");
    assert_eq!(repl, r"re\nplace");
    Ok(())
}
