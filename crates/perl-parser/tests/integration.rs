//! Integration tests for the perl-parser crate

use perl_parser::{NodeKind, Parser};

#[test]
fn test_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = 42;");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("variable"));
    assert!(sexp.contains("42"));
    Ok(())
}

#[test]
fn test_array_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my @array = (1, 2, 3);");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("array"));
    Ok(())
}

#[test]
fn test_if_statement() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("if ($x > 10) { print $x; }");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("if"));
    assert!(sexp.contains("binary_>"));
    Ok(())
}

#[test]
fn test_while_loop() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("while ($i < 10) { $i++; }");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("while"));
    assert!(sexp.contains("binary_<"));
    Ok(())
}

#[test]
fn test_function_definition() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("sub hello { return \"world\"; }");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("sub hello"));
    assert!(sexp.contains("return"));
    Ok(())
}

#[test]
fn test_complex_expression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$result = ($a + $b) * $c;");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("assignment"));
    assert!(sexp.contains("binary_*"));
    assert!(sexp.contains("binary_+"));
    Ok(())
}

#[test]
fn test_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$obj->method($arg);");
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("method_call"));
    Ok(())
}

#[test]
fn test_nested_structures() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
if ($x) {
    while ($y) {
        for (my $i = 0; $i < 10; $i++) {
            print $i;
        }
    }
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let sexp = ast.to_sexp();
    assert!(sexp.contains("if"));
    assert!(sexp.contains("while"));
    assert!(sexp.contains("for"));
    Ok(())
}

#[test]
fn test_error_recovery() {
    // Missing semicolon - parser should still work
    let mut parser = Parser::new("my $x = 42\nmy $y = 84;");
    let result = parser.parse();

    // Should still parse successfully
    assert!(result.is_ok());
}

#[test]
fn test_empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("");
    let ast = parser.parse()?;

    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 0);
    } else {
        return Err("Expected Program node".into());
    }
    Ok(())
}
