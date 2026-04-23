use perl_parser::Parser;

fn main() {
    let test_cases =
        vec!["{}", "{ key => 'value' }", "{ a => 1, b => 2 }", "{ 'one', 1, 'two', 2 }"];

    for code in test_cases {
        println!("Testing: {}", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("AST: {:?}", ast);
                println!("S-expr: {}\n", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {}\n", e);
            }
        }
    }
}
