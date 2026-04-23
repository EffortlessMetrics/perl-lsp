use perl_parser::Parser;

fn main() {
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
    }

    let code = "{ key => 'value' }";
    println!("Parsing: {}", code);

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Success: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
