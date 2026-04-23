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
