use perl_parser::dead_code_detector::{DeadCodeDetector, DeadCodeType};
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

#[test]
fn detects_dead_code() {
    let index = WorkspaceIndex::new();
    index.index_file_str("file:///main.pl", "use A;\nA::bar();\n").unwrap();
    index
        .index_file_str("file:///A.pm", "package A;\nsub foo { return 1; }\nsub bar { 1; }\n")
        .unwrap();
    index
        .index_file_str(
            "file:///Unused.pm",
            "package Unused;\nsub unused { return 1; }\nreturn 1;\nprint 'hi';\n",
        )
        .unwrap();

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
}
