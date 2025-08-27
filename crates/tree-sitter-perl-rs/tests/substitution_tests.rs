#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::{NodeKind, ParserV2};

    fn parse_first_node(code: &str) -> NodeKind {
        let mut parser = ParserV2::new(code);
        let ast = parser.parse().expect("parse");
        match ast.kind {
            NodeKind::Program { statements } => statements[0].kind.clone(),
            other => panic!("unexpected AST root: {:?}", other),
        }
    }

    #[test]
    fn captures_replacement_and_modifier() {
        match parse_first_node("s/foo/bar/g;") {
            NodeKind::Regex { pattern, replacement, modifiers } => {
                assert_eq!(pattern.as_ref(), "foo");
                assert_eq!(replacement.as_ref().map(|s| s.as_ref()), Some("bar"));
                assert_eq!(modifiers.as_ref(), "g");
            }
            other => panic!("expected regex node, got {:?}", other),
        }
    }

    #[test]
    fn captures_multiple_modifiers() {
        match parse_first_node("s/foo/bar/gi;") {
            NodeKind::Regex { pattern, replacement, modifiers } => {
                assert_eq!(pattern.as_ref(), "foo");
                assert_eq!(replacement.as_ref().map(|s| s.as_ref()), Some("bar"));
                assert_eq!(modifiers.as_ref(), "gi");
            }
            other => panic!("expected regex node, got {:?}", other),
        }
    }
}
