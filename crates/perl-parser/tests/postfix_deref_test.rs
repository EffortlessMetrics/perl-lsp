use perl_parser::Parser;

#[test]
fn test_postfix_array_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->@*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->@*"));
    Ok(())
}

#[test]
fn test_postfix_hash_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->%*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->%*"));
    Ok(())
}

#[test]
fn test_postfix_scalar_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->$*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->$*"));
    Ok(())
}

#[test]
fn test_postfix_code_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->&*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->&*"));
    Ok(())
}

#[test]
fn test_postfix_glob_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->**;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->**"));
    Ok(())
}

#[test]
fn test_postfix_array_slice() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->@[0..2];");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_->@[]"));
    Ok(())
}

#[test]
fn test_postfix_hash_slice() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$ref->%{'key'};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_->%{}"));
    Ok(())
}

#[test]
fn test_chained_postfix_deref() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$data->[0]->@*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->@*"));
    assert!(sexp.contains("binary_[]"));
    Ok(())
}

#[test]
fn test_postfix_deref_in_expression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my @array = $ref->@*;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("unary_->@*"));
    Ok(())
}
