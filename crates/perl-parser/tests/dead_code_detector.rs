use perl_parser::dead_code_detector::{DeadCodeDetector, DeadCodeType};
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn index_file_str(index: &WorkspaceIndex, uri: &str, code: &str) -> Result<(), String> {
    let indexed_uri = match uri.strip_prefix("file://") {
        Some(path) => perl_uri::fs_path_to_uri(PathBuf::from(path)),
        None => Ok(uri.to_string()),
    }?;
    index.index_file_str(&indexed_uri, code)
}

#[test]
fn detects_dead_code() -> TestResult {
    let index = WorkspaceIndex::new();
    index_file_str(&index, "file:///main.pl", "use A;\nA::bar();\n")?;
    index_file_str(&index, "file:///A.pm", "package A;\nsub foo { return 1; }\nsub bar { 1; }\n")?;
    index_file_str(
        &index,
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
    index_file_str(&index, "file:///module.pm", "sub foo {\n    return 42;\n}\n")?;

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
    index_file_str(&index, "file:///script.pl", "return 42 if $cond;\nsay 'live';\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/script.pl"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::UnreachableCode),
        "postfix conditional return should not produce unreachable diagnostics"
    );
    Ok(())
}

#[test]
fn return_prefix_inside_identifier_is_not_a_terminator() -> TestResult {
    let index = WorkspaceIndex::new();
    index_file_str(&index, "file:///script.pl", "return42();\nsay 'live';\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/script.pl"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::UnreachableCode),
        "return42 should not match the return terminator"
    );
    Ok(())
}

#[test]
fn unconditional_return_still_flags_following_statement() -> TestResult {
    let index = WorkspaceIndex::new();
    index_file_str(&index, "file:///module.pm", "sub foo {\n    return 42;\n    say 'dead';\n}\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/module.pm"))?;

    assert!(
        dead.iter().any(|d| d.code_type == DeadCodeType::UnreachableCode && d.start_line == 3),
        "statement after unconditional return should remain unreachable"
    );
    Ok(())
}

#[test]
fn for_loop_with_constant_list_is_not_dead_branch() -> TestResult {
    let index = WorkspaceIndex::new();
    index_file_str(&index, "file:///script.pl", "for (0) {\n    say 'runs once';\n}\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/script.pl"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
        "for (0) iterates once with $_ = 0 — it is not dead code"
    );
    Ok(())
}

#[test]
fn foreach_loop_with_constant_list_is_not_dead_branch() -> TestResult {
    let index = WorkspaceIndex::new();
    index_file_str(&index, "file:///script.pl", "foreach (0) {\n    say 'runs once';\n}\n")?;

    let detector = DeadCodeDetector::new(index);
    let dead = detector.analyze_file(&PathBuf::from("/script.pl"))?;

    assert!(
        !dead.iter().any(|d| d.code_type == DeadCodeType::DeadBranch),
        "foreach (0) iterates once with $_ = 0 — it is not dead code"
    );
    Ok(())
}