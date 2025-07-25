use perl_parser::Parser;

fn main() {
    let code = r#"sub mygrep(&@) { }"#;
    println!("Testing: {}", code);
    
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Success! AST:\n{}", ast.to_sexp());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    // Also test the passing cases to ensure they still work
    let test_cases = vec![
        "sub foo($) { }",
        "sub bar(&) { }",
        "sub baz(@) { }",
        "sub optional($;$) { }",
        "sub slurpy(@) { }",
        "sub ref(\\@) { }",
    ];
    
    for code in test_cases {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✓ Success");
            }
            Err(e) => {
                println!("✗ Error: {}", e);
            }
        }
    }
}