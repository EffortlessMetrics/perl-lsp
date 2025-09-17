/// Comprehensive tests for substitution operator (s///) parsing
/// This test module ensures complete coverage of the substitution operator
/// including edge cases, modifiers, and special delimiters
use perl_parser::{Parser, ast::NodeKind};


#[test]
// #[ignore = "substitution operator not implemented"]
fn test_basic_substitution() {
    let code = "s/foo/bar/";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, "foo");
                assert_eq!(replacement, "bar");
                assert_eq!(modifiers, "");
            } else {
                panic!("Expected Substitution node in expression, got {:?}", expression.kind);
            }
        } else {
            panic!("Expected ExpressionStatement node, got {:?}", statements[0].kind);
        }
    } else {
        panic!("Expected Program node");
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_modifiers() {
    let test_cases = vec![
        ("s/foo/bar/g", "g"),
        ("s/foo/bar/i", "i"),
        ("s/foo/bar/gi", "gi"),
        ("s/foo/bar/gix", "gix"),
        ("s/foo/bar/msxi", "msxi"),
        ("s/foo/bar/e", "e"),
        ("s/foo/bar/ee", "ee"),
        ("s/foo/bar/eeg", "eeg"),
        ("s/foo/bar/r", "r"),
    ];

    for (code, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { modifiers, .. } = &expression.kind {
                    assert_eq!(modifiers, expected_modifiers, "Failed for {}", code);
                } else {
                    panic!("Expected Substitution node in expression for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
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

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_nested_delimiters() {
    let test_cases = vec![
        ("s{f{o}o}{b{a}r}", "f{o}o", "b{a}r"),
        ("s[f[o]o][b[a]r]", "f[o]o", "b[a]r"),
        ("s(f(o)o)(b(a)r)", "f(o)o", "b(a)r"),
        ("s<f<o>o><b<a>r>", "f<o>o", "b<a>r"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_special_chars() {
    let test_cases = vec![
        (r#"s/\n/\\n/"#, r"\n", r"\\n"),
        (r#"s/\t/\s/"#, r"\t", r"\s"),
        (r#"s/\$var/\$new/"#, r"\$var", r"\$new"),
        (r#"s/\@array/\@new/"#, r"\@array", r"\@new"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_empty_pattern_or_replacement() {
    let test_cases = vec![("s///", "", ""), ("s/foo//", "foo", ""), ("s//bar/", "", "bar")];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_with_expressions() {
    // Test the /e modifier which evaluates replacement as Perl code
    let code = r#"s/(\d+)/sprintf("%02d", $1)/eg"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");

    if let NodeKind::Program { statements } = &ast.kind {
        if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
            if let NodeKind::Substitution { pattern, replacement, modifiers, .. } = &expression.kind
            {
                assert_eq!(pattern, r"(\d+)");
                assert_eq!(replacement, r#"sprintf("%02d", $1)"#);
                assert_eq!(modifiers, "eg");
            } else {
                panic!("Expected Substitution node");
            }
        } else {
            panic!("Expected ExpressionStatement node");
        }
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_in_context() {
    let test_cases = vec![
        ("$str =~ s/foo/bar/g;", "foo", "bar", "g"),
        ("if ($line =~ s/^\\s+//) { }", r"^\s+", "", ""),
        ("while (s/  / /g) { }", "  ", " ", "g"),
    ];

    for (code, expected_pattern, expected_replacement, expected_modifiers) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        // Find the substitution node (might be nested)
        let found = find_substitution_node(&ast);
        assert!(found.is_some(), "No Substitution node found in {}", code);

        let (pattern, replacement, modifiers) = found.unwrap();
        assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
        assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
        assert_eq!(modifiers, expected_modifiers, "Modifiers mismatch for {}", code);
    }
}

#[test]
// #[ignore = "substitution operator not implemented"]
fn test_substitution_unicode() {
    let test_cases = vec![
        ("s/café/coffee/", "café", "coffee"),
        ("s/😀/😎/g", "😀", "😎"),
        ("s/λ/lambda/", "λ", "lambda"),
    ];

    for (code, expected_pattern, expected_replacement) in test_cases {
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap_or_else(|_| panic!("parse {}", code));

        if let NodeKind::Program { statements } = &ast.kind {
            if let NodeKind::ExpressionStatement { expression } = &statements[0].kind {
                if let NodeKind::Substitution { pattern, replacement, .. } = &expression.kind {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", code);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", code);
                } else {
                    panic!("Expected Substitution node for {}", code);
                }
            } else {
                panic!("Expected ExpressionStatement node for {}", code);
            }
        }
    }
}

// Helper function to find substitution node in AST
fn find_substitution_node(node: &perl_parser::ast::Node) -> Option<(String, String, String)> {
    match &node.kind {
        NodeKind::Substitution { pattern, replacement, modifiers, .. } => {
            Some((pattern.clone(), replacement.clone(), modifiers.clone()))
        }
        NodeKind::Program { statements } => {
            for stmt in statements {
                if let Some(result) = find_substitution_node(stmt) {
                    return Some(result);
                }
            }
            None
        }
        NodeKind::ExpressionStatement { expression } => {
            find_substitution_node(expression)
        }
        NodeKind::Binary { left, right, .. } => {
            find_substitution_node(left).or_else(|| find_substitution_node(right))
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                if let Some(result) = find_substitution_node(stmt) {
                    return Some(result);
                }
            }
            None
        }
        NodeKind::If { condition, then_branch, else_branch, .. } => {
            find_substitution_node(condition)
                .or_else(|| find_substitution_node(then_branch))
                .or_else(|| else_branch.as_ref().and_then(|b| find_substitution_node(b)))
        }
        NodeKind::While { condition, body, .. } => {
            find_substitution_node(condition).or_else(|| find_substitution_node(body))
        }
        _ => None,
    }
}
