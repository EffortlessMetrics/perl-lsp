use perl_parser_pest::PureRustPerlParser;
use perl_tdd_support::must;

fn parse_source(source: &str) -> String {
    let mut parser = PureRustPerlParser::new();
    let ast = must(parser.parse(source));
    parser.to_sexp(&ast)
}

#[test]
fn bdd_given_scalar_assignment_when_parsed_then_sexp_contains_assignment_shape() {
    // Given
    let source = "my $answer = 42;";

    // When
    let sexp = parse_source(source);

    // Then
    assert!(sexp.contains("source_file"), "Expected root node in S-expression: {sexp}");
    assert!(sexp.contains("variable_declaration"), "Expected variable declaration: {sexp}");
    assert!(sexp.contains("$answer"), "Expected scalar variable name in output: {sexp}");
    assert!(sexp.contains("number"), "Expected numeric literal node: {sexp}");
}

#[test]
fn bdd_given_if_else_when_parsed_then_sexp_contains_both_branches() {
    // Given
    let source = r#"if ($x > 0) { print "positive"; } else { print "non-positive"; }"#;

    // When
    let sexp = parse_source(source);

    // Then
    assert!(sexp.contains("if_statement"), "Expected if statement: {sexp}");
    assert!(sexp.contains("binary_expression"), "Expected condition expression: {sexp}");
    assert!(sexp.contains("function_call"), "Expected body statement in S-expression: {sexp}");
    assert!(sexp.contains("positive"), "Expected then branch content in S-expression: {sexp}");
}

#[test]
fn bdd_given_subroutine_when_parsed_then_sexp_contains_sub_and_body() {
    // Given
    let source = r#"sub greet { return "hello"; }"#;

    // When
    let sexp = parse_source(source);

    // Then
    assert!(sexp.contains("subroutine"), "Expected sub declaration: {sexp}");
    assert!(sexp.contains("return_statement"), "Expected return statement: {sexp}");
    assert!(sexp.contains("string"), "Expected string literal in body: {sexp}");
}

#[test]
fn bdd_given_invalid_program_when_parsed_then_parser_returns_error() {
    // Given
    let source = "my = ;";
    let mut parser = PureRustPerlParser::new();

    // When
    let result = parser.parse(source);

    // Then
    assert!(result.is_err(), "Expected parse failure for invalid input");
}
