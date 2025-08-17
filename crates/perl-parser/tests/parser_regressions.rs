use perl_parser::Parser;

/// Helper to assert code parses successfully
fn assert_parses(code: &str) {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => {}
        Err(e) => panic!("Failed to parse code:\n{}\nError: {:?}", code, e),
    }
}

/// Helper to assert code fails to parse
#[allow(dead_code)]
fn assert_parse_fails(code: &str) {
    let mut parser = Parser::new(code);
    if let Ok(ast) = parser.parse() {
        panic!("Expected parse to fail but got AST:\n{:?}", ast)
    }
}

#[test]
fn print_scalar_in_simple_context() {
    // Basic print $var should work
    assert_parses("print $x;");
    assert_parses("print $x");
    assert_parses("{ print $x; }");
    assert_parses("if (1) { print $x; }");
}

#[test]
#[ignore] // Known issue: complex parser state management needed  
fn print_scalar_after_my_inside_if() {
    let code = r#"
my $y = 10;
if (1) {
    print $y;
}
"#;
    assert_parses(code);
}

#[test]
fn print_scalar_with_arithmetic() {
    let code = r#"print $x + 1;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x + 1");

    // Should parse as print($x + 1), NOT as indirect object
    let ast = ast.unwrap();
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse arithmetic as indirect object: {}",
        sexp
    );
}

#[test]
fn print_scalar_with_string_concat() {
    let code = r#"print $x . "s";"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x . \"s\"");

    // Should parse as print($x . "s"), NOT as indirect object
    let ast = ast.unwrap();
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse string concat as indirect object: {}",
        sexp
    );
}

#[test]
fn print_indirect_object_still_works() {
    // These should parse as indirect object syntax
    assert_parses(r#"open($fh, '<', 'x.txt'); print $fh "hi\n";"#);
    assert_parses(r#"print STDOUT "hello";"#);
    assert_parses(r#"print STDERR "error", "\n";"#);
    assert_parses(r#"say $fh "message";"#);
}

#[test]
fn print_scalar_vs_indirect_object() {
    // print $var; should NOT be treated as indirect object
    assert_parses("print $x;");
    assert_parses("print $x, $y;");
    assert_parses("print $array[0];");
    assert_parses("print $hash{key};");

    // print $fh ... with more args should be indirect object
    assert_parses(r#"print $fh "text";"#);
    assert_parses(r#"print $fh "text", "more";"#);
}

#[test]
fn new_constructor_pattern() {
    assert_parses("new Class");
    assert_parses("new Class()");
    assert_parses("new Class('arg')");
    assert_parses("$obj = new Class;");
}
