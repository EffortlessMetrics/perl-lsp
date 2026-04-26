use perl_parser::Parser;

type TestResult = Result<(), String>;

#[test]
fn test_complex_subroutine_signatures() -> TestResult {
    let input = "sub test($x) { return $x; }";
    let mut parser = Parser::new(input);
    let ast = parser.parse().unwrap();
    let sexp = ast.to_sexp();
    println!("{}", sexp);
    assert!(sexp.contains("sub") || sexp.contains("subroutine") || sexp.contains("Subroutine"));
    Ok(())
}
