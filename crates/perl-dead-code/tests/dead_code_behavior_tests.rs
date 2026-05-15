//! Behavior-driven tests for `perl-dead-code`.
//!
//! These tests focus on user-visible outcomes with a Given/When/Then structure.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

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

#[test]
fn scenario_unreachable_statement_after_return_is_reported() -> Result<(), String> {
    // Given a subroutine with a statement after an unconditional return
    let detector = detector_with_single_file(
        "file:///scenario_return.pl",
        "sub run {\n    return 1;\n    print 'never';\n}\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_return.pl")?;

    // Then the post-return statement is flagged as unreachable
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::UnreachableCode
            && item.start_line == 3
            && item.reason.contains("return")
    }));
    Ok(())
}

#[test]
fn scenario_if_zero_branch_is_marked_dead_branch() -> Result<(), String> {
    // Given an if block whose condition is always false
    let detector = detector_with_single_file(
        "file:///scenario_if_zero.pl",
        "if (0) {\n    print 'dead';\n}\nprint 'live';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_if_zero.pl")?;

    // Then the block is reported as a dead branch
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.start_line == 1
            && item.end_line == 3
            && item.reason.contains("always false")
    }));
    Ok(())
}

#[test]
fn scenario_unless_one_branch_is_marked_dead_branch() -> Result<(), String> {
    // Given an unless block whose condition is always true
    let detector = detector_with_single_file(
        "file:///scenario_unless_one.pl",
        "unless (1) {\n    print 'dead';\n}\nprint 'live';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_unless_one.pl")?;

    // Then the unless body is reported as dead
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always true")
            && item.reason.contains("never executed")
    }));
    Ok(())
}

#[test]
fn scenario_nested_parenthesized_false_condition_is_detected() -> Result<(), String> {
    // Given a branch using a nested always-false expression
    let detector = detector_with_single_file(
        "file:///scenario_nested_false.pl",
        "while (((0))) {\n    print 'dead loop';\n}\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_nested_false.pl")?;

    // Then the loop body is reported as dead
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always false")
            && item.start_line == 1
    }));
    Ok(())
}

#[test]
fn scenario_workspace_analysis_aggregates_unreachable_and_dead_branch() -> Result<(), String> {
    // Given a workspace with both unreachable code and a dead branch
    let index = WorkspaceIndex::new();
    let scenario_a_uri = test_uri_to_index_uri("file:///scenario_a.pl")?;
    let scenario_b_uri = test_uri_to_index_uri("file:///scenario_b.pl")?;
    index.index_file_str(&scenario_a_uri, "exit 1;\nprint 'never';\n")?;
    index.index_file_str(&scenario_b_uri, "if (0) {\n    print 'dead';\n}\n")?;
    let detector = DeadCodeDetector::new(index);

    // When workspace analysis runs
    let analysis = detector.analyze_workspace();

    // Then both behavior classes are represented in the result summary
    assert!(analysis.stats.unreachable_statements >= 1);
    assert!(analysis.stats.dead_branches >= 1);
    assert!(analysis.dead_code.iter().any(|item| item.code_type == DeadCodeType::UnreachableCode));
    assert!(analysis.dead_code.iter().any(|item| item.code_type == DeadCodeType::DeadBranch));
    Ok(())
}

// ---------------------------------------------------------------------------
// for / foreach — list iterator false-positive regression suite (#9009)
//
// `for` and `foreach` are list iterators, not boolean condition checkers.
// `for (0)` runs the body once with $_ = 0.  It is NEVER dead code, even
// though `0` is falsy in boolean context.  The dead-branch detector must
// not conflate list-iterator semantics with boolean condition semantics.
// ---------------------------------------------------------------------------

#[test]
fn scenario_for_loop_with_constant_zero_is_not_dead_branch() -> Result<(), String> {
    // Given: `for (0)` iterates once with $_ = 0 — this is live code
    let detector = detector_with_single_file(
        "file:///for_zero.pl",
        "for (0) {\n    say 'runs once, $_ is 0';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/for_zero.pl")?;

    // Then no dead-branch diagnostic is emitted
    assert!(
        !dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "for (0) iterates once — must not be reported as a dead branch"
    );
    Ok(())
}

