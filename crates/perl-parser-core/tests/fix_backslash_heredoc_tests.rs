use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

fn find_first_heredoc(node: &Node) -> Option<&NodeKind> {
    if matches!(node.kind, NodeKind::Heredoc { .. }) {
        return Some(&node.kind);
    }

    node.children().into_iter().find_map(find_first_heredoc)
}

#[test]
fn backslash_heredoc_uses_unescaped_delimiter_and_disables_interpolation()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<\\EOF;\nhello $name\nEOF\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let heredoc = find_first_heredoc(&ast).ok_or("expected heredoc node")?;
    match heredoc {
        NodeKind::Heredoc { delimiter, interpolated, content, .. } => {
            assert_eq!(delimiter, "EOF");
            assert!(!interpolated, "backslash heredocs should be non-interpolating");
            assert_eq!(content, "hello $name");
        }
        other => return Err(format!("expected heredoc node, got {other:?}").into()),
    }

    assert!(parser.errors().is_empty(), "unexpected parser errors: {:?}", parser.errors());
    Ok(())
}
