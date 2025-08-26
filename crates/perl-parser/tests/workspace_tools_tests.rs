use perl_parser::{
    dead_code_detector::DeadCodeDetector,
    import_optimizer::ImportOptimizer,
    workspace_index::WorkspaceIndex,
    workspace_refactor::{TextEdit, WorkspaceRefactor},
};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fn apply_edits(text: String, edits: &[TextEdit]) -> String {
    // apply from end to start so offsets remain valid
    let mut result = text;
    for edit in edits.iter().rev() {
        result.replace_range(edit.start..edit.end, &edit.new_text);
    }
    result
}

#[test]
fn test_workspace_rename() {
    let index = WorkspaceIndex::new();
    let file1_uri = "file:///workspace/lib/Foo.pm";
    let file1_content = "package Foo;\nsub foo { 1 }\n1;";
    index.index_file(url::Url::parse(file1_uri).unwrap(), file1_content.to_string()).unwrap();
    let file2_uri = "file:///workspace/main.pl";
    let file2_content = "use Foo;\nFoo::foo();";
    index.index_file(url::Url::parse(file2_uri).unwrap(), file2_content.to_string()).unwrap();

    let refactor = WorkspaceRefactor::new(index);
    let result = refactor.rename_symbol("foo", "bar", Path::new("dummy"), (0, 0)).unwrap();
    assert_eq!(result.file_edits.len(), 2);
    for edit in &result.file_edits {
        let orig = if edit.file_path.to_string_lossy().ends_with("Foo.pm") {
            file1_content
        } else {
            file2_content
        };
        let new_text = apply_edits(orig.to_string(), &edit.edits);
        assert!(!new_text.contains("foo"));
        assert!(new_text.contains("bar"));
    }
}

#[test]
fn test_import_optimizer_analysis() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "use Foo;").unwrap();
    writeln!(file, "use Foo;").unwrap();
    writeln!(file, "use Bar;").unwrap();
    writeln!(file, "my $x = Foo::baz();").unwrap();
    writeln!(file, "Baz::qux();").unwrap();
    let path = file.into_temp_path();

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(path.as_ref()).unwrap();
    assert_eq!(analysis.duplicate_imports.len(), 1);
    assert_eq!(analysis.unused_imports.len(), 1);
    assert_eq!(analysis.missing_imports.len(), 1);
}

#[test]
fn test_dead_code_detection() {
    let index = WorkspaceIndex::new();
    let file_uri = "file:///workspace/lib/Test.pm";
    let content = "sub used { 1 }\nsub unused { 2 }\nused();\n1;";
    index.index_file(url::Url::parse(file_uri).unwrap(), content.to_string()).unwrap();
    let detector = DeadCodeDetector::new(index);
    let analysis = detector.analyze_workspace();
    assert!(analysis.dead_code.iter().any(|d| d.name.as_deref() == Some("unused")));
}
