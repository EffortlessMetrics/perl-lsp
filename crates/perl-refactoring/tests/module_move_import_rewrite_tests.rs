use perl_refactoring::workspace_refactor::{RefactorResult, WorkspaceRefactor};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;

fn setup_workspace(
    files: &[(&str, &str)],
) -> Result<(TempDir, WorkspaceRefactor), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let index = WorkspaceIndex::new();

    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        let uri = format!("file://{}", path.display());
        index.index_file_str(&uri, content).map_err(|e| format!("index_file_str failed: {}", e))?;
    }

    Ok((dir, WorkspaceRefactor::new(index)))
}

fn apply_result(
    result: &RefactorResult,
    original_files: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut rewritten = BTreeMap::new();

    for (file, content) in original_files {
        let mut updated = content.clone();
        if let Some(file_edits) =
            result.file_edits.iter().find(|edit| edit.file_path.ends_with(Path::new(file)))
        {
            let mut sorted = file_edits.edits.clone();
            sorted.sort_by_key(|edit| std::cmp::Reverse(edit.start));
            for edit in sorted {
                updated.replace_range(edit.start..edit.end, &edit.new_text);
            }
        }
        rewritten.insert(file.clone(), updated);
    }

    rewritten
}

#[test]
fn rewrite_moved_module_imports_updates_multiple_consumers(
) -> Result<(), Box<dyn std::error::Error>> {
    let consumer_a = "use Old::Name;\nmy $x = Old::Name::build();\n";
    let consumer_b = "use Old::Name qw(run);\nOld::Name->new();\n";
    let ambiguous = "my $s = 'Old::Name::build';\n# Old::Name::debug();\n";

    let original_files = BTreeMap::from([
        ("consumer_a.pl".to_string(), consumer_a.to_string()),
        ("consumer_b.pl".to_string(), consumer_b.to_string()),
        ("ambiguous.pl".to_string(), ambiguous.to_string()),
    ]);

    let (_dir, refactor) = setup_workspace(&[
        ("consumer_a.pl", consumer_a),
        ("consumer_b.pl", consumer_b),
        ("ambiguous.pl", ambiguous),
    ])?;

    let result = refactor.rewrite_moved_module_imports("Old::Name", "New::Name")?;
    let rewritten = apply_result(&result, &original_files);

    assert!(rewritten["consumer_a.pl"].contains("use New::Name;"));
    assert!(rewritten["consumer_a.pl"].contains("New::Name::build()"));
    assert!(rewritten["consumer_b.pl"].contains("use New::Name qw(run);"));
    assert!(rewritten["consumer_b.pl"].contains("New::Name->new()"));

    assert_eq!(rewritten["ambiguous.pl"], ambiguous);

    Ok(())
}

#[test]
fn rewrite_moved_module_imports_rejects_invalid_input() {
    let refactor = WorkspaceRefactor::new(WorkspaceIndex::new());

    assert!(refactor.rewrite_moved_module_imports("", "New::Name").is_err());
    assert!(refactor.rewrite_moved_module_imports("Old::Name", "").is_err());
    assert!(refactor.rewrite_moved_module_imports("Old::Name", "Old::Name").is_err());
}
