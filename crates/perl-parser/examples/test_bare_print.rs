//! Test bare print/say
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "print",
        "say",
        "print;",
        "say;",
        "{ print }",
        "{ say }",
        "print 'hello'",
        "say 'hello'",
    ];

    for test in tests {
        print!("Testing: {:30} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}