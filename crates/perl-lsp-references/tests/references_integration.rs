use perl_lsp_references::find_references_single_file;
use perl_parser_core::{Node, Parser};
use perl_tdd_support::{must, must_some};

fn parse(source: &str) -> Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

#[test]
fn finds_variable_references_in_single_file() {
    let source = "my $count = 0;\n$count += 1;\nprint $count;\n";
    let ast = parse(source);
    let offset = must_some(source.find("$count"));

    let references = must_some(find_references_single_file(&ast, offset));

    assert_eq!(references.len(), 3);
}

#[test]
fn finds_subroutine_references_across_definition_and_calls() {
    let source = "sub validate { 1 }\nvalidate();\nmain::validate();\n";
    let ast = parse(source);
    let offset = must_some(source.find("validate"));

    let references = must_some(find_references_single_file(&ast, offset));

    assert_eq!(references.len(), 3);
}

#[test]
fn returns_none_for_non_symbol_offsets() {
    let source = "# comment\nmy $value = 1;\n";
    let ast = parse(source);

    assert!(find_references_single_file(&ast, 0).is_none());
}
