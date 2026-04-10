//! Behavior-driven scenarios for perl-dead-code.
//!
//! The intent of this suite is to keep the crate's externally-visible behavior
//! easy to read as Given/When/Then narratives.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

fn given_workspace_with_file(uri: &str, code: &str) -> Result<WorkspaceIndex, String> {
    let index = WorkspaceIndex::new();
    index.index_file_str(uri, code)?;
    Ok(index)
}

fn given_workspace_with_files(files: &[(&str, &str)]) -> Result<WorkspaceIndex, String> {
    let index = WorkspaceIndex::new();
    for (uri, code) in files {
        index.index_file_str(uri, code)?;
    }
    Ok(index)
}

#[test]
fn given_if_zero_branch_when_analyzing_file_then_branch_is_marked_dead() -> Result<(), String> {
    // Given
    let code = "if (0) {\n    print \"never\";\n}\nprint \"live\";\n";
    let index = given_workspace_with_file("file:///bdd_if_zero.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    // When
    let findings = detector.analyze_file(&PathBuf::from("/bdd_if_zero.pl"))?;

    // Then
    let dead_branch = findings
        .iter()
        .find(|finding| finding.code_type == DeadCodeType::DeadBranch)
        .ok_or_else(|| "expected a dead branch finding".to_string())?;

    assert_eq!(dead_branch.start_line, 1);
    assert_eq!(dead_branch.end_line, 3);
    assert!(dead_branch.reason.contains("always false"));
    Ok(())
}

#[test]
fn given_unless_true_branch_when_analyzing_file_then_branch_is_marked_dead() -> Result<(), String> {
    // Given
    let code = "unless (1) {\n    print \"never\";\n}\n";
    let index = given_workspace_with_file("file:///bdd_unless_one.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    // When
    let findings = detector.analyze_file(&PathBuf::from("/bdd_unless_one.pl"))?;

    // Then
    let dead_branch = findings
        .iter()
        .find(|finding| finding.code_type == DeadCodeType::DeadBranch)
        .ok_or_else(|| "expected a dead branch finding".to_string())?;

    assert_eq!(dead_branch.start_line, 1);
    assert_eq!(dead_branch.end_line, 3);
    assert!(dead_branch.reason.contains("always true"));
    Ok(())
}

#[test]
fn given_parenthesized_false_condition_when_analyzing_file_then_dead_branch_is_detected()
-> Result<(), String> {
    // Given
    let code = "while ((0)) {\n    print \"never\";\n}\n";
    let index = given_workspace_with_file("file:///bdd_nested_zero.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    // When
    let findings = detector.analyze_file(&PathBuf::from("/bdd_nested_zero.pl"))?;

    // Then
    let dead_branch_count =
        findings.iter().filter(|finding| finding.code_type == DeadCodeType::DeadBranch).count();
    assert_eq!(dead_branch_count, 1);
    Ok(())
}

#[test]
fn given_runtime_condition_when_analyzing_file_then_no_dead_branch_is_reported()
-> Result<(), String> {
    // Given
    let code = "my $feature_flag = 0;\nif ($feature_flag) {\n    print \"reachable\";\n}\n";
    let index = given_workspace_with_file("file:///bdd_runtime_cond.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    // When
    let findings = detector.analyze_file(&PathBuf::from("/bdd_runtime_cond.pl"))?;

    // Then
    assert!(
        findings.iter().all(|finding| finding.code_type != DeadCodeType::DeadBranch),
        "runtime conditions should not be treated as dead branches"
    );
    Ok(())
}

#[test]
fn given_workspace_with_dead_branch_and_unreachable_when_analyzing_workspace_then_stats_reflect_both()
-> Result<(), String> {
    // Given
    let files = [
        ("file:///bdd_branch.pl", "if (0) {\n    print \"never\";\n}\nprint \"live\";\n"),
        ("file:///bdd_unreachable.pl", "die \"boom\";\nprint \"never\";\n"),
    ];
    let index = given_workspace_with_files(&files)?;
    let detector = DeadCodeDetector::new(index);

    // When
    let analysis = detector.analyze_workspace();

    // Then
    assert_eq!(analysis.files_analyzed, 2);
    assert_eq!(analysis.stats.dead_branches, 1);
    assert_eq!(analysis.stats.unreachable_statements, 1);
    assert!(analysis.stats.total_dead_lines >= 2);
    Ok(())
}
