use perl_parser::Parser;

fn main() {
    let source = "print FOO;";
    let mut parser = Parser::new(source);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("AST for 'print FOO;':");
            println!("{:#?}", ast);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
