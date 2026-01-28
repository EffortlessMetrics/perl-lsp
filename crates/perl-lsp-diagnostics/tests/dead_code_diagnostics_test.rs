use perl_lsp_diagnostics::{DiagnosticSeverity, DiagnosticTag, detect_dead_code};
use perl_parser_core::position::LineStartsCache;
use perl_workspace_index::workspace_index::WorkspaceIndex;

#[test]
fn test_dead_code_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Create workspace index
    let index = WorkspaceIndex::new();

    // Index main file that uses some functions
    index.index_file_str("file:///main.pl", "use A;\nA::bar();\n")?;

    // Index module A with used and unused functions
    index.index_file_str("file:///A.pm", "package A;\nsub foo { return 1; }\nsub bar { 1; }\n")?;

    // Get line index for A.pm
    let source = "package A;\nsub foo { return 1; }\nsub bar { 1; }\n";
    let line_index = LineStartsCache::new(source);

    // Detect dead code in A.pm
    let diagnostics = detect_dead_code(&index, "file:///A.pm", source, &line_index);

    // Should find unused subroutine 'foo'
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("foo") && d.severity == DiagnosticSeverity::Hint),
        "Should detect unused subroutine 'foo'"
    );

    // Verify it's tagged as unnecessary
    assert!(
        diagnostics.iter().any(|d| d.tags.contains(&DiagnosticTag::Unnecessary)),
        "Dead code should be tagged as unnecessary"
    );

    Ok(())
}

#[test]
fn test_dead_code_only_in_current_document() -> Result<(), Box<dyn std::error::Error>> {
    // Create workspace index
    let index = WorkspaceIndex::new();

    // Index main file
    index.index_file_str("file:///main.pl", "sub main_unused { 1; }\n")?;

    // Index module with unused function
    index.index_file_str("file:///A.pm", "package A;\nsub unused { 1; }\n")?;

    // Get line index for main.pl
    let main_source = "sub main_unused { 1; }\n";
    let line_index = LineStartsCache::new(main_source);

    // Detect dead code only in main.pl
    let diagnostics = detect_dead_code(&index, "file:///main.pl", main_source, &line_index);

    // Should only find dead code in main.pl, not in A.pm
    assert!(diagnostics.iter().any(|d| d.message.contains("main_unused")));
    assert!(!diagnostics.iter().any(|d| d.message.contains("unused") && d.message.contains("A::")));

    Ok(())
}
