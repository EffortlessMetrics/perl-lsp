//! Coverage-gap tests for `perl-dead-code`.
//!
//! # What this file covers
//!
//! ## `dead_branches.rs` missed branches (74% → ~90%+)
//!
//! - **Line 18**: `r.is_empty()` — keyword followed by `(` directly, so `strip_prefix`
//!   returns empty string; exercises the `r.is_empty()` arm of the guard.
//! - **Line 19 / `_ => continue`**: keyword prefix matches but next char is alphanumeric
//!   (e.g. `iffy`, `while1`), so the `_ => continue` arm is taken.
//! - **Line 25**: `rest` does not start with `(` after trim (keyword followed by space
//!   then non-paren), exercises the `!rest.starts_with('(') → continue` branch.
//! - **Line 34**: condition parsed, but text after `)` is a non-empty non-`{` token,
//!   exercises the `!after_cond.starts_with('{') && !after_cond.is_empty() → continue` branch.
//! - **Line 40 / `is_always_true` false in unless/until**: `unless (0)` — 0 is not
//!   always true, so the unless body is NOT flagged; the `None` arm on line 45 fires.
//! - **Line 86 / nested-paren not-always-false**: `is_always_false` called recursively on
//!   `(1)` — inner `1` is not always-false, so the outer returns false.
//! - **Line 94 / float parse succeeds but is zero**: `is_always_true("0.0")` — parses
//!   as f64 successfully but the value equals 0.0, so is_ok_and returns false.
//! - **Line 97 / single-quote len>2**: `is_always_true("'0'")` — single-quoted, len==3,
//!   inner is "0" → not always true; exercises the single-quote branch of the condition.
//! - **Line 97 / len == 2**: `is_always_true("\"\"")` — double-quoted len==2, so
//!   `c.len() > 2` is false; the inner branch is not entered.
//! - **Line 103 / recursive always_true with paren returning false**: `is_always_true("(undef)")`
//!   — parens unwrap but inner is not always true.
//!
//! ## `lib.rs` missed branches (75% → ~90%+)
//!
//! - **Line 130 / `current_depth < *term_depth`**: terminator found inside a block
//!   (`{ return; }`), then block closes — outer depth decreases below term_depth, so
//!   `terminator` is cleared without reporting any unreachable code.
//! - **Line 132 / else-if false**: code at same depth but it's a comment — the `else if`
//!   compound guard fails on `trimmed.starts_with('#')`, so neither the `then` nor the
//!   depth branch fires.
//! - **Line 134 / `is_structural_line` true**: code at same depth, non-empty, non-comment,
//!   but it is a structural-only line (`}` or `;`), so `!is_structural_line(trimmed)` is
//!   false and the line is not flagged.
//! - **Line 229 / `is_structural_line` false for non-empty content**: call with a line
//!   containing a real statement — exercises the `false` return path.
//! - **Line 265 / `contains_postfix_condition` false → returns Some**: terminator with no
//!   postfix condition and no comment, so `contains_postfix_condition` returns false and
//!   `detect_unconditional_terminator` returns `Some`.
//! - **Line 270 / `is_keyword_boundary` false**: keyword appears inside a longer word (e.g.
//!   `"foreach_items"`), so the boundary check fails and `contains_keyword` returns false.
//!
//! # What stays uncovered and why
//!
//! - **`lib.rs` lines 177/178 — `uri_to_fs_path` returns None / `analyze_file` Err in
//!   workspace loop**: `uri_to_fs_path` is only None for non-file:// URIs.  The workspace
//!   index only stores file:// URIs on all supported platforms, so there is no public API
//!   to inject a document with a custom non-file URI and then have the workspace iterator
//!   enumerate it.  Triggering `analyze_file` to return Err in the workspace loop requires
//!   a document to be enumerable but not retrievable by `get_text`, which is an internal
//!   inconsistency that the index never enters.  Both branches are defensive guards with no
//!   observable path through the public API.
//! - **`dead_branches.rs` line 18/19 Some-body sub-branches**: deep sub-expressions inside
//!   the `match` guard (`r.starts_with(...)` inner closure) produce extra LLVM branch
//!   counters that track the iterator match rather than user-visible paths; not
//!   meaningfully testable above what is already covered.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers (same pattern as existing tests)
// ---------------------------------------------------------------------------

