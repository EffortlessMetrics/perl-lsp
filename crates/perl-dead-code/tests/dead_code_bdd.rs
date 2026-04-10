use perl_dead_code::{
    DeadCodeAnalysis, DeadCodeDetector, DeadCodeStats, DeadCodeType, generate_report,
};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

fn index_with_file(uri: &str, code: &str) -> Result<WorkspaceIndex, String> {
    let index = WorkspaceIndex::new();
    index.index_file_str(uri, code)?;
    Ok(index)
}

#[test]
fn given_return_followed_by_statement_when_analyzing_file_then_unreachable_code_is_reported()
-> Result<(), String> {
    let code = "sub run {\n    return 1;\n    print \"dead\";\n}\n";
    let index = index_with_file("file:///bdd_return.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let findings = detector.analyze_file(&PathBuf::from("/bdd_return.pl"))?;

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code_type, DeadCodeType::UnreachableCode);
    assert_eq!(findings[0].start_line, 3);
    assert!(findings[0].reason.contains("return"));
    Ok(())
}

#[test]
fn given_if_zero_branch_when_analyzing_file_then_dead_branch_is_reported() -> Result<(), String> {
    let code = "if (0) {\n    print \"never\";\n}\nprint \"live\";\n";
    let index = index_with_file("file:///bdd_if_zero.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let findings = detector.analyze_file(&PathBuf::from("/bdd_if_zero.pl"))?;
    let dead_branch = findings
        .iter()
        .find(|item| item.code_type == DeadCodeType::DeadBranch)
        .ok_or_else(|| "expected a dead branch finding".to_string())?;

    assert_eq!(dead_branch.start_line, 1);
    assert_eq!(dead_branch.end_line, 3);
    assert!(dead_branch.reason.contains("always false"));
    Ok(())
}

#[test]
fn given_unless_true_branch_when_analyzing_file_then_dead_branch_is_reported() -> Result<(), String>
{
    let code = "unless (1) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///bdd_unless_true.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let findings = detector.analyze_file(&PathBuf::from("/bdd_unless_true.pl"))?;
    let dead_branch = findings
        .iter()
        .find(|item| item.code_type == DeadCodeType::DeadBranch)
        .ok_or_else(|| "expected a dead branch finding".to_string())?;

    assert!(dead_branch.reason.contains("always true"));
    Ok(())
}

#[test]
fn given_workspace_with_unused_subroutine_when_analyzing_workspace_then_unused_symbol_is_reported()
-> Result<(), String> {
    let code = "sub helper_unused { return 1; }\n";
    let index = index_with_file("file:///bdd_unused_sub.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();

    let unused_sub = analysis
        .dead_code
        .iter()
        .find(|item| {
            item.code_type == DeadCodeType::UnusedSubroutine
                && item.name.as_deref() == Some("helper_unused")
        })
        .ok_or_else(|| "expected an unused subroutine finding".to_string())?;

    assert_eq!(unused_sub.start_line, 1);
    assert_eq!(analysis.files_analyzed, 1);
    Ok(())
}

#[test]
fn given_analysis_summary_when_generating_report_then_report_contains_counts() {
    let analysis = DeadCodeAnalysis {
        dead_code: vec![],
        stats: DeadCodeStats {
            unused_subroutines: 2,
            unused_variables: 1,
            unused_constants: 0,
            unused_packages: 0,
            unreachable_statements: 3,
            dead_branches: 4,
            total_dead_lines: 10,
        },
        files_analyzed: 5,
        total_lines: 250,
    };

    let report = generate_report(&analysis);

    assert!(report.contains("Dead Code Analysis Report"));
    assert!(report.contains("Files analyzed: 5"));
    assert!(report.contains("Dead branches: 4"));
    assert!(report.contains("Total dead lines: 10"));
}
