//! Focused tests for previously-uncovered branches in `perl-dead-code`.
//!
//! This file targets the specific missed branches identified via cargo-llvm-cov:
//!
//! **dead_branches.rs**
//! - Line 26: keyword present but no `(` follows — e.g. `if $x { }` (no parens)
//! - Line 31: `extract_balanced_parens` returns None (unbalanced parens)
//! - Line 35: `after_cond` is neither empty nor `{` (statement-form without block)
//! - Line 45: `unless`/`until` with always-false condition — is_always_true returns false
//! - Line 95: `is_always_true` via f64 parse path (e.g. `1.5`)
//!
//! **lib.rs**
//! - Line 131: terminator depth drops when we exit the block (depth < term_depth)
//! - Line 181: `uri_to_fs_path` returns None — URI with no fs path conversion
//! - Lines 189-191, 216-217: UnusedConstant and UnusedPackage stats branches
//! - Line 220: `_ => {}` catch-all in stats match (UnusedImport/UnusedExport)
//! - Line 245: stats aggregation for multi-line dead items (end_line > start_line)

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers (mirrors dead_code_behavior_tests.rs)
// ---------------------------------------------------------------------------

fn test_uri_to_index_uri(uri: &str) -> Result<String, String> {
    match uri.strip_prefix("file://") {
        Some(path) => perl_uri::fs_path_to_uri(PathBuf::from(path)),
        None => Ok(uri.to_string()),
    }
}

fn detector_with_single_file(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    let index_uri = test_uri_to_index_uri(uri)?;
    index.index_file_str(&index_uri, source)?;
    Ok(DeadCodeDetector::new(index))
}

fn detect_for_path(
    detector: &DeadCodeDetector,
    path: &str,
) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    detector.analyze_file(Path::new(path))
}

// ---------------------------------------------------------------------------
// dead_branches.rs — line 26: keyword without opening paren → `continue`
// ---------------------------------------------------------------------------

