use perl_parser::Parser;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = fs::read_to_string("test_parser_improvements.pl")?;

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

    Ok(())
}