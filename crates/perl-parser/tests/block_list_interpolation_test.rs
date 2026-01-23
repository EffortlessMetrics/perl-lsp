#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_parser::{NodeKind, Parser};

#[test]
fn test_simple_array_interpolation() {
    // Test basic array interpolation @{[...]}
    let mut parser = Parser::new(r#"my $str = "Array: @{[1, 2, 3]}";"#);
    let ast = parser.parse().expect("Failed to parse array interpolation");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(
                        value.contains("@{[1, 2, 3]}"),
                        "Should contain interpolation construct"
                    );
                } else {
                    panic!("Expected string initializer");
                }
            } else {
                panic!("Expected initializer");
            }
        } else {
            panic!("Expected variable declaration");
        }
    } else {
        panic!("Expected program");
    }
}

#[test]
fn test_complex_map_interpolation() {
    // Test complex expression with map inside interpolation
    let mut parser = Parser::new(r#"my $str = "Complex: @{[ map { $_ * 2 } @array ]}";"#);
    let ast = parser.parse().expect("Failed to parse complex interpolation");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(
                        value.contains("@{[ map { $_ * 2 } @array ]}"),
                        "Should contain complex interpolation construct"
                    );
                } else {
                    panic!("Expected string initializer");
                }
            } else {
                panic!("Expected initializer");
            }
        } else {
            panic!("Expected variable declaration");
        }
    } else {
        panic!("Expected program");
    }
}

#[test]
fn test_hash_interpolation() {
    // Test hash variable interpolation ${...}
    let mut parser = Parser::new(r#"my $str = "Hash: ${hash{key}}";"#);
    let ast = parser.parse().expect("Failed to parse hash interpolation");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(value.contains("${hash{key}}"), "Should contain hash interpolation");
                } else {
                    panic!("Expected string initializer");
                }
            } else {
                panic!("Expected initializer");
            }
        } else {
            panic!("Expected variable declaration");
        }
    } else {
        panic!("Expected program");
    }
}

#[test]
fn test_variable_interpolation() {
    // Test simple variable interpolation ${...}
    let mut parser = Parser::new(r#"my $str = "Value: ${name}";"#);
    let ast = parser.parse().expect("Failed to parse variable interpolation");

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(value.contains("${name}"), "Should contain variable interpolation");
                } else {
                    panic!("Expected string initializer");
                }
            } else {
                panic!("Expected initializer");
            }
        } else {
            panic!("Expected variable declaration");
        }
    } else {
        panic!("Expected program");
    }
}
