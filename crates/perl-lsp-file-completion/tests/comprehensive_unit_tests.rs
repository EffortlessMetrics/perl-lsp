use perl_lsp_completion_item::CompletionItemKind;
use perl_lsp_file_completion::{FileCompletionContext, complete_file_paths};
use std::fs;

#[test]
fn completes_visible_matching_files_and_directories() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let cwd = std::env::current_dir()?;
    std::env::set_current_dir(tmp.path())?;

    fs::write("lib.pm", "1;\n")?;
    fs::write("lib.rs", "fn main() {}\n")?;
    fs::create_dir("libdir")?;

    let context = FileCompletionContext::new("lib", 3, 6);
    let completions = complete_file_paths(&context, &|| false);

    std::env::set_current_dir(cwd)?;

    let labels: Vec<_> = completions.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"lib.pm"));
    assert!(labels.contains(&"lib.rs"));
    assert!(labels.contains(&"libdir/"));
    assert!(
        completions
            .iter()
            .all(|item| item.kind == CompletionItemKind::File)
    );
    Ok(())
}

#[test]
fn rejects_traversal_prefixes() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let cwd = std::env::current_dir()?;
    std::env::set_current_dir(tmp.path())?;

    fs::write("safe.txt", "ok\n")?;

    let completions = complete_file_paths(&FileCompletionContext::new("../sec", 0, 6), &|| false);

    std::env::set_current_dir(cwd)?;

    assert!(completions.is_empty());
    Ok(())
}

#[test]
fn respects_cancellation_before_traversal() {
    let completions = complete_file_paths(&FileCompletionContext::new("", 0, 0), &|| true);
    assert!(completions.is_empty());
}
