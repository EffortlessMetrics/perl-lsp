use perl_parser::Parser;
use perl_tdd_support::must;

type TestResult = Result<(), String>;

#[test]
fn test_complex_subroutine_signatures() -> TestResult {
    let input = "sub test($x) { return $x; }";
    let mut parser = Parser::new(input);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("sub") || sexp.contains("subroutine") || sexp.contains("Subroutine"));
    Ok(())
}
