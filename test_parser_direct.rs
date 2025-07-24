use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "SUPER::method();",
        "Package::function();",
        "$self->SUPER::method();",
    ];
    
    for code in test_cases {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}