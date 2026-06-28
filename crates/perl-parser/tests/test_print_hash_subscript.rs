use perl_parser::Parser;

#[test]
fn print_hash_subscript_not_indirect_call() {
    let code = "print $config{host};";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    let sexp = ast.to_sexp();

    println!("Code: {}", code);
    println!("S-expr: {}", sexp);

    // This should NOT contain indirect_call
    if sexp.contains("indirect_call") {
        panic!("FAIL: print $config{{host}} was misparsed as indirect_call");
    }
}

#[test]
fn print_array_subscript_not_indirect_call() {
    let code = "print $array[0];";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    let sexp = ast.to_sexp();

    println!("Code: {}", code);
    println!("S-expr: {}", sexp);

    // This should NOT contain indirect_call
    if sexp.contains("indirect_call") {
        panic!("FAIL: print $array[0] was misparsed as indirect_call");
    }
}

#[test]
fn print_legitimate_indirect_call() {
    let code = r#"print $fh "text";"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    let sexp = ast.to_sexp();

    println!("Code: {}", code);
    println!("S-expr: {}", sexp);

    // This SHOULD contain indirect_call
    assert!(sexp.contains("indirect_call"), "print $fh \"text\" should be indirect_call");
}
