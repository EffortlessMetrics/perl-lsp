//! Error-path and edge-case tests for `perl-dead-code` (#814).
//!
//! Covers paths that the existing test suites leave untested:
//! - Syntactically malformed / recovery-parsed Perl source (no panic, graceful result)
//! - Empty-program variants (whitespace-only, newline-only, truly empty)
//! - Unterminated / dangling control structures (missing `}`, dangling `elsif`)
//! - Multiple terminators in succession (no double-count)
//! - Visited-set / block-advancement invariants (multiple dead branches, no infinite loop)
//! - Unicode content in source (non-ASCII identifiers, comments)
//! - `analyze_file` called on an unindexed path (returns `Err`, no panic)
//! - Terminator at last line with no trailing code (no false positive)
//! - Deeply chained if/elsif chains (termination, no combinatorial explosion)

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers (matches the pattern already established in the other test files)
// ---------------------------------------------------------------------------

fn test_uri_to_index_uri(uri: &str) -> Result<String, String> {
    match uri.strip_prefix("file://") {
        Some(path) => perl_uri::fs_path_to_uri(PathBuf::from(path)),
        None => Ok(uri.to_string()),
    }
}

fn detector_with_file(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    let index_uri = test_uri_to_index_uri(uri)?;
    index.index_file_str(&index_uri, source)?;
    Ok(DeadCodeDetector::new(index))
}

fn analyze(uri: &str, source: &str) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    let detector = detector_with_file(uri, source)?;
    let path_str = uri.strip_prefix("file://").ok_or("bad uri")?;
    detector.analyze_file(Path::new(path_str))
}

// ---------------------------------------------------------------------------
// 1. Malformed Perl — recovery-parsed inputs must not panic
//
// The workspace indexer runs the Perl parser in error-recovery mode before
// the dead-code scanner sees the text. These inputs contain syntax errors that
// force the parser to emit ERROR nodes internally. The dead-code analyzer must
// return gracefully (either Ok with a possibly-empty result, or Err) without
// panicking regardless of the parser ERROR nodes present.
// ---------------------------------------------------------------------------

/// Source that starts with a bare block terminator — structurally malformed.
/// The analyzer must return without panicking.
#[test]
fn malformed_bare_closing_brace_does_not_panic() -> Result<(), String> {
    // Leading `}` without an opening `{` — parser ERROR node territory.
    let source = "}\nmy $x = 1;\nprint $x;\n";
    // We cannot know whether indexing succeeds or fails; either outcome is valid.
    let result = detector_with_file("file:///malformed_leading_brace.pl", source);
    match result {
        Ok(detector) => {
            // If indexing succeeded, analysis must also not panic.
            let _dead = detector.analyze_file(Path::new("/malformed_leading_brace.pl"));
        }
        Err(_) => {
            // Rejected at index time — that is also acceptable.
        }
    }
    Ok(())
}

/// Source with mismatched string delimiters — a common parse-error case.
#[test]
fn malformed_unclosed_string_does_not_panic() -> Result<(), String> {
    // Unterminated double-quoted string.
    let source = "my $x = \"unterminated\nprint $x;\n";
    let result = detector_with_file("file:///malformed_string.pl", source);
    if let Ok(detector) = result {
        let _dead = detector.analyze_file(Path::new("/malformed_string.pl"));
    }
    Ok(())
}

