//! Test multiline parsing
use perl_parser::Parser;

fn main() {
    let test = r#"package MyModule;
use strict;
use warnings;"#;

    println!("Testing multiline code:");
    let mut parser = Parser::new(test);
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