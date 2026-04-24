//! Focused tests for first module-move import rewrite slice.

use perl_refactoring::module_move_imports::ModuleMoveImportRewriter;
use std::path::PathBuf;

#[test]
fn rewrites_imports_and_qualified_references_across_multiple_consumers()
-> Result<(), Box<dyn std::error::Error>> {
    let rewriter = ModuleMoveImportRewriter::new("Old::Name", "New::Name");

    let files = vec![
        (
            PathBuf::from("lib/ConsumerOne.pm"),
            "use strict;\nuse Old::Name qw(run);\nsub call { Old::Name::run(); }\n".to_string(),
        ),
        (
            PathBuf::from("lib/ConsumerTwo.pm"),
            "use warnings;\nuse Old::Name;\nmy $value = Old::Name::helper();\n".to_string(),
        ),
        (
            PathBuf::from("lib/Ambiguous.pm"),
            "my $text = 'Old::Name::run';\n# Old::Name::helper\n".to_string(),
        ),
    ];

    let edits = rewriter.rewrite_workspace(&files);
    assert_eq!(edits.len(), 2, "only clear consumer files should be edited");

    let first = edits
        .iter()
        .find(|edit| edit.file_path == PathBuf::from("lib/ConsumerOne.pm"))
        .ok_or("missing ConsumerOne.pm edits")?;
    let first_text = first
        .edits
        .iter()
        .map(|edit| edit.new_text.as_str())
        .collect::<String>();
    assert!(first_text.contains("use New::Name qw(run);"));
    assert!(first_text.contains("sub call { New::Name::run(); }"));

    let second = edits
        .iter()
        .find(|edit| edit.file_path == PathBuf::from("lib/ConsumerTwo.pm"))
        .ok_or("missing ConsumerTwo.pm edits")?;
    let second_text = second
        .edits
        .iter()
        .map(|edit| edit.new_text.as_str())
        .collect::<String>();
    assert!(second_text.contains("use New::Name;"));
    assert!(second_text.contains("my $value = New::Name::helper();"));

    assert!(edits
        .iter()
        .all(|edit| edit.file_path != PathBuf::from("lib/Ambiguous.pm")));

    Ok(())
}

#[test]
fn leaves_comments_and_strings_untouched_in_mixed_line() -> Result<(), Box<dyn std::error::Error>> {
    let rewriter = ModuleMoveImportRewriter::new("Old::Name", "New::Name");
    let file = PathBuf::from("lib/Mixed.pm");
    let content = "my $x = Old::Name::run(); # Old::Name::run\nmy $s = \"Old::Name::run\";\n";

    let edit = rewriter.rewrite_file(&file, content).ok_or("expected edit")?;
    assert_eq!(edit.edits.len(), 1, "only the safe qualified call line should be edited");
    let rewritten = &edit.edits[0].new_text;

    assert!(rewritten.contains("my $x = New::Name::run(); # Old::Name::run"));
    assert!(!rewritten.contains("my $s = \"Old::Name::run\";"));

    Ok(())
}
