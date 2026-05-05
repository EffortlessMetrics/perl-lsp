use perl_parser::dead_code_detector::{DeadCodeDetector, DeadCodeType};
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn detects_dead_code() -> TestResult {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///main.pl", "use A;\nA::bar();\n")?;
    index.index_file_str("file:///A.pm", "package A;\nsub foo { return 1; }\nsub bar { 1; }\n")?;
    index.index_file_str(
        "file:///Unused.pm",
        "package Unused;\nsub unused { return 1; }\nreturn 1;\nprint 'hi';\n",
    )?;

    let mut detector = DeadCodeDetector::new(index);
    detector.add_entry_point(PathBuf::from("/main.pl"));
    let analysis = detector.analyze_workspace();

    assert!(
        analysis
            .dead_code
            .iter()
            .any(|d| d.code_type == DeadCodeType::UnusedSubroutine
                && d.name.as_deref() == Some("foo"))
    );
    assert!(
        analysis
            .dead_code
            .iter()
            .any(|d| d.code_type == DeadCodeType::UnusedPackage
                && d.name.as_deref() == Some("Unused"))
    );
    assert!(analysis.dead_code.iter().any(
        |d| d.code_type == DeadCodeType::UnreachableCode && d.file_path.ends_with("Unused.pm")
    ));
    Ok(())
}

#[test]
fn return_at_end_of_sub_does_not_flag_closing_brace() -> TestResult {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///module.pm", "sub foo {\n    return 42;\n}\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/module.pm"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::UnreachableCode),
        "closing brace should not be flagged as unreachable"
    );
    Ok(())
}

#[test]
fn postfix_conditional_return_is_not_unconditional_terminator() -> TestResult {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///script.pl", "return if $cond;\nsay 'live';\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/script.pl"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::UnreachableCode),
        "postfix conditional return should not produce unreachable diagnostics"
    );
    Ok(())
}
