use perl_parser::Parser;

#[test]
fn test_postfix_array_deref() {
    let mut parser = Parser::new("$ref->@*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->@*"));
}

#[test]
fn test_postfix_hash_deref() {
    let mut parser = Parser::new("$ref->%*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->%*"));
}

#[test]
fn test_postfix_scalar_deref() {
    let mut parser = Parser::new("$ref->$*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->$*"));
}

#[test]
fn test_postfix_code_deref() {
    let mut parser = Parser::new("$ref->&*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->&*"));
}

#[test]
fn test_postfix_glob_deref() {
    let mut parser = Parser::new("$ref->**;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->**"));
}

#[test]
fn test_postfix_array_slice() {
    let mut parser = Parser::new("$ref->@[0..2];");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_->@[]"));
}

#[test]
fn test_postfix_hash_slice() {
    let mut parser = Parser::new("$ref->%{'key'};");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_->%{}"));
}

#[test]
fn test_chained_postfix_deref() {
    let mut parser = Parser::new("$data->[0]->@*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("unary_->@*"));
    assert!(sexp.contains("binary_[]"));
}

#[test]
fn test_postfix_deref_in_expression() {
    let mut parser = Parser::new("my @array = $ref->@*;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("my_declaration"));
    assert!(sexp.contains("unary_->@*"));
}
