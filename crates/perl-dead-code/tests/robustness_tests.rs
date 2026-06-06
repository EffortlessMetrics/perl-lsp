//! Robustness tests for `perl-dead-code` (#791, #795).
//!
//! - #791: bounds-safe slicing — empty / short conditions must not panic.
//! - #795: recursion depth guard — deeply-nested conditions must not stack-overflow.
//! - Regression: normal dead-branch detection continues to work after the fixes.
//!
//! Note on #795 deep-nesting tests: the public API flows through `WorkspaceIndex`,
//! which rejects files that exceed the parser's recursion limit before the text ever
//! reaches the dead-branch scanner.  The critical depth-guard logic in
//! `is_always_false` / `is_always_true` is therefore exercised via unit tests inside
//! `dead_branches.rs` (see `#[cfg(test)]` there).  These integration tests verify
//! the integration path and confirm graceful handling of degenerate inputs.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers (mirrors pattern from dead_code_behavior_tests.rs)
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
// Bug #791 — bounds-safe slicing: empty / degenerate conditions must not panic
//
// These tests confirm that the detector handles empty and degenerate conditions
// gracefully (no panic, no spurious dead-branch report).
// ---------------------------------------------------------------------------

/// `if () { 1 }` — empty condition parens.
#[test]
fn test_empty_condition_if_does_not_panic() -> Result<(), String> {
    let source = "if () { 1 }\n";
    let detector = detector_with_single_file("file:///empty_cond.pl", source)?;
    let dead = detect_for_path(&detector, "/empty_cond.pl")?;
    // empty condition is not a recognised always-false literal → no dead branch
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "empty condition must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

/// `unless () { 1 }` — same empty-condition path for the inverted keyword.
#[test]
fn test_empty_condition_unless_does_not_panic() -> Result<(), String> {
    let source = "unless () { 1 }\n";
    let detector = detector_with_single_file("file:///empty_cond_unless.pl", source)?;
    let dead = detect_for_path(&detector, "/empty_cond_unless.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "empty condition must not produce a dead branch; got {dead:?}"
    );
    Ok(())
}

/// `if (1) { 1 } elsif () { 2 }` — empty condition on elsif.
#[test]
fn test_empty_condition_elsif_does_not_panic() -> Result<(), String> {
    let source = "if (1) { 1 } elsif () { 2 }\n";
    let detector = detector_with_single_file("file:///empty_cond_elsif.pl", source)?;
    // Must not panic; result doesn't need to contain a dead branch
    let _dead = detect_for_path(&detector, "/empty_cond_elsif.pl")?;
    Ok(())
}

/// `while () { 1 }` — empty condition on while.
#[test]
fn test_empty_condition_while_does_not_panic() -> Result<(), String> {
    let source = "while () { 1 }\n";
    let detector = detector_with_single_file("file:///empty_cond_while.pl", source)?;
    let _dead = detect_for_path(&detector, "/empty_cond_while.pl")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Bug #795 — recursion depth guard via integration path
//
// We cannot trigger stack overflow via the public API because deeply-nested
// conditions (>128 parens) are rejected by the workspace parser before
// the dead-branch scanner runs.  Integration tests here use a depth just
// within the parser limit (≤ 64) to confirm correct end-to-end handling.
//
// The actual depth guard in `is_always_false` / `is_always_true` is tested
// by the unit tests inside `dead_branches.rs` (#[cfg(test)]).
// ---------------------------------------------------------------------------

/// 64 layers of nested parens around `0` — within parser limit.
/// Verifies that the dead-branch scanner correctly handles moderately deep
/// nesting (which is legal Perl) and still detects the always-false condition.
#[test]
fn test_moderately_nested_false_condition_detected() -> Result<(), String> {
    let depth = 64usize;
    let open_parens: String = "(".repeat(depth);
    let close_parens: String = ")".repeat(depth);
    let source = format!("if ({open_parens}0{close_parens}) {{ die; }}\n");
    let detector = detector_with_single_file("file:///moderate_deep_false.pl", &source)?;
    let dead = detect_for_path(&detector, "/moderate_deep_false.pl")?;
    // Within the depth limit: should still detect the always-false condition.
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "moderately-nested if (0) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

/// 64 layers of nested parens around `1` inside `unless` — within parser limit.
#[test]
fn test_moderately_nested_true_condition_detected() -> Result<(), String> {
    let depth = 64usize;
    let open_parens: String = "(".repeat(depth);
    let close_parens: String = ")".repeat(depth);
    let source = format!("unless ({open_parens}1{close_parens}) {{ die; }}\n");
    let detector = detector_with_single_file("file:///moderate_deep_true.pl", &source)?;
    let dead = detect_for_path(&detector, "/moderate_deep_true.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "moderately-nested unless (1) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Regression: normal dead-branch detection still works after the fixes
// ---------------------------------------------------------------------------

#[test]
fn test_regression_if_zero_still_detected() -> Result<(), String> {
    let source = "if (0) { print 'dead'; }\n";
    let detector = detector_with_single_file("file:///reg_if_zero.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_if_zero.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "if (0) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_if_one_not_dead() -> Result<(), String> {
    let source = "if (1) { print 'live'; }\n";
    let detector = detector_with_single_file("file:///reg_if_one.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_if_one.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "if (1) must not be dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_unless_one_still_detected() -> Result<(), String> {
    let source = "unless (1) { print 'dead'; }\n";
    let detector = detector_with_single_file("file:///reg_unless_one.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_unless_one.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "unless (1) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_while_undef_still_detected() -> Result<(), String> {
    let source = "while (undef) { print 'dead'; }\n";
    let detector = detector_with_single_file("file:///reg_while_undef.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_while_undef.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "while (undef) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_shallow_nested_false_still_detected() -> Result<(), String> {
    // (0) — one level of extra parens, well within any depth limit
    let source = "if ((0)) { print 'dead'; }\n";
    let detector = detector_with_single_file("file:///reg_nested_false.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_nested_false.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "if ((0)) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_shallow_nested_true_still_detected() -> Result<(), String> {
    // (1) inside unless
    let source = "unless ((1)) { print 'dead'; }\n";
    let detector = detector_with_single_file("file:///reg_nested_true.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_nested_true.pl")?;
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "unless ((1)) must still be detected as dead; got {dead:?}"
    );
    Ok(())
}

#[test]
fn test_regression_variable_condition_not_dead() -> Result<(), String> {
    let source = "if ($x) { print 'maybe'; }\n";
    let detector = detector_with_single_file("file:///reg_var_cond.pl", source)?;
    let dead = detect_for_path(&detector, "/reg_var_cond.pl")?;
    assert!(
        dead.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "variable condition must never be classified as dead; got {dead:?}"
    );
    Ok(())
}
