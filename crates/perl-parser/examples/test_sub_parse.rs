//! Test how sub is being parsed
use perl_parser::{Parser, ast::NodeKind};

fn main() {
    let tests = vec!["sub", "sub {", "sub { }", "sub { 42 }"];

    for test in tests {
        println!("\n=== Input: {} ===", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("Success! S-expr: {}", ast.to_sexp());
                // Print the AST structure
                if let NodeKind::Program { statements } = &ast.kind {
                    if let Some(first) = statements.first() {
                        println!("First statement kind: {:?}", first.kind);
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
