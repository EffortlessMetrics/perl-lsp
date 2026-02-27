use perl_parser::{NodeKind, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_simple_array_interpolation() -> TestResult {
    // Test basic array interpolation @{[...]}
    let mut parser = Parser::new(r#"my $str = "Array: @{[1, 2, 3]}";"#);
    let ast = parser.parse()?;

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
                    return Err("Expected string initializer".into());
                }
            } else {
                return Err("Expected initializer".into());
            }
        } else {
            return Err("Expected variable declaration".into());
        }
    } else {
        return Err("Expected program".into());
    }
    Ok(())
}

#[test]
fn test_complex_map_interpolation() -> TestResult {
    // Test complex expression with map inside interpolation
    let mut parser = Parser::new(r#"my $str = "Complex: @{[ map { $_ * 2 } @array ]}";"#);
    let ast = parser.parse()?;

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
                    return Err("Expected string initializer".into());
                }
            } else {
                return Err("Expected initializer".into());
            }
        } else {
            return Err("Expected variable declaration".into());
        }
    } else {
        return Err("Expected program".into());
    }
    Ok(())
}

#[test]
fn test_hash_interpolation() -> TestResult {
    // Test hash variable interpolation ${...}
    let mut parser = Parser::new(r#"my $str = "Hash: ${hash{key}}";"#);
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(value.contains("${hash{key}}"), "Should contain hash interpolation");
                } else {
                    return Err("Expected string initializer".into());
                }
            } else {
                return Err("Expected initializer".into());
            }
        } else {
            return Err("Expected variable declaration".into());
        }
    } else {
        return Err("Expected program".into());
    }
    Ok(())
}

#[test]
fn test_variable_interpolation() -> TestResult {
    // Test simple variable interpolation ${...}
    let mut parser = Parser::new(r#"my $str = "Value: ${name}";"#);
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);

        if let NodeKind::VariableDeclaration { variable: _, initializer, .. } = &statements[0].kind
        {
            if let Some(init) = initializer {
                if let NodeKind::String { value, interpolated } = &init.kind {
                    assert_eq!(interpolated, &true, "String should be interpolated");
                    assert!(value.contains("${name}"), "Should contain variable interpolation");
                } else {
                    return Err("Expected string initializer".into());
                }
            } else {
                return Err("Expected initializer".into());
            }
        } else {
            return Err("Expected variable declaration".into());
        }
    } else {
        return Err("Expected program".into());
    }
    Ok(())
}
