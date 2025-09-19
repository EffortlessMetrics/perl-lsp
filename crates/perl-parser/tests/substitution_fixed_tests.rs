/// Fixed substitution operator tests with correct AST structure
use perl_parser::{Parser, ast::NodeKind};

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
fn test_basic_substitution() {
    let code = "s/foo/bar/";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");

    if let Some((pattern, replacement, modifiers)) = extract_substitution(&ast) {
        assert_eq!(pattern, "foo");
        assert_eq!(replacement, "bar");
        assert_eq!(modifiers, "");
    } else {
        panic!("Expected Substitution node");
    }
}

#[test]
fn test_substitution_with_modifiers() {
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
        let ast = parser.parse().expect("parse");

        if let Some((_pattern, _replacement, modifiers)) = extract_substitution(&ast) {
            assert_eq!(modifiers, expected_modifiers, "Failed for {}", code);
        } else {
            panic!("Expected Substitution node for {}", code);
        }
    }
}

#[test]
fn test_substitution_with_different_delimiters() {
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
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let Some((pattern, replacement, _modifiers)) = extract_substitution(&ast) {
            assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
            assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        } else {
            panic!("Expected Substitution node for {}", code);
        }
    }
}

#[test]
fn test_substitution_empty_pattern_or_replacement() {
    let test_cases = vec![("s///", "", ""), ("s/foo//", "foo", ""), ("s//bar/", "", "bar")];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let Some((pattern, replacement, _modifiers)) = extract_substitution(&ast) {
            assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
            assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        } else {
            panic!("Expected Substitution node for {}", code);
        }
    }
}
