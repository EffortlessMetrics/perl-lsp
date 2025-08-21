use perl_parser::Parser;
use std::fs;

fn main() {
    let input = fs::read_to_string("/tmp/test_corpus_simple.pl").unwrap();

    println!("Parsing corpus file...");
    println!();

    let mut parser = Parser::new(&input);
    match parser.parse() {
        Ok(ast) => {
            println!("Success! AST S-expression:");
            println!("{}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
