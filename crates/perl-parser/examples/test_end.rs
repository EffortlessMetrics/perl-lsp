use perl_parser::Parser;
use std::fs;

fn main() {
    let input = fs::read_to_string("/tmp/test_end.pl").unwrap();

    let mut parser = Parser::new(&input);
    match parser.parse() {
        Ok(ast) => {
            println!("{}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