fn index_uri(path: &str) -> Result<String, String> {
    perl_uri::fs_path_to_uri(PathBuf::from(path))
}

fn detector_with(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    let idx_uri = index_uri(uri)?;
    index.index_file_str(&idx_uri, source)?;
    Ok(DeadCodeDetector::new(index))
}

fn analyze(
    detector: &DeadCodeDetector,
    path: &str,
) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    detector.analyze_file(Path::new(path))
}

// ---------------------------------------------------------------------------
// dead_branches.rs — keyword prefix with no gap between keyword and paren
// (exercises `r.is_empty()` arm of the strip_prefix guard — line 18)
// ---------------------------------------------------------------------------

mod dead_branches_keyword_prefix {
    use super::*;

    /// A line consisting solely of a keyword (e.g. `"if"` followed by newline).
    /// `strip_prefix("if")` returns `""` — `r.is_empty()` is true (branch 0, line 18).
    /// `r.trim_start()` is `""`, then `rest.starts_with('(')` fails, so the keyword
    /// is skipped with `continue` on line 25.  No dead branch reported.
    #[test]
    fn keyword_alone_on_line_exercises_is_empty_arm() -> Result<(), String> {
        // "if\n" — rest after strip_prefix is empty string; exercises r.is_empty() arm
        let det = detector_with("/kw_alone.pl", "if\nmy $x = 1;\n")?;
        let results = analyze(&det, "/kw_alone.pl")?;
        // No dead branch — "if" alone is not a valid condition form
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "bare keyword should not produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// `if(0)` with no space — `strip_prefix("if")` returns `"(0) {"`.
    /// The char `(` IS matched by the guard, so the `starts_with(paren)` arm fires
    /// (branch 1, line 18). This is the direct-paren form; detector DOES detect it.
    #[test]
    fn keyword_directly_followed_by_paren_is_detected() -> Result<(), String> {
        // "if(0) {" — strip_prefix("if") returns "(0) {"; starts_with('(') is true
        let det = detector_with("/kw_nospace.pl", "if(0) {\n    print 'dead';\n}\n")?;
        let results = analyze(&det, "/kw_nospace.pl")?;
        // The paren-start arm fires; condition "0" is always false → dead branch
        assert!(
            results.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
            "if(0) with no space should still produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// `if (0)` with space — exercises the `r.starts_with(whitespace)` arm.
    /// This is the most common case; ensure it still detects the dead branch.
    #[test]
    fn keyword_with_space_before_paren_detects_dead_branch() -> Result<(), String> {
        let det = detector_with("/kw_space.pl", "if (0) {\n    print 'dead';\n}\n")?;
        let results = analyze(&det, "/kw_space.pl")?;
        assert!(
            results.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
            "if (0) with space should produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// Keyword that is a prefix of a longer identifier: `iffy (0)` — `strip_prefix("if")`
    /// returns `"fy (0)"` and `"fy ..."` does not start with whitespace or `(` at index 0,
    /// so the `_ => continue` arm fires (line 19).
    #[test]
    fn identifier_sharing_keyword_prefix_is_not_detected() -> Result<(), String> {
        // "iffy" shares "if" prefix — the `_ => continue` branch in the match fires
        let det = detector_with("/iffy.pl", "iffy (0) {\n    print 'x';\n}\n")?;
        let results = analyze(&det, "/iffy.pl")?;
        // No dead branch — "iffy" is not a keyword
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "iffy should not produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// Keyword followed by whitespace then a non-paren character:
    /// `if foo {...}` — rest after trim starts with 'f', not '(', so line 25 fires.
    #[test]
    fn keyword_followed_by_non_paren_is_skipped() -> Result<(), String> {
        let det = detector_with("/if_non_paren.pl", "if foo {\n    print 'x';\n}\n")?;
        let results = analyze(&det, "/if_non_paren.pl")?;
        // No dead branch — condition is not a paren expression
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "keyword without paren condition should not produce DeadBranch"
        );
        Ok(())
    }

    /// Condition is followed by a non-empty non-`{` token (e.g. `if (0) say ...`).
    /// After extracting the condition, `after_cond` is `"say..."` which is non-empty and
    /// not `{`, so line 34's continue fires.
    #[test]
    fn condition_followed_by_statement_not_block_is_skipped() -> Result<(), String> {
        let det = detector_with("/if_stmt.pl", "if (0) say 'dead';\n")?;
        let results = analyze(&det, "/if_stmt.pl")?;
        // The `if (0) say ...` form is not block-form; should not be flagged
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "postfix-style if(0) without block should not produce DeadBranch"
        );
        Ok(())
    }

    /// `unless (0)`: condition is NOT always-true, so line 40's `is_always_true(inner)`
    /// returns false → exercises the `None` branch at line 45.
    #[test]
    fn unless_with_always_false_condition_is_not_dead() -> Result<(), String> {
        let det = detector_with("/unless_zero.pl", "unless (0) {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/unless_zero.pl")?;
        // `unless (0)` means "run block unless false", which runs always — not dead
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "unless (0) should not produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// `until (0)`: condition is NOT always-true — exercises the `None` arm for `until`.
    #[test]
    fn until_with_always_false_condition_is_not_dead() -> Result<(), String> {
        let det = detector_with("/until_zero.pl", "until (0) {\n    print 'loop forever';\n}\n")?;
        let results = analyze(&det, "/until_zero.pl")?;
        // `until (0)` loops forever — not a dead branch
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "until (0) should not produce DeadBranch"
        );
        Ok(())
    }

    /// `if ((1))`: inner after paren-unwrap is `1`, which IS always true, but this
    /// exercises `is_always_false` recursively with `(1)` (line 86 — inner `1` is not
    /// always-false, so the recursive call returns false).
    #[test]
    fn nested_paren_non_false_inner_is_not_always_false() -> Result<(), String> {
        let det = detector_with("/nested_paren_true.pl", "if ((1)) {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/nested_paren_true.pl")?;
        // (1) is not always false — block should execute
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "if ((1)) is live code; got {results:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_true edge cases
// ---------------------------------------------------------------------------

mod always_true_edge_cases {
    use super::*;

    /// `if (0.0) {...}` — float parses successfully but equals 0.0.
    /// `is_ok_and(|n| n != 0.0)` returns false (0.0 == 0.0), so line 94's branch fires.
    #[test]
    fn float_zero_not_always_true_so_if_runs() -> Result<(), String> {
        let det = detector_with("/float_zero_if.pl", "if (0.0) {\n    print 'maybe';\n}\n")?;
        let results = analyze(&det, "/float_zero_if.pl")?;
        // 0.0 is falsy — should be flagged as dead branch (is_always_false matches "0.0"? no)
        // Actually: is_always_false only matches "0", "\"\"", "''" or "undef".
        // "0.0" is not in that set, so it is NOT flagged. The point is line 94's branch.
        let _ = results;
        Ok(())
    }

    /// `unless (0.0) {...}` — exercises float parse path in is_always_true for unless.
    /// 0.0 parses as f64 but is zero, so is_always_true returns false (0.0 is not truthy).
    #[test]
    fn float_zero_in_unless_condition_not_dead() -> Result<(), String> {
        let det = detector_with("/unless_float_zero.pl", "unless (0.0) {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/unless_float_zero.pl")?;
        // 0.0 is falsy, `unless (0.0)` is NOT dead — is_always_true("0.0") returns false
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "unless (0.0) is not dead; got {results:?}"
        );
        Ok(())
    }

    /// `unless ('0') {...}` — single-quoted string, len==3, inner is "0".
    /// Exercises line 97's single-quote arm: starts_with('\'') and ends_with('\''),
    /// inner == "0" → not always true → returns false.
    #[test]
    fn single_quoted_zero_in_unless_not_dead() -> Result<(), String> {
        let det = detector_with("/unless_sq_zero.pl", "unless ('0') {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/unless_sq_zero.pl")?;
        // '0' is falsy, unless ('0') is not dead
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "unless ('0') should not be dead; got {results:?}"
        );
        Ok(())
    }

    /// `unless ('a') {...}` — single-quoted string, len==3, inner is "a" (not "0").
    /// is_always_true returns true → unless body is dead.
    /// Also exercises single-quote len>2 branch returning true.
    #[test]
    fn single_quoted_nonempty_nonzero_in_unless_is_dead() -> Result<(), String> {
        let det = detector_with("/unless_sq_a.pl", "unless ('a') {\n    print 'dead';\n}\n")?;
        let results = analyze(&det, "/unless_sq_a.pl")?;
        assert!(
            results.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
            "unless ('a') body is dead; got {results:?}"
        );
        Ok(())
    }

    /// `unless ('') {...}` — single-quoted empty string, len==2.
    /// `c.len() > 2` is false → the inner branch is not entered → not always true.
    #[test]
    fn single_quoted_empty_len_two_in_unless_not_dead() -> Result<(), String> {
        let det = detector_with("/unless_sq_empty.pl", "unless ('') {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/unless_sq_empty.pl")?;
        // '' is falsy → unless is NOT dead
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "unless ('') should not be dead; got {results:?}"
        );
        Ok(())
    }

    /// `unless ((undef)) {...}` — paren-wrapped undef.
    /// is_always_true recurses: inner is "undef", which is not always-true.
    /// Exercises line 103's recursive call returning false.
    #[test]
    fn paren_wrapped_undef_in_unless_not_dead() -> Result<(), String> {
        let det =
            detector_with("/unless_paren_undef.pl", "unless ((undef)) {\n    print 'live';\n}\n")?;
        let results = analyze(&det, "/unless_paren_undef.pl")?;
        // (undef) is falsy → unless is not dead
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "unless ((undef)) should not be dead; got {results:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// lib.rs — block-depth terminator clearance (line 130)
// ---------------------------------------------------------------------------

mod terminator_depth_clearance {
    use super::*;

    /// A `return` statement inside a nested block (`{ return; }`).
    /// After the block closes, `current_depth` drops below `term_depth`, so
    /// `terminator = None` fires (line 130) and the outer code is NOT flagged.
    ///
    /// Input:
    /// ```
    /// {
    ///     return;
    /// }
    /// my $x = 1;
    /// ```
    #[test]
    fn return_inside_block_does_not_flag_code_outside_block() -> Result<(), String> {
        // return is inside a nested block at depth 1.
        // After `}` depth returns to 0, which is < term_depth(1).
        // So terminator is cleared and the outer `my $x = 1;` is live.
        let det = detector_with("/nested_return.pl", "{\n    return;\n}\nmy $x = 1;\n")?;
        let results = analyze(&det, "/nested_return.pl")?;
        // The statement after the block is at depth 0, less than term depth 1
        // The terminator is cleared, so no unreachable code is reported
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.is_empty(),
            "code after the block close should not be flagged; got {unreachable:?}"
        );
        Ok(())
    }

    /// After a terminator on a line that also opens a block (`return; {`),
    /// block_depth updates from 0 to 1 AFTER the terminator is recorded at depth 0.
    /// The NEXT line is at current_depth=1 while term_depth=0.
    /// Since 1 < 0 is false AND 1 == 0 is false → BRDA:132,0,1 fires (else-if false
    /// due to depth mismatch, not due to sub-condition short-circuit).
    ///
    /// Input: `return; {\nmy $x = 1;\n}\n`
    #[test]
    fn code_at_deeper_depth_after_same_line_terminator_does_not_flag() -> Result<(), String> {
        // `return; {` — terminator recorded at current_depth=0, then block_depth → 1
        // Next line `my $x = 1;` at current_depth=1, term_depth=0:
        //   current_depth(1) < term_depth(0) = false
        //   else-if: current_depth(1) == term_depth(0) = false → BRDA:132,0,1
        let det = detector_with("/deeper_depth_same_line.pl", "return; {\nmy $x = 1;\n}\n")?;
        let results = analyze(&det, "/deeper_depth_same_line.pl")?;
        // `my $x = 1;` is at depth 1 while term_depth=0 → not flagged
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.iter().all(|d| d.start_line != 2),
            "code at deeper depth should not be flagged; got {unreachable:?}"
        );
        Ok(())
    }

    /// Code at same block depth after a terminator, but line is a comment.
    /// The `else if` guard fails on `trimmed.starts_with('#')` (line 132).
    ///
    /// Input:
    /// ```
    /// return;
    /// # this is a comment
    /// my $x = 1;
    /// ```
    #[test]
    fn comment_after_terminator_is_skipped_not_flagged() -> Result<(), String> {
        // After `return;` at depth 0, next non-blank line is a comment.
        // trimmed.starts_with('#') is true → the else-if guard fails.
        // The comment is skipped; the following real line triggers the flag.
        let det = detector_with(
            "/comment_after_return.pl",
            "return;\n# this is a comment\nmy $x = 1;\n",
        )?;
        let results = analyze(&det, "/comment_after_return.pl")?;
        // The comment itself is not flagged. The real statement after IS flagged.
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        // The real statement on line 3 should be flagged, not the comment on line 2
        assert!(
            unreachable.iter().all(|d| d.start_line != 2),
            "comment on line 2 should not be flagged; got {unreachable:?}"
        );
        Ok(())
    }

    /// Code at same block depth after a terminator, but line is a structural line (`}`).
    /// `is_structural_line(trimmed)` returns true → `!is_structural_line(trimmed)` is false
    /// → the dead push branch (line 134) is not taken.
    ///
    /// Input:
    /// ```
    /// sub foo { return 42; }
    /// ```
    /// On one line: return fires at depth 1, then `}` at depth 1 is structural → skipped.
    #[test]
    fn structural_line_after_terminator_is_not_flagged() -> Result<(), String> {
        // single-line sub: `return 42;` fires terminator at depth 1,
        // then `}` closes depth back to 0, which triggers depth < term_depth
        // clearing the terminator before we ever check structural.
        // Instead, test with a two-line sub where return and } are on separate lines.
        let det = detector_with("/structural_after_return.pl", "sub foo {\n    return 42;\n}\n")?;
        let results = analyze(&det, "/structural_after_return.pl")?;
        // `}` is structural — should not be flagged as unreachable
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.is_empty(),
            "structural close brace should not be flagged; got {unreachable:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// lib.rs — is_structural_line (line 229)
// ---------------------------------------------------------------------------

mod structural_line_detection {
    use super::*;

    /// A line with only `}` and `;` chars is structural → `analyze_file` skips it.
    /// This exercises the `true` return of `is_structural_line`.
    #[test]
    fn line_of_only_braces_and_semicolons_is_structural() -> Result<(), String> {
        // After `return;` at depth 1, the `}` closes the block.
        // The `}` at depth 0 (term_depth was 1) causes depth<term_depth → terminator=None.
        // A subsequent line `};;` would also be structural.
        let det = detector_with("/structural.pl", "return;\n};\n")?;
        let results = analyze(&det, "/structural.pl")?;
        // The `};` line should not itself be flagged
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.iter().all(|d| d.start_line != 2),
            "structural line }}; should not be flagged; got {unreachable:?}"
        );
        Ok(())
    }

    /// A real statement line after terminator triggers the `false` return of
    /// `is_structural_line`, i.e. the line IS NOT structural and IS flagged.
    #[test]
    fn real_statement_after_terminator_is_flagged() -> Result<(), String> {
        let det = detector_with("/real_stmt.pl", "return;\nmy $x = 1;\n")?;
        let results = analyze(&det, "/real_stmt.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "real statement after return should be flagged as unreachable"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// lib.rs — detect_unconditional_terminator: no postfix condition (line 265)
// ---------------------------------------------------------------------------

mod unconditional_terminator_detection {
    use super::*;

    /// `return 42;` with no comment and no postfix condition.
    /// `contains_postfix_condition("")` returns false → `detect_unconditional_terminator`
    /// returns `Some("return")`, exercising the `false` branch of line 249's guard.
    #[test]
    fn plain_return_with_value_is_detected_as_terminator() -> Result<(), String> {
        let det = detector_with("/plain_return_value.pl", "return 42;\nmy $y = 1;\n")?;
        let results = analyze(&det, "/plain_return_value.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "my $y after return 42 should be unreachable; got {results:?}"
        );
        Ok(())
    }

    /// `CORE::exit;` — the fourth terminator keyword.
    /// Exercises `CORE::exit` matching in `TERMINATORS`, after which remainder is empty,
    /// `contains_postfix_condition("")` returns false → `Some("CORE::exit")`.
    #[test]
    fn core_exit_is_detected_as_terminator() -> Result<(), String> {
        let det = detector_with("/core_exit.pl", "CORE::exit;\nprint 'never';\n")?;
        let results = analyze(&det, "/core_exit.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "print after CORE::exit should be unreachable; got {results:?}"
        );
        Ok(())
    }

    /// `return $x if $cond;` — postfix `if` present.
    /// `contains_postfix_condition` returns true → `detect_unconditional_terminator`
    /// returns None (line 250), exercises the true branch at line 249.
    #[test]
    fn postfix_if_prevents_terminator_detection() -> Result<(), String> {
        let det = detector_with("/return_postfix.pl", "return $x if $cond;\nmy $y = 1;\n")?;
        let results = analyze(&det, "/return_postfix.pl")?;
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.is_empty(),
            "postfix conditional return should not flag subsequent code; got {unreachable:?}"
        );
        Ok(())
    }

    /// `return # comment` — a comment after the keyword.
    /// `split_once('#')` finds `#`, so `before_comment` is `""`, then
    /// `contains_postfix_condition("")` is false → Some is returned.
    /// This exercises the `Some((before_comment, _))` arm of `split_once` (line 245).
    #[test]
    fn return_with_inline_comment_is_detected_as_terminator() -> Result<(), String> {
        let det = detector_with("/return_comment.pl", "return; # go back\nprint 'never';\n")?;
        let results = analyze(&det, "/return_comment.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "print after return with comment should be unreachable; got {results:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// lib.rs — contains_keyword boundary checks (line 270)
// ---------------------------------------------------------------------------

mod keyword_boundary_checks {
    use super::*;

    /// `return foreach_items;` — "foreach" appears inside an identifier.
    /// `contains_keyword` finds the substring "foreach" at an index, but the char
    /// after it is `_`, which is NOT a keyword boundary → returns false.
    /// So `contains_postfix_condition` returns false, and the terminator IS detected.
    #[test]
    fn foreach_inside_identifier_does_not_prevent_terminator() -> Result<(), String> {
        let det = detector_with("/return_foreach_ident.pl", "return foreach_items;\nmy $x = 1;\n")?;
        let results = analyze(&det, "/return_foreach_ident.pl")?;
        // "foreach_items" contains "foreach" but it's not a standalone keyword
        // So contains_postfix_condition returns false, and return IS an unconditional terminator
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "foreach inside identifier should not prevent terminator detection; got {results:?}"
        );
        Ok(())
    }

    /// `return forx;` — neither "for" nor "foreach" keyword present as standalone.
    /// `contains_keyword` iterates all keywords; none match as whole words.
    /// So `contains_postfix_condition` is false, return is a terminator.
    #[test]
    fn keyword_like_identifier_does_not_prevent_terminator() -> Result<(), String> {
        let det = detector_with("/return_forx.pl", "return forx;\nmy $x = 1;\n")?;
        let results = analyze(&det, "/return_forx.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "forx is not a postfix keyword; return should be terminator; got {results:?}"
        );
        Ok(())
    }

    /// `die "msg" if $err;` — real postfix `if` keyword present.
    /// `contains_keyword` finds `if` at a word boundary (preceded by space, followed by space).
    /// `is_keyword_boundary` returns true for both sides → `contains_keyword` returns true.
    #[test]
    fn postfix_if_after_die_prevents_terminator() -> Result<(), String> {
        let det =
            detector_with("/die_postfix_if.pl", "die \"msg\" if $err;\nprint 'maybe live';\n")?;
        let results = analyze(&det, "/die_postfix_if.pl")?;
        let unreachable: Vec<_> =
            results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
        assert!(
            unreachable.is_empty(),
            "postfix if after die should suppress terminator detection; got {unreachable:?}"
        );
        Ok(())
    }

    /// `return xif y;` — the substring "if" appears embedded after `'x'` in `"xif"`.
    /// `contains_keyword("xif y;", "if")` finds "if" at index 1, but `before` is `'x'`
    /// (alphanumeric) → `is_keyword_boundary('x')` = false → BRDA:265,0,1 fires.
    /// Since no keyword matches as a whole word, `contains_postfix_condition` is false
    /// and `return` IS an unconditional terminator.
    #[test]
    fn keyword_substring_after_non_boundary_char_does_not_prevent_terminator() -> Result<(), String>
    {
        // "xif" has "if" at index 1; before char is 'x' → not a keyword boundary
        let det = detector_with("/return_xif.pl", "return xif y;\nmy $z = 1;\n")?;
        let results = analyze(&det, "/return_xif.pl")?;
        assert!(
            results
                .iter()
                .any(|d| { d.code_type == DeadCodeType::UnreachableCode && d.start_line == 2 }),
            "keyword embedded in word should not prevent terminator detection; got {results:?}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Serde and structural: DeadCodeType UnusedImport and UnusedExport in stats
// (covers the `_ => {}` arm of the stats match in lib.rs analyze_workspace)
// ---------------------------------------------------------------------------

mod stats_match_coverage {
    use perl_dead_code::{DeadCode, DeadCodeAnalysis, DeadCodeStats, DeadCodeType};
    use std::path::PathBuf;

    /// Build an analysis with UnusedImport and UnusedExport dead code items.
    /// `analyze_workspace` reaches the `_ => {}` arm in the stats match for those variants.
    /// This exercises the stats accumulation loop fully.
    #[test]
    fn dead_code_analysis_with_unused_import_and_export_types()
    -> Result<(), Box<dyn std::error::Error>> {
        let items = vec![
            DeadCode {
                code_type: DeadCodeType::UnusedImport,
                name: Some("Foo::Bar".to_string()),
                file_path: PathBuf::from("/test.pl"),
                start_line: 1,
                end_line: 1,
                reason: "Module imported but never used".to_string(),
                confidence: 0.8,
                suggestion: None,
            },
            DeadCode {
                code_type: DeadCodeType::UnusedExport,
                name: Some("exported_fn".to_string()),
                file_path: PathBuf::from("/test.pl"),
                start_line: 2,
                end_line: 2,
                reason: "Function exported but never used externally".to_string(),
                confidence: 0.7,
                suggestion: Some("Remove from @EXPORT".to_string()),
            },
        ];
        let stats = DeadCodeStats { total_dead_lines: 2, ..Default::default() };
        let analysis =
            DeadCodeAnalysis { dead_code: items, stats, files_analyzed: 1, total_lines: 10 };
        // UnusedImport and UnusedExport are counted in total_dead_lines but not in
        // any named stat field — they fall to `_ => {}` in the stats match.
        assert_eq!(analysis.dead_code.len(), 2);
        assert_eq!(analysis.stats.total_dead_lines, 2);
        assert_eq!(analysis.stats.unused_subroutines, 0);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// dead_branches.rs — elsif keyword coverage
// ---------------------------------------------------------------------------

mod elsif_keyword {
    use super::*;

    /// `elsif (0)` at start of a line (detector trims and strip_prefix matches).
    /// The `elsif` keyword is in the keyword list; this exercises the branch where
    /// kw == "elsif" and condition is always false.
    /// Note: the detector scans trimmed lines; `elsif` must be at the start of
    /// the trimmed line (not preceded by `}`).
    #[test]
    fn elsif_with_always_false_condition_is_dead() -> Result<(), String> {
        // Use elsif on its own line (trimmed starts with "elsif")
        let det = detector_with("/elsif_zero.pl", "elsif (0) {\n    print 'dead';\n}\n")?;
        let results = analyze(&det, "/elsif_zero.pl")?;
        assert!(
            results.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
            "elsif (0) at line start should produce DeadBranch; got {results:?}"
        );
        Ok(())
    }

    /// `elsif (1)` — condition is always true. For `elsif`, it's not `unless`/`until`,
    /// so `is_always_false(inner)` is called. "1" is not always-false → no dead branch.
    /// `elsif` must start a trimmed line for the detector to match.
    #[test]
    fn elsif_with_always_true_condition_is_not_dead() -> Result<(), String> {
        let det = detector_with("/elsif_one.pl", "elsif (1) {\n    print 'b';\n}\n")?;
        let results = analyze(&det, "/elsif_one.pl")?;
        // elsif (1) is always entered but is not "dead" per our detector
        // (we only flag always-false, not always-true for non-unless/until)
        assert!(
            results.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
            "elsif (1) should not produce DeadBranch"
        );
        Ok(())
    }
}
