//! BDD-style scenario coverage for perl-dead-code.
//!
//! These tests intentionally use a Given/When/Then structure to document
//! behavior from a user perspective.

use perl_dead_code::{DeadCodeAnalysis, DeadCodeDetector, DeadCodeType, generate_report};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

struct Scenario {
    detector: DeadCodeDetector,
}

impl Scenario {
    fn given_workspace(files: &[(&str, &str)]) -> Result<Self, String> {
        let index = WorkspaceIndex::new();
        for (uri, code) in files {
            index.index_file_str(uri, code)?;
        }

        Ok(Self { detector: DeadCodeDetector::new(index) })
    }

    fn when_analyzing_file(&self, path: &str) -> Result<Vec<perl_dead_code::DeadCode>, String> {
        self.detector.analyze_file(&PathBuf::from(path))
    }

    fn when_analyzing_workspace(&self) -> DeadCodeAnalysis {
        self.detector.analyze_workspace()
    }
}

#[test]
fn scenario_given_return_then_next_statement_is_reported_unreachable() -> Result<(), String> {
    // Given
    let scenario = Scenario::given_workspace(&[(
        "file:///bdd_unreachable.pl",
        "sub run {\n    return 1;\n    print 'never';\n}\n",
    )])?;

    // When
    let results = scenario.when_analyzing_file("/bdd_unreachable.pl")?;

    // Then
    let unreachable: Vec<_> =
        results.iter().filter(|item| item.code_type == DeadCodeType::UnreachableCode).collect();
    assert_eq!(unreachable.len(), 1);
    assert_eq!(unreachable[0].start_line, 3);
    Ok(())
}

#[test]
fn scenario_given_if_zero_then_branch_is_reported_dead() -> Result<(), String> {
    // Given
    let scenario = Scenario::given_workspace(&[(
        "file:///bdd_dead_branch.pl",
        "if (0) {\n    print 'never';\n}\n",
    )])?;

    // When
    let results = scenario.when_analyzing_file("/bdd_dead_branch.pl")?;

    // Then
    let dead_branch: Vec<_> =
        results.iter().filter(|item| item.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(dead_branch.len(), 1);
    assert!(dead_branch[0].reason.contains("always false"));
    Ok(())
}

#[test]
fn scenario_given_unused_subroutine_then_workspace_flags_it() -> Result<(), String> {
    // Given
    let scenario = Scenario::given_workspace(&[(
        "file:///bdd_unused_sub.pl",
        "sub helper_unused { return 1; }\n",
    )])?;

    // When
    let analysis = scenario.when_analyzing_workspace();

    // Then
    let flagged: Vec<_> = analysis
        .dead_code
        .iter()
        .filter(|item| {
            item.code_type == DeadCodeType::UnusedSubroutine
                && item.name.as_deref() == Some("helper_unused")
        })
        .collect();
    assert_eq!(flagged.len(), 1);
    Ok(())
}

#[test]
fn scenario_given_cross_file_usage_then_subroutine_is_not_flagged() -> Result<(), String> {
    // Given
    let scenario = Scenario::given_workspace(&[
        ("file:///bdd_lib.pm", "package BddLib;\nsub shared { return 1; }\n1;\n"),
        ("file:///bdd_main.pl", "use BddLib;\nBddLib::shared();\n"),
    ])?;

    // When
    let analysis = scenario.when_analyzing_workspace();

    // Then
    let flagged_shared =
        analysis.dead_code.iter().any(|item| item.name.as_deref() == Some("shared"));
    assert!(!flagged_shared);
    Ok(())
}

#[test]
fn scenario_given_dead_items_then_report_summarizes_counts() -> Result<(), String> {
    // Given
    let scenario = Scenario::given_workspace(&[(
        "file:///bdd_report.pl",
        "if (0) {\n    print 'never';\n}\n",
    )])?;

    // When
    let analysis = scenario.when_analyzing_workspace();
    let report = generate_report(&analysis);

    // Then
    assert!(report.contains("Dead Code Analysis Report"));
    assert!(report.contains("Files analyzed: 1"));
    assert!(report.contains("Dead branches:"));
    Ok(())
}