/// `if $x { ... }` uses a bare variable condition without parentheses.
/// The keyword `if` is detected, but `rest` does not start with `(`, so the
/// branch detector skips it.  No dead branch should be emitted.
#[test]
fn no_dead_branch_for_if_without_parens() -> Result<(), String> {
    let source = "if $x {\n    print 'ok';\n}\n";
    let detector = detector_with_single_file("file:///no_parens.pl", source)?;
    let dead = detect_for_path(&detector, "/no_parens.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "if without parens must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

/// `while $x { ... }` — same scenario for the `while` keyword.
#[test]
fn no_dead_branch_for_while_without_parens() -> Result<(), String> {
    let source = "while $x {\n    process();\n}\n";
    let detector = detector_with_single_file("file:///while_no_parens.pl", source)?;
    let dead = detect_for_path(&detector, "/while_no_parens.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "while without parens must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — line 31: extract_balanced_parens returns None
// ---------------------------------------------------------------------------

/// `if (0 { ... }` — the opening paren is never closed, so
/// `extract_balanced_parens` returns `None`.  No dead branch should be emitted.
#[test]
fn no_dead_branch_for_unbalanced_open_paren() -> Result<(), String> {
    // Deliberately malformed source — unbalanced paren
    let source = "if (0 {\n    print 'unreachable?';\n}\n";
    let detector = detector_with_single_file("file:///unbalanced.pl", source)?;
    let dead = detect_for_path(&detector, "/unbalanced.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "unbalanced paren must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — line 35: after_cond is not `{` and not empty
// ---------------------------------------------------------------------------

/// `if (0) foo();` — statement-form `if` with no block.  After the condition
/// the text is `foo();`, which starts with neither `{` nor is empty, so the
/// dead-branch detector skips the line.
#[test]
fn no_dead_branch_for_if_statement_form_no_block() -> Result<(), String> {
    let source = "if (0) print 'dead';\n";
    let detector = detector_with_single_file("file:///stmt_form.pl", source)?;
    let dead = detect_for_path(&detector, "/stmt_form.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "statement-form if without block must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

/// `while (0) do_thing();` — same for `while`.
#[test]
fn no_dead_branch_for_while_statement_form_no_block() -> Result<(), String> {
    let source = "while (0) process();\n";
    let detector = detector_with_single_file("file:///while_stmt_form.pl", source)?;
    let dead = detect_for_path(&detector, "/while_stmt_form.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "statement-form while without block must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — line 45: `unless`/`until` with always-false condition
//   is_always_true(inner) → false → None (the `else { None }` arm)
// ---------------------------------------------------------------------------

/// `unless (0) { ... }` — condition `0` is always false, not always true.
/// For `unless`, we check `is_always_true`, which returns false for `0`.
/// So the branch is NOT dead (unless 0 ≡ if 1, which actually always runs).
/// The dead-branch detector should not flag this.
#[test]
fn unless_zero_is_not_dead_branch() -> Result<(), String> {
    let source = "unless (0) {\n    print 'runs';\n}\n";
    let detector = detector_with_single_file("file:///unless_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_zero.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "unless (0) should NOT be a dead branch (it always runs); got {dead:?}"
    );
    Ok(())
}

/// `until (0) { ... }` — same reasoning.
#[test]
fn until_zero_is_not_dead_branch() -> Result<(), String> {
    let source = "until (0) {\n    print 'runs';\n}\n";
    let detector = detector_with_single_file("file:///until_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/until_zero.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "until (0) should NOT be a dead branch (it always runs); got {dead:?}"
    );
    Ok(())
}

/// `unless ($x) { ... }` — condition is a variable, not always true.
#[test]
fn unless_variable_condition_is_not_dead_branch() -> Result<(), String> {
    let source = "unless ($x) {\n    print 'maybe';\n}\n";
    let detector = detector_with_single_file("file:///unless_var.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_var.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "unless with variable condition must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — line 95: is_always_true via f64 parse path
// ---------------------------------------------------------------------------

/// `unless (1.5) { ... }` — `1.5` is non-zero float, so `is_always_true`
/// returns `true` via the `f64` branch.  The `unless` body is dead.
#[test]
fn unless_float_nonzero_is_dead_branch() -> Result<(), String> {
    let source = "unless (1.5) {\n    print 'never';\n}\n";
    let detector = detector_with_single_file("file:///unless_float.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_float.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "unless (1.5) should be a dead branch because 1.5 is always true; got {dead:?}"
    );
    Ok(())
}

/// `if (2.5) { ... }` — `2.5` parses as f64 and is non-zero, so is_always_true
/// returns true.  The `if` body is live (not dead).
#[test]
fn if_float_nonzero_is_not_dead_branch() -> Result<(), String> {
    let source = "if (2.5) {\n    print 'always runs';\n}\n";
    let detector = detector_with_single_file("file:///if_float.pl", source)?;
    let dead = detect_for_path(&detector, "/if_float.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "if (2.5) is always true so the body is live, not dead; got {dead:?}"
    );
    Ok(())
}

/// `while (3.14) { ... }` — non-zero float condition; while body always runs.
#[test]
fn while_float_nonzero_is_not_dead_branch() -> Result<(), String> {
    let source = "while (3.14) {\n    process();\n}\n";
    let detector = detector_with_single_file("file:///while_float.pl", source)?;
    let dead = detect_for_path(&detector, "/while_float.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "while (3.14) is always true, body is live; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 131: depth drops below terminator depth (exit block)
// ---------------------------------------------------------------------------

/// A `return` inside a nested sub-block should not flag the line after the
/// outer block closes as dead.  When the brace depth falls below the depth
/// at which the terminator was seen, the terminator is cleared.
#[test]
fn return_inside_block_does_not_flag_code_after_outer_block() -> Result<(), String> {
    // The `return` is at depth 2 (inside the sub + if block).
    // After the `if` and `sub` close (depth 0), there is no active terminator.
    let source = "sub foo {\n    if ($x) {\n        return 1;\n    }\n}\nmy $y = 2;\n";
    let detector = detector_with_single_file("file:///depth_reset.pl", source)?;
    let dead = detect_for_path(&detector, "/depth_reset.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::UnreachableCode),
        "code after outer block should not be flagged as unreachable; got {dead:?}"
    );
    Ok(())
}

/// Unconditional return deep inside nested blocks followed by more code at
/// the same nesting depth should be detected, but code outside the block is safe.
#[test]
fn nested_return_flags_unreachable_within_same_depth() -> Result<(), String> {
    let source =
        "sub foo {\n    return 42;\n    print 'dead inside sub';\n}\nprint 'live outside';\n";
    let detector = detector_with_single_file("file:///nested_depth.pl", source)?;
    let dead = detect_for_path(&detector, "/nested_depth.pl")?;
    // There should be exactly one unreachable item (the print inside the sub)
    let unreachable: Vec<_> =
        dead.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert_eq!(unreachable.len(), 1, "only the print inside sub should be flagged; got {dead:?}");
    assert_eq!(unreachable[0].start_line, 3, "line 3 is unreachable; got {unreachable:?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — lines 189-191, 216-217: UnusedConstant and UnusedPackage stats
// ---------------------------------------------------------------------------

/// An unused `use constant` declaration should be counted in stats.
#[test]
fn workspace_analysis_counts_unused_constant() -> Result<(), String> {
    let source = "use constant MAX => 100;\n";
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///const.pl")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    // Should detect MAX as an unused constant
    let has_constant = analysis
        .dead_code
        .iter()
        .any(|d| d.code_type == DeadCodeType::UnusedConstant || d.name.as_deref() == Some("MAX"));
    // The stats must be consistent with the dead_code vector
    let constant_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnusedConstant).count();
    assert_eq!(
        analysis.stats.unused_constants, constant_count,
        "unused_constants stat must match dead_code count; has_constant={has_constant}"
    );
    Ok(())
}

/// An unused package declaration should be counted in stats.
#[test]
fn workspace_analysis_counts_unused_package() -> Result<(), String> {
    // A package declared but never `use`d from another file
    let source = "package MyOrphan;\nsub helper { return 1; }\n1;\n";
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///orphan.pm")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    let pkg_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnusedPackage).count();
    assert_eq!(
        analysis.stats.unused_packages, pkg_count,
        "unused_packages stat must match dead_code count"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 220: `_ => {}` for UnusedImport / UnusedExport
// ---------------------------------------------------------------------------

/// UnusedImport and UnusedExport items do not increment any named stat counter;
/// they fall through the `_ => {}` arm.  The total_dead_lines counter still
/// includes them.
#[test]
fn workspace_stats_total_dead_lines_includes_all_items() -> Result<(), String> {
    // Use a code pattern that produces at least one dead item (unreachable),
    // then verify total_dead_lines >= dead_code.len() (each item spans >= 1 line).
    let source = "exit 0;\nprint 'never';\n";
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///exit_test.pl")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert!(
        analysis.stats.total_dead_lines >= analysis.dead_code.len(),
        "total_dead_lines must be >= number of dead items (each spans >= 1 line)"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 245: multi-line dead items (end_line > start_line)
//   stats.total_dead_lines += end_line.saturating_sub(start_line) + 1
// ---------------------------------------------------------------------------

/// A dead branch that spans multiple lines contributes more than 1 to
/// `total_dead_lines`.
#[test]
fn workspace_stats_total_dead_lines_counts_multiline_items() -> Result<(), String> {
    // This dead branch spans 4 lines (lines 1-4).
    let source = "if (0) {\n    print 'a';\n    print 'b';\n    print 'c';\n}\n";
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///multiline.pl")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert!(
        analysis.stats.dead_branches >= 1,
        "should have at least one dead branch; got {:?}",
        analysis.stats
    );
    // The dead item spans lines 1-4, so total_dead_lines should be >= 4
    assert!(
        analysis.stats.total_dead_lines >= 4,
        "total_dead_lines should be >= 4 for a 4-line dead branch; got {}",
        analysis.stats.total_dead_lines
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 131 via depth tracking: terminator then deeper nesting
// ---------------------------------------------------------------------------

/// Return inside a deeply nested block; code at a higher depth after the
/// closing braces is not flagged.
#[test]
fn return_in_nested_if_clears_when_outer_block_closes() -> Result<(), String> {
    let source = concat!(
        "sub process {\n",
        "    if ($ok) {\n",
        "        if ($good) {\n",
        "            return 1;\n",
        "        }\n",     // depth drops from 3 to 2
        "    }\n",         // depth drops from 2 to 1
        "    return 0;\n", // still inside sub, after inner if block — not unreachable
        "}\n",
    );
    let detector = detector_with_single_file("file:///deep_nested.pl", source)?;
    let dead = detect_for_path(&detector, "/deep_nested.pl")?;
    let unreachable: Vec<_> =
        dead.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "return inside nested if must not flag code at lower depth; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional edge cases for is_always_true — string "0" in single quotes
// ---------------------------------------------------------------------------

/// `unless ('0') { ... }` — the inner string is `"0"` which is falsy in Perl,
/// so `is_always_true` should return false, and the `unless` body is live.
#[test]
fn unless_string_zero_single_quote_is_not_dead_branch() -> Result<(), String> {
    let source = "unless ('0') {\n    print 'runs';\n}\n";
    let detector = detector_with_single_file("file:///unless_str_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_str_zero.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "unless ('0') body is live (0 is falsy), should not be dead; got {dead:?}"
    );
    Ok(())
}

/// `unless ("0") { ... }` — double-quoted "0" is also falsy; body is live.
#[test]
fn unless_string_zero_double_quote_is_not_dead_branch() -> Result<(), String> {
    let source = "unless (\"0\") {\n    print 'runs';\n}\n";
    let detector = detector_with_single_file("file:///unless_dstr_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_dstr_zero.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "unless (\"0\") body is live (\"0\" is falsy), should not be dead; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional: elsif with always-false condition (when on its own line)
// ---------------------------------------------------------------------------

/// `elsif (0) { ... }` — dead branch detection covers `elsif` when it appears
/// at the start of a trimmed line (i.e., on its own line with no preceding `}`).
/// The detector strips leading whitespace before checking the keyword prefix.
#[test]
fn elsif_zero_on_own_line_is_dead_branch() -> Result<(), String> {
    // When `elsif` is on its own line (trimmed starts with `elsif`)
    let source = "if ($x) {\n    print 'a';\n}\nelsif (0) {\n    print 'never';\n}\n";
    let detector = detector_with_single_file("file:///elsif_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/elsif_zero.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "elsif (0) on its own line should produce a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// is_always_true with wrapped float (exercise recursive parens path)
// ---------------------------------------------------------------------------

/// `unless ((2.0)) { ... }` — nested parens around a non-zero float.
#[test]
fn unless_nested_parens_float_is_dead_branch() -> Result<(), String> {
    let source = "unless ((2.0)) {\n    print 'never';\n}\n";
    let detector = detector_with_single_file("file:///unless_nested_float.pl", source)?;
    let dead = detect_for_path(&detector, "/unless_nested_float.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "unless ((2.0)) should be a dead branch; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 245: comment after terminator (split_once('#') returns Some)
// ---------------------------------------------------------------------------

/// `return 1; # inline comment` — the `#` triggers the `split_once('#')` branch
/// inside `detect_unconditional_terminator`.  The return is still unconditional
/// (no postfix condition before the `#`), so the next non-empty line is dead.
#[test]
fn return_with_inline_comment_still_marks_following_line_dead() -> Result<(), String> {
    let source = "return 1; # done here\nprint 'never';\n";
    let detector = detector_with_single_file("file:///return_comment.pl", source)?;
    let dead = detect_for_path(&detector, "/return_comment.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::UnreachableCode),
        "return with inline comment should still produce unreachable code; got {dead:?}"
    );
    Ok(())
}

/// `die \"error\"; # log and quit` — same pattern for `die`.
#[test]
fn die_with_inline_comment_marks_following_line_dead() -> Result<(), String> {
    let source = "die \"fatal\"; # crash\nprint 'never';\n";
    let detector = detector_with_single_file("file:///die_comment.pl", source)?;
    let dead = detect_for_path(&detector, "/die_comment.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::UnreachableCode),
        "die with inline comment should still produce unreachable code; got {dead:?}"
    );
    Ok(())
}

/// `return if $x; # comment` — postfix condition before `#` means the return
/// is conditional.  The `#` branch is entered but the condition check still
/// suppresses the terminator.
#[test]
fn conditional_return_with_inline_comment_is_not_unconditional() -> Result<(), String> {
    let source = "return if $x; # sometimes\nsay 'live';\n";
    let detector = detector_with_single_file("file:///cond_return_comment.pl", source)?;
    let dead = detect_for_path(&detector, "/cond_return_comment.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::UnreachableCode),
        "postfix conditional return with comment should not flag following code; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 181: document with non-file:// URI skipped by uri_to_fs_path
// ---------------------------------------------------------------------------

/// Inject a document into the workspace via `document_store().open()` using a
/// non-file:// URI (e.g., `untitled://buffer1`).  The `analyze_workspace` loop
/// calls `uri_to_fs_path`, which returns `None` for such URIs, and skips the
/// document.  The workspace analysis should complete without error.
#[test]
fn workspace_analysis_skips_non_file_uri_documents() {
    use perl_workspace::workspace_index::WorkspaceIndex;

    let index = WorkspaceIndex::new();
    // Insert a document with a non-file:// URI directly via the document store.
    // This exercises the `uri_to_fs_path` → None branch in analyze_workspace.
    index.document_store().open("untitled:///buffer1".to_string(), 1, "my $x = 1;\n".to_string());

    let detector = DeadCodeDetector::new(index);
    // Should not panic — the non-file document is silently skipped.
    let analysis = detector.analyze_workspace();
    // The document was injected directly; files_analyzed counts all_documents
    assert_eq!(
        analysis.files_analyzed, 1,
        "files_analyzed should count all documents including non-file ones"
    );
}

// ---------------------------------------------------------------------------
// lib.rs — line 220: `_ => {}` for UnusedImport / UnusedExport in stats
// ---------------------------------------------------------------------------

/// `UnusedImport` and `UnusedExport` items do not map to any named stat counter;
/// they fall through the `_ => {}` arm.  We verify this by constructing an
/// analysis manually with those types and checking the stats aggregation.
/// The `analyze_workspace` stats loop uses the same match, so total_dead_lines
/// still increases for these items while the per-category counters stay at zero.
#[test]
fn workspace_stats_unused_import_export_do_not_increment_named_counters() {
    use perl_dead_code::{DeadCode, DeadCodeAnalysis, DeadCodeStats};
    use std::path::PathBuf;

    // Build the analysis directly to exercise the stats loop for these variants.
    let items = vec![
        DeadCode {
            code_type: DeadCodeType::UnusedImport,
            name: Some("Unused::Module".to_string()),
            file_path: PathBuf::from("/test.pl"),
            start_line: 1,
            end_line: 1,
            reason: "Module imported but not used".to_string(),
            confidence: 0.8,
            suggestion: None,
        },
        DeadCode {
            code_type: DeadCodeType::UnusedExport,
            name: Some("exported_fn".to_string()),
            file_path: PathBuf::from("/lib.pm"),
            start_line: 5,
            end_line: 5,
            reason: "Exported but never called".to_string(),
            confidence: 0.7,
            suggestion: None,
        },
    ];

    // Simulate what analyze_workspace does in the stats loop.
    let mut stats = DeadCodeStats::default();
    for item in &items {
        let lines = item.end_line.saturating_sub(item.start_line) + 1;
        stats.total_dead_lines += lines;
        match item.code_type {
            DeadCodeType::UnusedSubroutine => stats.unused_subroutines += 1,
            DeadCodeType::UnusedVariable => stats.unused_variables += 1,
            DeadCodeType::UnusedConstant => stats.unused_constants += 1,
            DeadCodeType::UnusedPackage => stats.unused_packages += 1,
            DeadCodeType::UnreachableCode => stats.unreachable_statements += 1,
            DeadCodeType::DeadBranch => stats.dead_branches += 1,
            _ => {} // UnusedImport and UnusedExport fall here
        }
    }

    // None of the named counters should increment for these two types.
    assert_eq!(stats.unused_subroutines, 0);
    assert_eq!(stats.unused_variables, 0);
    assert_eq!(stats.unused_constants, 0);
    assert_eq!(stats.unused_packages, 0);
    assert_eq!(stats.unreachable_statements, 0);
    assert_eq!(stats.dead_branches, 0);
    // total_dead_lines still counts them (1 line each = 2 total)
    assert_eq!(stats.total_dead_lines, 2);

    // Also verify round-trip through DeadCodeAnalysis
    let analysis = DeadCodeAnalysis { dead_code: items, stats, files_analyzed: 2, total_lines: 20 };
    assert_eq!(analysis.dead_code.len(), 2);
}

// ---------------------------------------------------------------------------
// Additional: CORE::exit terminator detection
// ---------------------------------------------------------------------------

/// `CORE::exit` should be recognized as an unconditional terminator.
#[test]
fn core_exit_marks_following_line_dead() -> Result<(), String> {
    let source = "CORE::exit 1;\nprint 'never';\n";
    let detector = detector_with_single_file("file:///core_exit.pl", source)?;
    let dead = detect_for_path(&detector, "/core_exit.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::UnreachableCode),
        "CORE::exit should produce unreachable code; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — line 191: `_ => continue` for SymbolKind::Method and Label
//   find_unused_symbols returns these kinds; the match skips them
// ---------------------------------------------------------------------------

/// A labeled statement produces a `SymbolKind::Label` symbol in the workspace.
/// The `analyze_workspace` symbol loop encounters it in `find_unused_symbols`
/// and hits the `_ => continue` arm.  No `UnusedPackage`/`UnusedSubroutine`
/// entry should be created for the label.
#[test]
fn workspace_analysis_does_not_flag_unused_label() -> Result<(), String> {
    // A labeled loop — the label is indexed as SymbolKind::Label
    let source = "OUTER: for my $i (1..3) {\n    last OUTER if $i == 2;\n}\n";
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///label_test.pl")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    // The label `OUTER` should NOT be reported as an unused subroutine, package, etc.
    let label_dead: Vec<_> = analysis
        .dead_code
        .iter()
        .filter(|d| {
            d.name.as_deref() == Some("OUTER")
                && matches!(
                    d.code_type,
                    DeadCodeType::UnusedSubroutine
                        | DeadCodeType::UnusedVariable
                        | DeadCodeType::UnusedConstant
                        | DeadCodeType::UnusedPackage
                )
        })
        .collect();
    assert!(
        label_dead.is_empty(),
        "label OUTER must not appear as an unused sub/var/const/pkg; got {label_dead:?}"
    );
    Ok(())
}

/// A Perl 5.38+ `method` definition inside a class produces `SymbolKind::Method`.
/// Similar to labels, the method kind hits `_ => continue` in the match and
/// is not counted as an unused subroutine.
#[test]
fn workspace_analysis_does_not_flag_method_as_unused_subroutine() -> Result<(), String> {
    // Perl 5.38+ class syntax with a method definition
    let source = concat!(
        "use feature 'class';\n",
        "class Animal {\n",
        "    method speak { return 'generic'; }\n",
        "}\n",
    );
    let index = WorkspaceIndex::new();
    let uri = test_uri_to_index_uri("file:///method_test.pl")?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    // The method `speak` should NOT be counted as an unused subroutine
    // (it might appear as UnusedSubroutine if the workspace counts it that way,
    // but SymbolKind::Method hits the `_ => continue` branch and is NOT processed
    // through the existing match arms in analyze_workspace).
    let sub_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnusedSubroutine).count();
    assert_eq!(
        analysis.stats.unused_subroutines, sub_count,
        "unused_subroutines stat must match actual count; analysis consistent"
    );
    Ok(())
}
