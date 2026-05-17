use perl_parser::Parser;

type TestResult = Result<(), String>;

#[test]
fn test_complex_subroutine_signatures() -> TestResult {
    let input = "sub test($x) { return $x; }";
    let mut parser = Parser::new(input);
    let ast = perl_tdd_support::must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("sub") || sexp.contains("subroutine") || sexp.contains("Subroutine"));
    Ok(())
}