#[test]
fn scenario_foreach_loop_with_constant_zero_is_not_dead_branch() -> Result<(), String> {
    // Given: `foreach (0)` iterates once with $_ = 0 — this is live code
    let detector = detector_with_single_file(
        "file:///foreach_zero.pl",
        "foreach (0) {\n    say 'runs once, $_ is 0';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/foreach_zero.pl")?;

    // Then no dead-branch diagnostic is emitted
    assert!(
        !dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "foreach (0) iterates once — must not be reported as a dead branch"
    );
    Ok(())
}

#[test]
fn scenario_for_loop_with_empty_string_is_not_dead_branch() -> Result<(), String> {
    // Given: `for ("")` iterates once with $_ = "" — still live code
    let detector = detector_with_single_file(
        "file:///for_empty_str.pl",
        "for (\"\") {\n    say \"runs once, \\$_ is empty string\";\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/for_empty_str.pl")?;

    // Then no dead-branch diagnostic is emitted
    assert!(
        !dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "for (\"\") iterates once — must not be reported as a dead branch"
    );
    Ok(())
}

#[test]
fn scenario_for_loop_with_undef_is_not_dead_branch() -> Result<(), String> {
    // Given: `for (undef)` iterates once with $_ = undef — still live code
    let detector = detector_with_single_file(
        "file:///for_undef.pl",
        "for (undef) {\n    say 'runs once, $_ is undef';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/for_undef.pl")?;

    // Then no dead-branch diagnostic is emitted
    assert!(
        !dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "for (undef) iterates once with $_ = undef — must not be a dead branch"
    );
    Ok(())
}

#[test]
fn scenario_for_loop_with_multi_element_list_is_not_dead_branch() -> Result<(), String> {
    // Given: `for (1, 2, 3)` iterates three times — definitely live
    let detector =
        detector_with_single_file("file:///for_list.pl", "for (1, 2, 3) {\n    say $_ ;\n}\n")?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/for_list.pl")?;

    // Then no dead-branch diagnostic is emitted
    assert!(
        !dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "for (1, 2, 3) iterates — must not be a dead branch"
    );
    Ok(())
}

#[test]
fn scenario_for_loop_mixed_with_dead_if_in_same_file() -> Result<(), String> {
    // Given: a file with a live `for (0)` loop AND a genuinely dead `if (0)` branch
    let detector = detector_with_single_file(
        "file:///mixed.pl",
        "for (0) {\n    say 'live';\n}\nif (0) {\n    say 'dead';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/mixed.pl")?;

    // Then exactly one dead-branch is reported (the `if (0)`, not the `for (0)`)
    let dead_branches: Vec<_> =
        dead.iter().filter(|item| item.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(
        dead_branches.len(),
        1,
        "only the `if (0)` block should be flagged; got {dead_branches:?}"
    );
    assert!(
        dead_branches[0].reason.contains("if"),
        "the dead branch should be the `if` block, not the `for` loop"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Regression guards — existing detections must still fire after the fix
// ---------------------------------------------------------------------------

#[test]
fn regression_while_zero_loop_is_still_dead() -> Result<(), String> {
    // Given: `while (0)` — loop body is never executed (0 is always false)
    let detector =
        detector_with_single_file("file:///while_zero.pl", "while (0) {\n    say 'never';\n}\n")?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/while_zero.pl")?;

    // Then the loop is flagged as a dead branch
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always false")),
        "while (0) must still be flagged as a dead branch"
    );
    Ok(())
}

#[test]
fn regression_if_zero_branch_is_still_dead() -> Result<(), String> {
    // Given: `if (0)` — body never executes
    let detector = detector_with_single_file(
        "file:///if_zero_regression.pl",
        "if (0) {\n    say 'never';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/if_zero_regression.pl")?;

    // Then the if-block is flagged
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "if (0) must still be flagged as a dead branch"
    );
    Ok(())
}

#[test]
fn regression_unless_one_branch_is_still_dead() -> Result<(), String> {
    // Given: `unless (1)` — body never executes
    let detector = detector_with_single_file(
        "file:///unless_one_regression.pl",
        "unless (1) {\n    say 'never';\n}\n",
    )?;

    // When dead-code analysis runs
    let dead = detect_for_path(&detector, "/unless_one_regression.pl")?;

    // Then the block is flagged as dead
    assert!(
        dead.iter().any(|item| item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always true")),
        "unless (1) must still be flagged as a dead branch"
    );
    Ok(())
}
