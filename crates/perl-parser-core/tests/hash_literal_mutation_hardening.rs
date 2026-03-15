//! Mutation hardening tests for hash literal parsing.
//!
//! These tests specifically protect the `=>`/`,` branch selection logic in
//! `parse_braced_expression` so hash literals are not misparsed as blocks.

use perl_parser_core::{Node, NodeKind, Parser};

fn parse_single_initializer(source: &str) -> Result<Node, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = ast.kind {
        if statements.len() != 1 {
            return Err(format!(
                "expected exactly one top-level statement, got {}",
                statements.len()
            )
            .into());
        }

        if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &statements[0].kind {
            return Ok(*init.clone());
        }
    }

    Err("expected variable declaration with initializer".into())
}

#[test]
fn hash_literal_with_fat_arrow_is_not_parsed_as_block() -> Result<(), Box<dyn std::error::Error>> {
    let initializer = parse_single_initializer("my $h = { foo => 1, bar => 2 };")?;

    if let NodeKind::HashLiteral { pairs } = initializer.kind {
        assert_eq!(pairs.len(), 2, "expected two hash pairs");
        // Fat arrow auto-quotes barewords: `foo =>` produces a String node
        assert!(
            matches!(pairs[0].0.kind, NodeKind::Identifier { .. } | NodeKind::String { .. }),
            "first key should be an identifier or auto-quoted string"
        );
        assert!(
            matches!(pairs[0].1.kind, NodeKind::Number { .. }),
            "first value should be numeric"
        );
        return Ok(());
    }

    Err("expected HashLiteral initializer".into())
}

#[test]
fn hash_literal_with_comma_pairs_stays_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    let initializer = parse_single_initializer("my $h = { foo, 1, bar, 2 };")?;

    if let NodeKind::HashLiteral { pairs } = initializer.kind {
        assert_eq!(pairs.len(), 2, "expected two hash pairs");
        assert!(
            pairs.iter().all(|(_, value)| matches!(value.kind, NodeKind::Number { .. })),
            "all values should be numbers"
        );
        return Ok(());
    }

    Err("expected HashLiteral initializer".into())
}
