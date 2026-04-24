use super::*;
use perl_tdd_support::must;

#[test]
fn test_legacy_typed_my_declaration_parses_without_error_node()
-> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("sub new { my Debconf::DbDriver $this = shift; return $this; }");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(
        !sexp.contains("ERROR"),
        "Expected typed my declaration to parse without ERROR node, got: {sexp}",
    );
    assert!(
        sexp.contains("my_declaration (variable $ this)"),
        "Expected my declaration variable in sexp, got: {sexp}",
    );
    Ok(())
}

#[test]
fn test_plain_my_declaration_not_affected() -> Result<(), Box<dyn std::error::Error>> {
    // Regression: ensure ordinary `my $x = 1` is NOT accidentally treated as
    // a typed declaration (the heuristic must not consume the variable token).
    let mut parser = Parser::new("my $x = 1;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(
        !sexp.contains("ERROR"),
        "Plain my declaration should parse without ERROR node, got: {sexp}",
    );
    assert!(sexp.contains("(variable $ x)"), "Expected variable $x in sexp, got: {sexp}",);
    Ok(())
}

#[test]
fn test_legacy_typed_our_declaration_parses_without_error_node()
-> Result<(), Box<dyn std::error::Error>> {
    // `our` shares the same non-local declarator path, so the type-constraint
    // consumer applies there too.
    let mut parser = Parser::new("our Foo::Bar $baz;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(
        !sexp.contains("ERROR"),
        "Typed our declaration should parse without ERROR node, got: {sexp}",
    );
    Ok(())
}

#[test]
fn test_legacy_typed_declaration_with_qualified_type_constraint()
-> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my ExtUtils::Typemaps::Type $type = shift;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(
        !sexp.contains("ERROR"),
        "Qualified typed declaration should parse without ERROR node, got: {sexp}",
    );
    assert!(
        sexp.contains("(variable $ type)"),
        "Expected variable `$type` after qualified type constraint, got: {sexp}",
    );
    Ok(())
}
