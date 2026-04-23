//! Fixed substitution operator tests with correct AST structure

use perl_parser::{Parser, ast::NodeKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// Helper function to extract substitution node from AST
fn extract_substitution(ast: &perl_parser::ast::Node) -> Option<(&str, &str, &str)> {
    if let NodeKind::Program { statements } = &ast.kind
        && let Some(stmt) = statements.first()
        && let NodeKind::ExpressionStatement { expression } = &stmt.kind
        && let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
    {
        return Some((pattern, replacement, modifiers));
    }
    None
}

#[test]
fn test_basic_substitution() -> TestResult {
    let code = "s/foo/bar/";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    if let Some((pattern, replacement, modifiers)) = extract_substitution(&ast) {
        assert_eq!(pattern, "foo");
        assert_eq!(replacement, "bar");
        assert_eq!(modifiers, "");
    } else {
        return Err("Expected Substitution node".into());
    }
    Ok(())
}

#[test]
fn test_substitution_with_modifiers() -> TestResult {
    let test_cases = vec![
        ("s/foo/bar/g", "g"),
        ("s/foo/bar/i", "i"),
        ("s/foo/bar/gi", "gi"),
        ("s/foo/bar/gix", "gix"),
        ("s/foo/bar/msxi", "msxi"),
        ("s/foo/bar/e", "e"),
        ("s/foo/bar/r", "r"),
    ];

    for (code, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;

        if let Some((_pattern, _replacement, modifiers)) = extract_substitution(&ast) {
            assert_eq!(modifiers, expected_modifiers, "Failed for {}", code);
        } else {
            return Err(format!("Expected Substitution node for {}", code).into());
        }
    }
    Ok(())
}

#[test]
fn test_substitution_with_different_delimiters() -> TestResult {
    let test_cases = vec![
        ("s(foo)(bar)", "foo", "bar"),
        ("s{foo}{bar}", "foo", "bar"),
        ("s[foo][bar]", "foo", "bar"),
        ("s<foo><bar>", "foo", "bar"),
        ("s#foo#bar#", "foo", "bar"),
        ("s!foo!bar!", "foo", "bar"),
        ("s|foo|bar|", "foo", "bar"),
        ("s,foo,bar,", "foo", "bar"),
        ("s'foo'bar'", "foo", "bar"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {}: {:?}", code, e))?;

        if let Some((pattern, replacement, _modifiers)) = extract_substitution(&ast) {
            assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
            assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        } else {
            return Err(format!("Expected Substitution node for {}", code).into());
        }
    }
    Ok(())
}

#[test]
fn test_substitution_empty_pattern_or_replacement() -> TestResult {
    let test_cases = vec![("s///", "", ""), ("s/foo//", "foo", ""), ("s//bar/", "", "bar")];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().map_err(|e| format!("parse {}: {:?}", code, e))?;

        if let Some((pattern, replacement, _modifiers)) = extract_substitution(&ast) {
            assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
            assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        } else {
            return Err(format!("Expected Substitution node for {}", code).into());
        }
    }
    Ok(())
}
