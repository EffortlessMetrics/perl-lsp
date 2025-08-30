use perl_parser::{CompletionItemKind, CompletionProvider, Parser};

#[test]
fn completes_files_in_src_directory() {
    let code = "\"src/com\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("com").unwrap() + "com".len();
    let completions = provider.get_completions(code, pos);
    assert!(
        completions
            .iter()
            .any(|c| c.label == "src/completion.rs" && c.kind == CompletionItemKind::File)
    );
}

#[test]
fn completes_files_in_tests_directory() {
    let code = "\"tests/incre\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("incre").unwrap() + "incre".len();
    let completions = provider.get_completions(code, pos);
    assert!(completions.iter().any(|c| c.label == "tests/incremental_integration_test.rs"));
}