/// Source that is just a sequence of Perl sigils with no valid expression.
#[test]
fn malformed_sigil_soup_does_not_panic() -> Result<(), String> {
    let source = "$@%$@%\n";
    let result = detector_with_file("file:///malformed_sigil.pl", source);
    if let Ok(detector) = result {
        let _dead = detector.analyze_file(Path::new("/malformed_sigil.pl"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Empty-program variants
//
// These tests verify that analyzing empty-ish inputs terminates and produces
// empty (or at most zero) dead-code reports. No panic, no false positives.
// ---------------------------------------------------------------------------

/// A completely empty source string produces zero dead code items.
#[test]
fn empty_source_produces_no_dead_code() -> Result<(), String> {
    let items = analyze("file:///empty_program.pl", "")?;
    assert!(items.is_empty(), "empty source should produce no dead code items; got {items:?}");
    Ok(())
}

/// Source containing only whitespace (spaces and tabs, no newlines) produces no dead code.
#[test]
fn whitespace_only_source_produces_no_dead_code() -> Result<(), String> {
    let items = analyze("file:///whitespace_only.pl", "   \t   \t   ")?;
    assert!(
        items.is_empty(),
        "whitespace-only source should produce no dead code items; got {items:?}"
    );
    Ok(())
}

/// Source containing only newlines (blank lines) produces no dead code.
#[test]
fn newlines_only_source_produces_no_dead_code() -> Result<(), String> {
    let items = analyze("file:///newlines_only.pl", "\n\n\n\n\n")?;
    assert!(
        items.is_empty(),
        "newline-only source should produce no dead code items; got {items:?}"
    );
    Ok(())
}

/// An `if` block with an empty body `{}` should not panic and may or may not
/// be classified as dead — but the analyzer must return.
#[test]
fn empty_if_block_body_does_not_panic() -> Result<(), String> {
    // `if (0) {}` — constant-false condition but empty body.
    let items = analyze("file:///empty_if_body.pl", "if (0) {}\n")?;
    // We assert only that analysis succeeds and returns a valid result.
    // A dead branch may or may not be detected for an empty block.
    let _ = items; // result is valid either way
    Ok(())
}

/// An `if` block with an empty condition `if () {}` must not panic.
#[test]
fn empty_condition_and_empty_body_does_not_panic() -> Result<(), String> {
    let items = analyze("file:///empty_cond_empty_body.pl", "if () {}\n")?;
    // Neither always-false nor always-true — no dead branch expected.
    assert!(
        items.iter().all(|d| d.code_type != DeadCodeType::DeadBranch),
        "empty condition should not produce a dead branch; got {items:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Malformed control flow — dangling elsif / else, unterminated blocks
//
// These test that error-recovery paths in the analyzer handle structural
// anomalies gracefully. The key invariant: no panic, no infinite loop.
// ---------------------------------------------------------------------------

/// An `elsif` with no preceding `if` (dangling) must not crash the analyzer.
#[test]
fn dangling_elsif_no_preceding_if_does_not_panic() -> Result<(), String> {
    // Semantically invalid Perl but may be accepted or rejected at index time.
    let source = "elsif (0) {\n    print 'dead';\n}\n";
    let result = detector_with_file("file:///dangling_elsif.pl", source);
    if let Ok(detector) = result {
        let _dead = detector.analyze_file(Path::new("/dangling_elsif.pl"));
    }
    Ok(())
}

/// A dangling `else` with no matching `if` must not crash the analyzer.
#[test]
fn dangling_else_no_preceding_if_does_not_panic() -> Result<(), String> {
    let source = "else {\n    print 'orphan';\n}\n";
    let result = detector_with_file("file:///dangling_else.pl", source);
    if let Ok(detector) = result {
        let _dead = detector.analyze_file(Path::new("/dangling_else.pl"));
    }
    Ok(())
}

/// An unterminated block (missing closing `}`) followed by live code must not
/// cause the analyzer to loop or panic. The missing brace is the case already
/// tested in `find_block_end` unit tests, but here we verify the integration
/// path via the public API.
#[test]
fn unterminated_block_returns_result_without_panic() -> Result<(), String> {
    // `if (1) {` with no closing brace — unterminated block.
    let source = "if (1) {\n    print 'live';\n";
    let result = detector_with_file("file:///unterminated_block.pl", source);
    if let Ok(detector) = result {
        // Must complete without looping or panicking.
        let _dead = detector.analyze_file(Path::new("/unterminated_block.pl"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Visited-set / block-advancement invariants
//
// These tests verify that the scanner advances past each dead block without
// re-visiting it, and that multiple dead branches in sequence are each counted
// exactly once (no double-counting, no missed branch).
// ---------------------------------------------------------------------------

/// Three consecutive dead branches — all three must be detected exactly once.
/// This exercises the `i = end_line; continue` advancement through the block loop.
#[test]
fn three_consecutive_dead_branches_each_counted_once() -> Result<(), String> {
    let source = concat!(
        "if (0) {\n",
        "    print 'dead1';\n",
        "}\n",
        "while (0) {\n",
        "    print 'dead2';\n",
        "}\n",
        "unless (1) {\n",
        "    print 'dead3';\n",
        "}\n",
    );
    let items = analyze("file:///three_dead_branches.pl", source)?;
    let dead_branches: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(
        dead_branches.len(),
        3,
        "all three dead branches must be detected exactly once; got {dead_branches:?}"
    );
    // Verify they are in order and have distinct start lines.
    assert_eq!(dead_branches[0].start_line, 1, "first dead branch should start at line 1");
    assert_eq!(dead_branches[1].start_line, 4, "second dead branch should start at line 4");
    assert_eq!(dead_branches[2].start_line, 7, "third dead branch should start at line 7");
    Ok(())
}

/// A dead branch followed by live code followed by another dead branch — the
/// live code must not be miscounted and the second dead branch must be found.
#[test]
fn dead_branch_live_code_dead_branch_sequence_correct() -> Result<(), String> {
    let source = concat!(
        "if (0) {\n",
        "    print 'dead1';\n",
        "}\n",
        "my $x = live_call();\n",
        "if (0) {\n",
        "    print 'dead2';\n",
        "}\n",
        "print 'live';\n",
    );
    let items = analyze("file:///dead_live_dead.pl", source)?;
    let dead_branches: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(
        dead_branches.len(),
        2,
        "both dead branches separated by live code must be detected; got {dead_branches:?}"
    );
    Ok(())
}

/// A dead branch whose body contains multiple nested dead branches — the outer
/// dead branch is detected, and the scan advances past the entire outer block,
/// so the inner branches are NOT double-counted as separate dead branches.
#[test]
fn nested_dead_branches_inside_outer_dead_branch_not_double_counted() -> Result<(), String> {
    // The outer `if (0)` covers lines 1-6; `if (0)` inside is inside a dead block.
    // Only the outer dead branch should be reported.
    let source = concat!(
        "if (0) {\n",
        "    if (0) {\n",
        "        print 'inner dead';\n",
        "    }\n",
        "    while (0) {\n",
        "        print 'inner dead2';\n",
        "    }\n",
        "}\n",
    );
    let items = analyze("file:///nested_dead_inside_dead.pl", source)?;
    let dead_branches: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    // Exactly one outer dead branch; inner ones are inside the skipped block.
    assert_eq!(
        dead_branches.len(),
        1,
        "only the outer dead branch should be reported; got {dead_branches:?}"
    );
    assert_eq!(dead_branches[0].start_line, 1, "the outer dead branch should be at line 1");
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Multiple terminators in succession — no double-count of unreachable code
// ---------------------------------------------------------------------------

/// Two terminators in a row (`return` then `exit`) — the code after the first
/// terminator is unreachable; the second terminator is itself that unreachable code.
/// The analyzer must report at most one unreachable-code item (the first one wins)
/// and must not loop or double-count.
#[test]
fn two_terminators_in_succession_single_unreachable_item() -> Result<(), String> {
    // `return` terminates; `exit` is unreachable; nothing follows.
    let source = "return 1;\nexit 0;\n";
    let items = analyze("file:///two_terminators.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    // The line-by-line scanner stops after the first flagged line (it `break`s).
    // So only one unreachable item is reported.
    assert_eq!(
        unreachable.len(),
        1,
        "only the first unreachable item should be reported; got {unreachable:?}"
    );
    assert_eq!(unreachable[0].start_line, 2, "exit is the unreachable line");
    Ok(())
}

/// Terminator at the very last line — no code follows, so there must be no
/// unreachable-code false positive.
#[test]
fn terminator_at_last_line_produces_no_unreachable_code() -> Result<(), String> {
    // `return` is on the last (and only) line. Nothing can be unreachable after it.
    let source = "return;\n";
    let items = analyze("file:///terminator_last_line.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "terminator at last line must not produce unreachable-code items; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Unicode / non-ASCII content in source
//
// The analyzer uses `str::lines()` and `str::chars()` throughout. Non-ASCII
// characters in comments, strings, or identifiers must not trigger char-boundary
// panics or cause incorrect line counting.
// ---------------------------------------------------------------------------

/// A source file containing Unicode in a comment must analyze cleanly.
#[test]
fn source_with_unicode_comment_does_not_panic() -> Result<(), String> {
    // Non-ASCII chars in a comment line; Perl source is otherwise valid.
    let source = "# Ünïcödé cömmënt — αβγδ — 日本語\nmy $x = 1;\nreturn $x;\n";
    let items = analyze("file:///unicode_comment.pl", source)?;
    // No dead code expected in this valid program.
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "unicode comment must not cause false unreachable-code reports; got {unreachable:?}"
    );
    Ok(())
}

/// A source file with Unicode in a string literal must analyze cleanly.
#[test]
fn source_with_unicode_string_does_not_panic() -> Result<(), String> {
    let source = "my $greeting = \"こんにちは\";\nreturn $greeting;\n";
    let items = analyze("file:///unicode_string.pl", source)?;
    // `return` at end of file — no following code, so no unreachable items.
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "unicode string literal must not cause false positives; got {unreachable:?}"
    );
    Ok(())
}

/// Unreachable code AFTER a terminator in a source that also has Unicode content.
/// The unreachable item should still be detected and line numbers must be correct.
#[test]
fn unreachable_code_after_terminator_with_unicode_content_correct_line() -> Result<(), String> {
    // Line 1: Unicode comment
    // Line 2: terminator
    // Line 3: unreachable statement
    let source = "# Коментарий на русском\nreturn 1;\nprint 'unreachable';\n";
    let items = analyze("file:///unicode_unreachable.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "unreachable code after return must be detected even with unicode content; got {items:?}"
    );
    assert_eq!(
        unreachable[0].start_line, 3,
        "unreachable code must be on line 3; got line {}",
        unreachable[0].start_line
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. analyze_file error path — unindexed file returns Err, no panic
//
// Already partially covered in `comprehensive_unit_tests.rs`, but we add
// a variant confirming the error message is non-empty (not a bare empty string).
// ---------------------------------------------------------------------------

/// Calling `analyze_file` for a file that was never indexed must return a
/// descriptive `Err`, not panic or return `Ok(empty)`.
#[test]
fn analyze_file_unindexed_path_returns_descriptive_error() {
    let index = WorkspaceIndex::new();
    let detector = DeadCodeDetector::new(index);
    let result = detector.analyze_file(Path::new("/definitely/not/indexed.pl"));
    assert!(result.is_err(), "unindexed path must return Err; got Ok");
    // The error string must be non-empty so callers can surface it.
    if let Err(msg) = result {
        assert!(!msg.is_empty(), "error message must not be empty for unindexed path");
    }
}

// ---------------------------------------------------------------------------
// 8. Deeply chained if/elsif — terminates without combinatorial blowup
//
// A long chain of `if (0) { ... } elsif (0) { ... } elsif ...` on separate lines
// (Allman style, so `elsif` is recognised by the scanner) must complete in
// reasonable time and report the correct number of dead branches.
// ---------------------------------------------------------------------------

/// 20 Allman-style `elsif (0)` branches — all dead, all detected, no hang.
#[test]
fn deeply_chained_elsif_allman_style_all_detected() -> Result<(), String> {
    // Build: if ($x) { ... } \n elsif(0) { ... } \n elsif(0) { ... } ... × 20
    let mut source = "if ($x) {\n    print 'maybe';\n}\n".to_string();
    for i in 0..20 {
        source.push_str(&format!("elsif (0) {{\n    print 'dead{i}';\n}}\n"));
    }
    let items = analyze("file:///deep_elsif_chain.pl", &source)?;
    let dead_branches: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(
        dead_branches.len(),
        20,
        "all 20 dead elsif branches must be detected; got {dead_branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. analyze_workspace — empty workspace terminates cleanly
//
// Confirms the workspace analysis returns a well-formed result even when
// no files are indexed. (This supplements the unit test in comprehensive_unit_tests
// by checking both files_analyzed and the zero-value stats.)
// ---------------------------------------------------------------------------

/// Workspace with zero files must return all-zero stats and an empty dead_code list.
#[test]
fn analyze_workspace_empty_produces_all_zero_stats() {
    let index = WorkspaceIndex::new();
    let detector = DeadCodeDetector::new(index);
    let analysis = detector.analyze_workspace();

    assert_eq!(analysis.files_analyzed, 0, "empty workspace: files_analyzed must be 0");
    assert_eq!(analysis.total_lines, 0, "empty workspace: total_lines must be 0");
    assert!(analysis.dead_code.is_empty(), "empty workspace: dead_code must be empty");
    assert_eq!(analysis.stats.unused_subroutines, 0);
    assert_eq!(analysis.stats.unused_variables, 0);
    assert_eq!(analysis.stats.unused_constants, 0);
    assert_eq!(analysis.stats.dead_branches, 0);
    assert_eq!(analysis.stats.unreachable_statements, 0);
    assert_eq!(analysis.stats.total_dead_lines, 0);
}
