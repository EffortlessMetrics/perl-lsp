use perl_parser::Parser;

fn main() {
    let input = std::fs::read_to_string("test_indirect_call.pl").expect("Failed to read file");
    
    let mut parser = Parser::new(&input);
    match parser.parse() {
        Ok(ast) => {
            println!("Parsed successfully!");
            println!("S-expression:\n{}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}