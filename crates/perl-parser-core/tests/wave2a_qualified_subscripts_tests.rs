use perl_parser_core::Parser;

#[test]
fn wave2a_scalar_qualified_array_subscript() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$Text::Unidecode::Char[255];");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    assert!(sexp.contains("[]"), "should have array subscript");
    Ok(())
}

#[test]
fn wave2a_scalar_qualified_array_subscript_hex() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$Text::Unidecode::Char[0xff];");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    assert!(sexp.contains("[]"), "should have array subscript");
    Ok(())
}

#[test]
fn wave2a_scalar_qualified_hash_subscript() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$Package::Hash{key};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    assert!(sexp.contains("{}"), "should have hash subscript");
    Ok(())
}

#[test]
fn wave2a_deep_qualified_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$A::B::C::D[42];");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2a_qualified_hash_string_key() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$Config::Config{'osname'};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2a_qualified_in_expression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $val = $Pkg::data{$key};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2a_unqualified_regression() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure normal subscripts still work
    let mut parser = Parser::new("$hash{key};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    Ok(())
}
