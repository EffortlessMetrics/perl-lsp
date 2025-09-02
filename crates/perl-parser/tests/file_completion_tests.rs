use perl_parser::{CompletionItemKind, CompletionProvider, Parser};

#[test]
fn completes_files_in_src_directory() {
    let code = "\"src/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").unwrap() + "com".len();
    let completions = provider.get_completions(code, pos);

    // Should find completion.rs file
    assert!(
        completions
            .iter()
            .any(|c| c.label == "src/completion.rs" && c.kind == CompletionItemKind::File)
    );

    // Should provide proper file type information
    let completion_file = completions.iter().find(|c| c.label == "src/completion.rs");
    if let Some(comp) = completion_file {
        assert_eq!(comp.detail.as_ref().unwrap(), "Rust source file");
    }
}

#[test]
fn completes_files_in_tests_directory() {
    let code = "\"tests/file_comp\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("file_comp").unwrap() + "file_comp".len();
    let completions = provider.get_completions(code, pos);

    // Should find our comprehensive test file
    assert!(completions.iter().any(|c| c.label.contains("file_completion")));
}

#[test]
fn does_complete_current_directory_files() {
    let code = "\"Cargo\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("Cargo").unwrap() + "Cargo".len();
    let completions = provider.get_completions(code, pos);

    // Should provide file completions for current directory files matching prefix
    assert!(completions.iter().any(|c| c.label.starts_with("Cargo")));
}

#[test]
fn basic_security_test_rejects_path_traversal() {
    let code = "\"../src/completion.rs\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("completion").unwrap() + "completion".len();
    let completions = provider.get_completions(code, pos);

    // Should reject path traversal attempts
    assert!(completions.is_empty());
}
