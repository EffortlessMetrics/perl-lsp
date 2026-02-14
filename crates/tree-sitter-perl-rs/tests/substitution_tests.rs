#[cfg(all(feature = "pure-rust", not(feature = "pure-rust-standalone")))]
mod tests {
    use tree_sitter_perl::{NodeKind, ParserV2};

    fn parse_first_node(code: &str) -> NodeKind {
        let mut parser = ParserV2::new(code);
        let ast = parser.parse().expect("parse");
        match ast.kind {
            NodeKind::Program { statements } => statements[0].kind.clone(),
            other => must(Err::<(), _>(format!("unexpected AST root: {:?}", other))),
        }
    }

    #[test]
    fn captures_replacement_and_modifier() {
        match parse_first_node("s/foo/bar/g;") {
            NodeKind::Substitution { pattern, replacement, modifiers } => {
                assert_eq!(pattern.as_ref(), "foo");
                assert_eq!(replacement.as_ref(), "bar");
                assert_eq!(modifiers.as_ref(), "g");
            }
            other => must(Err::<(), _>(format!("expected substitution node, got {:?}", other))),
        }
    }

    #[test]
    fn captures_multiple_modifiers() {
        match parse_first_node("s/foo/bar/gi;") {
            NodeKind::Substitution { pattern, replacement, modifiers } => {
                assert_eq!(pattern.as_ref(), "foo");
                assert_eq!(replacement.as_ref(), "bar");
                assert_eq!(modifiers.as_ref(), "gi");
            }
            other => must(Err::<(), _>(format!("expected substitution node, got {:?}", other))),
        }
    }

    #[test]
    fn single_quote_delimiters() {
        let cases = [
            ("s'foo'bar';", "foo", "bar", ""),
            ("s'foo'bar'gi;", "foo", "bar", "gi"),
            ("s'it\\'s'it is';", "it\\'s", "it is", ""),
            ("s''bar';", "", "bar", ""),
            ("s'foo'';", "foo", "", ""),
        ];

        for (code, pat, repl, mods) in cases {
            match parse_first_node(code) {
                NodeKind::Substitution { pattern, replacement, modifiers } => {
                    assert_eq!(pattern.as_ref(), pat, "pattern mismatch for {code}");
                    assert_eq!(replacement.as_ref(), repl, "replacement mismatch for {code}");
                    assert_eq!(modifiers.as_ref(), mods, "modifier mismatch for {code}");
                }
                other => must(Err::<(), _>(format!("expected substitution node, got {:?}", other))),
            }
        }
    }
}
