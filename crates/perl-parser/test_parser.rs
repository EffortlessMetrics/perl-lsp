use perl_parser::Parser;
use std::fs;

fn main() {
    let code = fs::read_to_string("test_parser_improvements.pl")
        .expect("Failed to read test file");
    
    let mut parser = Parser::new(&code);
    match parser.parse() {
        Ok(ast) => {
            println!("Parse successful!");
            println!("S-expression output:");
            println!("{}", ast.to_sexp());
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
        }
    }
}