use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_and_provider(code: &str) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, None)
}

fn completions_at_path(code: &str, pos: usize, path: &str) -> Vec<CompletionItem> {
    let provider = parse_and_provider(code);
    provider.get_completions_with_path(code, pos, Some(path))
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|item| item.label.clone()).collect()
}

fn kind_for(items: &[CompletionItem], label: &str) -> Option<CompletionItemKind> {
    items.iter().find(|item| item.label == label).map(|item| item.kind)
}

#[test]
fn xs_api_completion_is_gated_to_xs_sources() {
    let code = "package My::Module;\nnew";
    let items = completions_at_path(code, code.len(), "example.pl");
    let names = labels(&items);

    assert!(!names.contains(&"dXSARGS".to_string()));
    assert!(!names.contains(&"newSVpv".to_string()));
    assert!(!names.contains(&"PL_sv_yes".to_string()));
}

#[test]
fn xs_api_completion_is_available_in_xs_sources() {
    let code = "package My::Module;\n";
    let items = completions_at_path(code, code.len(), "example.xs");
    let names = labels(&items);

    assert!(names.contains(&"dXSARGS".to_string()));
    assert!(names.contains(&"ST".to_string()));
    assert!(names.contains(&"newSVpv".to_string()));
    assert!(names.contains(&"PL_sv_yes".to_string()));
    assert_eq!(kind_for(&items, "ST"), Some(CompletionItemKind::Snippet));
    assert_eq!(kind_for(&items, "newSVpv"), Some(CompletionItemKind::Snippet));
}
