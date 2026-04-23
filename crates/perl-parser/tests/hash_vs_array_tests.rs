#![allow(clippy::collapsible_if)]

use perl_parser::{Parser, ast::NodeKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_parenthesized_hash_with_fat_comma() -> TestResult {
    let code = "my %h = (a => 1, b => 2);";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Find the variable declaration
    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(stmt) = statements.first() {
            if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
                // Should be a HashLiteral
                assert!(
                    matches!(&init.kind, NodeKind::HashLiteral { .. }),
                    "Expected HashLiteral for (a => 1, b => 2), got {:?}",
                    init.kind
                );
                return Ok(());
            }
        }
    }
    Err("Failed to find expected structure".into())
}

#[test]
fn test_parenthesized_array_without_fat_comma() -> TestResult {
    let code = "my @a = (1, 2, 3, 4);";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Find the assignment
    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(stmt) = statements.first() {
            if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
                // Should remain an ArrayLiteral
                assert!(
                    matches!(&init.kind, NodeKind::ArrayLiteral { .. }),
                    "Expected ArrayLiteral for (1, 2, 3, 4), got {:?}",
                    init.kind
                );
                return Ok(());
            }
        }
    }
    Err("Failed to find expected structure".into())
}

#[test]
fn test_parenthesized_array_with_identifier_pairs() -> TestResult {
    let code = "my @a = (a, 1, b, 2);";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Find the assignment
    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(stmt) = statements.first() {
            if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
                // Should remain an ArrayLiteral (no fat comma)
                assert!(
                    matches!(&init.kind, NodeKind::ArrayLiteral { .. }),
                    "Expected ArrayLiteral for (a, 1, b, 2) without fat comma, got {:?}",
                    init.kind
                );
                return Ok(());
            }
        }
    }
    Err("Failed to find expected structure".into())
}

#[test]
fn test_mixed_commas_still_hash() -> TestResult {
    let code = "my %h = (a => 1, b, 2);";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Find the assignment
    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(stmt) = statements.first() {
            if let NodeKind::VariableDeclaration { initializer: Some(init), .. } = &stmt.kind {
                // Should be a HashLiteral because it has at least one fat comma
                assert!(
                    matches!(&init.kind, NodeKind::HashLiteral { .. }),
                    "Expected HashLiteral for (a => 1, b, 2) with mixed separators, got {:?}",
                    init.kind
                );
                return Ok(());
            }
        }
    }
    Err("Failed to find expected structure".into())
}
