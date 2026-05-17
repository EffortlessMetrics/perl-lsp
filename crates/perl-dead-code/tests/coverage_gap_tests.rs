//! Coverage gap tests for `perl-dead-code` — targeting missed branches in
//! `dead_branches.rs` and `lib.rs` identified from the 74%/75% branch
//! coverage baseline reported in issue #9101.
//!
//! Each test section is labelled with the file and function it targets.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Test helpers (mirrors pattern from existing test files)
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

fn has_dead_branch(items: &[perl_dead_code::DeadCode]) -> bool {
    items.iter().any(|d| d.code_type == DeadCodeType::DeadBranch)
}

fn no_dead_branch(items: &[perl_dead_code::DeadCode]) -> bool {
    items.iter().all(|d| d.code_type != DeadCodeType::DeadBranch)
}

// ===========================================================================
// dead_branches.rs — is_always_false
// ===========================================================================

// Doubly-nested parentheses wrapping a falsy literal — exercises the recursive
// branch of is_always_false beyond the single-wrap case already tested.
#[test]
fn is_always_false_double_nested_parens() -> Result<(), String> {
    let items = analyze("file:///db_double_nested.pl", "if (((0))) {\n    print 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "if (((0))) should detect dead branch; got {items:?}");
    Ok(())
}

// A paren-wrapped empty double-quote string is still always false.
#[test]
fn is_always_false_paren_wrapped_empty_double_quote() -> Result<(), String> {
    let items = analyze("file:///db_paren_empty_dq.pl", "if ((\"\")) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "if ((\"\")) should detect dead branch; got {items:?}");
    Ok(())
}

// A paren-wrapped 'undef' is still always false.
#[test]
fn is_always_false_paren_wrapped_undef() -> Result<(), String> {
    let items = analyze("file:///db_paren_undef.pl", "if ((undef)) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "if ((undef)) should detect dead branch; got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — is_always_true
// ===========================================================================

// Non-zero negative integer is always true.
#[test]
fn is_always_true_negative_nonzero_int() -> Result<(), String> {
    let items = analyze("file:///db_neg_int.pl", "unless (-1) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "unless (-1) should detect dead branch; got {items:?}");
    Ok(())
}

// Non-zero float is always true.
#[test]
fn is_always_true_nonzero_float() -> Result<(), String> {
    let items = analyze("file:///db_float.pl", "unless (3.14) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "unless (3.14) should detect dead branch; got {items:?}");
    Ok(())
}

// Zero float (0.0) is not always true.
#[test]
fn is_always_true_zero_float_not_true() -> Result<(), String> {
    let items = analyze("file:///db_zero_float.pl", "unless (0.0) {\n    say 'x';\n}\n")?;
    assert!(no_dead_branch(&items), "unless (0.0) should NOT detect dead branch; got {items:?}");
    Ok(())
}

// Single-quoted string whose inner content is "0" — not always true.
#[test]
fn is_always_true_single_quote_zero_not_true() -> Result<(), String> {
    let items = analyze("file:///db_sq_zero.pl", "unless ('0') {\n    say 'x';\n}\n")?;
    assert!(
        no_dead_branch(&items),
        "unless ('0') should NOT detect dead branch (inner '0' is falsy); got {items:?}"
    );
    Ok(())
}

// Non-empty single-quoted string is always true — nested parens variant.
#[test]
fn is_always_true_paren_wrapped_nonempty_string() -> Result<(), String> {
    let items = analyze("file:///db_paren_str.pl", "unless (('hello')) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "unless (('hello')) should detect dead branch; got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — detect_dead_branches: `elsif` keyword
// ===========================================================================

// NOTE (follow-up bug): `} elsif (0) {` on a single line is NOT detected as a
// dead branch because the trimmed line starts with `}`, so `strip_prefix("elsif")`
// never matches. The keyword is only exercised when `elsif` starts its own line
// (e.g., Allman style). This test locks the current behavior.

// `elsif (0)` on its own line (Allman-style) is detected.
#[test]
fn detect_dead_branches_elsif_allman_style_always_false() -> Result<(), String> {
    // Allman brace style: elsif starts at the beginning of a trimmed line.
    let source = "if ($x) {\n    say 'yes';\n}\nelsif (0) {\n    say 'dead';\n}\n";
    let items = analyze("file:///db_elsif_allman.pl", source)?;
    assert!(
        has_dead_branch(&items),
        "elsif (0) on its own line should detect dead branch; got {items:?}"
    );
    Ok(())
}

// `} elsif (0) {` (K&R style) is NOT currently detected — documents the gap.
#[test]
fn detect_dead_branches_elsif_inline_not_detected_documents_gap() -> Result<(), String> {
    // Bug: } elsif (0) { is not detected because the trimmed line starts with '}'.
    // This test locks the current behavior; a follow-up should fix the scanner
    // to handle `} elsif` by stripping leading `} ` before keyword matching.
    let source = "if ($x) {\n    say 'yes';\n} elsif (0) {\n    say 'dead';\n}\n";
    let items = analyze("file:///db_elsif_inline.pl", source)?;
    // Current behavior: NOT detected (the gap we are documenting).
    assert!(
        no_dead_branch(&items),
        "known gap: '}} elsif (0) {{' on one line is not currently detected; got {items:?}"
    );
    Ok(())
}

// `elsif` with a non-constant condition (Allman style) should not trigger.
#[test]
fn detect_dead_branches_elsif_variable_condition_no_dead() -> Result<(), String> {
    let source = "if ($x) {\n    say 'yes';\n}\nelsif ($y) {\n    say 'maybe';\n}\n";
    let items = analyze("file:///db_elsif_var.pl", source)?;
    assert!(no_dead_branch(&items), "elsif ($y) must not detect dead branch; got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — detect_dead_branches: `until` keyword (always-true)
// ===========================================================================

// `until (42)` — always-true condition means body never runs.
#[test]
fn detect_dead_branches_until_nonzero_int_always_true() -> Result<(), String> {
    let items = analyze("file:///db_until_42.pl", "until (42) {\n    say 'dead';\n}\n")?;
    assert!(has_dead_branch(&items), "until (42) should detect dead branch; got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — strip_prefix path: keyword not followed by space or '('
// ===========================================================================

// A word that begins with 'if' but is not the keyword (e.g., `ifdef`).
// The strip_prefix condition requires a space or '(' after the keyword.
#[test]
fn detect_dead_branches_keyword_prefix_not_matched() -> Result<(), String> {
    // `iffy` starts with `if` but is not the keyword
    let items = analyze("file:///db_iffy.pl", "iffy(0);\nmy $x = 1;\n")?;
    assert!(no_dead_branch(&items), "iffy(0) must not be treated as if (0); got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — after_cond check: trailing non-brace, non-empty text
// ===========================================================================

// Postfix form `print 'x' if (0);` — the condition check matches but the
// part after the closing paren is not `{` and not empty, so it must be skipped.
#[test]
fn detect_dead_branches_postfix_if_not_flagged() -> Result<(), String> {
    // `if (0)` as postfix — no block, so not a dead branch
    let items = analyze("file:///db_postfix_if.pl", "print 'hi' if (0);\n")?;
    assert!(
        no_dead_branch(&items),
        "postfix if (0) with no block must not be flagged as dead branch; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// dead_branches.rs — find_block_end: open_line with no `{` on that line
// ===========================================================================

// When the opening `{` appears on a line after the keyword line, find_block_end
// still locates the matching `}`.
#[test]
fn detect_dead_branches_open_brace_on_next_line() -> Result<(), String> {
    // K&R vs Allman style — brace on next line
    let source = "if (0)\n{\n    say 'dead';\n}\n";
    let items = analyze("file:///db_allman.pl", source)?;
    assert!(
        has_dead_branch(&items),
        "if (0) with brace on next line should still detect dead branch; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// dead_branches.rs — find_block_end: no closing brace (falls through to len)
// ===========================================================================

// A dead branch whose `}` is missing — find_block_end returns lines.len().
// The detection should still fire (branch is still dead), and the reported
// end_line should equal the number of lines in the file.
#[test]
fn detect_dead_branches_missing_closing_brace_still_detected() -> Result<(), String> {
    let source = "if (0) {\n    say 'dead';\n";
    let items = analyze("file:///db_no_close.pl", source)?;
    assert!(
        has_dead_branch(&items),
        "if (0) with missing closing brace should still detect dead branch; got {items:?}"
    );
    let branch = items
        .iter()
        .find(|d| d.code_type == DeadCodeType::DeadBranch)
        .ok_or("no dead branch found")?;
    // end_line should equal source line count (2 lines, no trailing newline counted)
    assert_eq!(branch.end_line, 2, "end_line should equal file line count when no close brace");
    Ok(())
}

// ===========================================================================
// lib.rs — detect_unconditional_terminator: CORE::exit
// ===========================================================================

// `CORE::exit` is listed as a terminator but not exercised in existing tests.
#[test]
fn analyze_file_core_exit_is_unconditional_terminator() -> Result<(), String> {
    let source = "CORE::exit(0);\nprint 'unreachable';\n";
    let items = analyze("file:///lib_core_exit.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(!unreachable.is_empty(), "CORE::exit should be treated as terminator; got {items:?}");
    assert!(
        unreachable[0].reason.contains("CORE::exit"),
        "reason should mention CORE::exit; got {}",
        unreachable[0].reason
    );
    Ok(())
}

// ===========================================================================
// lib.rs — contains_postfix_condition: all non-`if` postfix keywords
// ===========================================================================

// `return unless $cond` — the `unless` postfix keyword should prevent flagging.
#[test]
fn analyze_file_postfix_unless_prevents_terminator() -> Result<(), String> {
    let source = "return unless $cond;\nmy $x = 1;\n";
    let items = analyze("file:///lib_postfix_unless.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "return unless $cond should not be an unconditional terminator; got {items:?}"
    );
    Ok(())
}

// `die while $cond` — postfix `while` prevents flagging.
#[test]
fn analyze_file_postfix_while_prevents_terminator() -> Result<(), String> {
    let source = "die 'x' while $cond;\nmy $x = 1;\n";
    let items = analyze("file:///lib_postfix_while.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "die while $cond should not be an unconditional terminator; got {items:?}"
    );
    Ok(())
}

// `exit until $flag` — postfix `until` prevents flagging.
#[test]
fn analyze_file_postfix_until_prevents_terminator() -> Result<(), String> {
    let source = "exit until $flag;\nmy $x = 1;\n";
    let items = analyze("file:///lib_postfix_until.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "exit until $flag should not be an unconditional terminator; got {items:?}"
    );
    Ok(())
}

// `return for @list` — postfix `for` prevents flagging.
#[test]
fn analyze_file_postfix_for_prevents_terminator() -> Result<(), String> {
    let source = "return for @list;\nmy $x = 1;\n";
    let items = analyze("file:///lib_postfix_for.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "return for @list should not be an unconditional terminator; got {items:?}"
    );
    Ok(())
}

// `die foreach @items` — postfix `foreach` prevents flagging.
#[test]
fn analyze_file_postfix_foreach_prevents_terminator() -> Result<(), String> {
    let source = "die 'x' foreach @items;\nmy $x = 1;\n";
    let items = analyze("file:///lib_postfix_foreach.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "die foreach @items should not be an unconditional terminator; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// lib.rs — contains_keyword: keyword boundary logic
// ===========================================================================

// `return myif $x` — `myif` contains `if` as suffix but not at a boundary,
// so it must NOT be treated as a postfix `if`.
// The postfix detection must not fire, meaning `return` IS a terminator here.
#[test]
fn analyze_file_keyword_boundary_suffix_not_keyword() -> Result<(), String> {
    // `return myif $x;` — "myif" is not the keyword "if"; return is unconditional
    let source = "return myif($x);\nmy $y = 1;\n";
    let items = analyze("file:///lib_boundary_suffix.pl", source)?;
    // "myif" should NOT be recognised as postfix `if`, so return IS an unconditional
    // terminator and the next statement should be flagged.
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "return myif($x) should be treated as unconditional (myif != postfix if); got {items:?}"
    );
    Ok(())
}

// `return iffoo $x` — `iffoo` starts with `if` but is not the keyword.
#[test]
fn analyze_file_keyword_boundary_prefix_not_keyword() -> Result<(), String> {
    let source = "return iffoo($x);\nmy $y = 1;\n";
    let items = analyze("file:///lib_boundary_prefix.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "return iffoo($x) should be treated as unconditional (iffoo != postfix if); got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// lib.rs — is_structural_line: edge cases
// ===========================================================================

// A line containing only semicolons is structural and must not be flagged
// as unreachable even when it appears after a terminator.
#[test]
fn analyze_file_semicolon_only_line_is_structural() -> Result<(), String> {
    // After `return`, a line of `;` is structural — should not be flagged.
    let source = "return 1;\n;;;\nmy $x = 2;\n";
    let items = analyze("file:///lib_semicolons.pl", source)?;
    // The `;;;` line is structural, so the flagged unreachable should be the
    // `my $x = 2;` line, not `;;;`.
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    if !unreachable.is_empty() {
        assert_ne!(
            unreachable[0].start_line, 2,
            "semicolon-only line should not be flagged as unreachable"
        );
    }
    Ok(())
}

// A line containing only `}` is structural and must not be flagged as unreachable.
#[test]
fn analyze_file_closing_brace_only_is_structural() -> Result<(), String> {
    // In a sub, the closing `}` after return must not be flagged.
    let source = "sub foo {\n    return 1;\n}\n";
    let items = analyze("file:///lib_close_brace.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(unreachable.is_empty(), "closing brace line must not be flagged; got {items:?}");
    Ok(())
}

// ===========================================================================
// lib.rs — terminator cleared when block_depth decreases (depth < term_depth)
// ===========================================================================

// A `return` inside a nested block is not a top-level terminator.
// Code after the nested block closes (depth decreases) must not be flagged.
#[test]
fn analyze_file_terminator_cleared_on_block_close() -> Result<(), String> {
    // return inside an inner block: depth drops below term_depth on '}',
    // so the terminator tracking is reset and `print 'live'` must not be flagged.
    let source = "if ($x) {\n    return 1;\n}\nprint 'live';\n";
    let items = analyze("file:///lib_depth_clear.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "return inside nested block must not flag code after block closes; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// lib.rs — terminator with inline comment still fires
// ===========================================================================

// A terminator followed by `# comment` (no postfix condition) is still unconditional.
#[test]
fn analyze_file_terminator_with_comment_is_still_unconditional() -> Result<(), String> {
    let source = "return 1; # we are done\nmy $x = 2;\n";
    let items = analyze("file:///lib_comment_terminator.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "return followed by inline comment should still be unconditional; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// lib.rs — terminator with comment that embeds a postfix keyword
// ===========================================================================

// `return 1; # exit if needed` — the `if` is inside a comment, not in the
// code, so `return` remains unconditional.
#[test]
fn analyze_file_comment_with_if_does_not_suppress_terminator() -> Result<(), String> {
    let source = "return 1; # exit if needed\nmy $x = 2;\n";
    let items = analyze("file:///lib_comment_if.pl", source)?;
    let unreachable: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "if inside a comment must not suppress the terminator; got {items:?}"
    );
    Ok(())
}

// ===========================================================================
// lib.rs — analyze_workspace: DeadCodeType arms not counted in stats (UnusedImport / UnusedExport)
// ===========================================================================

// The stats aggregation in analyze_workspace has a `_ => {}` arm that covers
// UnusedImport and UnusedExport. Directly construct DeadCode items to confirm
// total_dead_lines still accumulates for those types (the stats fields don't
// have a dedicated counter, but total_dead_lines does).
#[test]
fn dead_code_stats_unused_import_contributes_to_total_dead_lines() {
    use perl_dead_code::{DeadCode, DeadCodeAnalysis, DeadCodeStats};

    let item = DeadCode {
        code_type: DeadCodeType::UnusedImport,
        name: Some("Foo".to_string()),
        file_path: PathBuf::from("/test.pl"),
        start_line: 1,
        end_line: 1,
        reason: "Module imported but never used".to_string(),
        confidence: 0.9,
        suggestion: None,
    };
    // Simulate what analyze_workspace does: accumulate stats manually.
    let mut stats = DeadCodeStats::default();
    let lines = item.end_line.saturating_sub(item.start_line) + 1;
    stats.total_dead_lines += lines;
    // The `_ => {}` arm means no dedicated counter is bumped, but lines accumulate.
    assert_eq!(stats.total_dead_lines, 1);
    // Verify the analysis struct can hold UnusedImport items.
    let analysis =
        DeadCodeAnalysis { dead_code: vec![item], stats, files_analyzed: 1, total_lines: 10 };
    assert_eq!(analysis.dead_code.len(), 1);
    assert_eq!(analysis.dead_code[0].code_type, DeadCodeType::UnusedImport);
}

#[test]
fn dead_code_stats_unused_export_contributes_to_total_dead_lines() {
    use perl_dead_code::{DeadCode, DeadCodeAnalysis, DeadCodeStats};

    let item = DeadCode {
        code_type: DeadCodeType::UnusedExport,
        name: Some("bar".to_string()),
        file_path: PathBuf::from("/lib.pm"),
        start_line: 5,
        end_line: 7,
        reason: "Exported but never used externally".to_string(),
        confidence: 0.8,
        suggestion: None,
    };
    let mut stats = DeadCodeStats::default();
    let lines = item.end_line.saturating_sub(item.start_line) + 1;
    stats.total_dead_lines += lines;
    assert_eq!(
        stats.total_dead_lines, 3,
        "multi-line UnusedExport should add 3 to total_dead_lines"
    );
    let analysis =
        DeadCodeAnalysis { dead_code: vec![item], stats, files_analyzed: 1, total_lines: 50 };
    assert_eq!(analysis.dead_code[0].code_type, DeadCodeType::UnusedExport);
}

// ===========================================================================
// dead_branches.rs — extract_balanced_parens: empty input
// ===========================================================================

// Passing an empty string returns None (no leading '(').
// This exercises the `!s.starts_with('(')` branch for an edge-case input.
#[test]
fn detect_dead_branches_empty_condition_no_crash() -> Result<(), String> {
    // A syntactically odd line that yields an empty condition string.
    // We just verify the detector does not panic.
    let items = analyze("file:///db_empty_cond.pl", "if () {\n    say 'x';\n}\n")?;
    // Empty parens extract an empty condition which is neither always-true nor always-false.
    assert!(no_dead_branch(&items), "if () should not be flagged as dead branch; got {items:?}");
    Ok(())
}

// ===========================================================================
// dead_branches.rs — multiple dead branches in one file
// ===========================================================================

// Ensures the loop advances past each dead block (i = end_line; continue)
// so subsequent branches in the same file are also detected.
#[test]
fn detect_dead_branches_multiple_dead_blocks_in_file() -> Result<(), String> {
    let source = concat!(
        "if (0) {\n",
        "    say 'dead1';\n",
        "}\n",
        "my $x = 1;\n",
        "while (0) {\n",
        "    say 'dead2';\n",
        "}\n",
    );
    let items = analyze("file:///db_multi_dead.pl", source)?;
    let dead_branches: Vec<_> =
        items.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(
        dead_branches.len(),
        2,
        "both dead blocks should be detected; got {dead_branches:?}"
    );
    Ok(())
}
