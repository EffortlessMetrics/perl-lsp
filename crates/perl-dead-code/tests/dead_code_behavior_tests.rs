//! Behavior-driven tests for `perl-dead-code`.
//!
//! These tests focus on user-visible outcomes with a Given/When/Then structure.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::Path;

fn detector_with_single_file(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    index.index_file_str(uri, source)?;
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
    index.index_file_str("file:///scenario_a.pl", "exit 1;\nprint 'never';\n")?;
    index.index_file_str("file:///scenario_b.pl", "if (0) {\n    print 'dead';\n}\n")?;
    let detector = DeadCodeDetector::new(index);

    // When workspace analysis runs
    let analysis = detector.analyze_workspace();

    // Then both behavior classes are represented in the result summary
    assert!(analysis.stats.unreachable_statements >= 1);
    assert!(analysis.stats.dead_branches >= 1);
    assert!(
        analysis
            .dead_code
            .iter()
            .any(|item| item.code_type == DeadCodeType::UnreachableCode)
    );
    assert!(
        analysis
            .dead_code
            .iter()
            .any(|item| item.code_type == DeadCodeType::DeadBranch)
    );
    Ok(())
}
