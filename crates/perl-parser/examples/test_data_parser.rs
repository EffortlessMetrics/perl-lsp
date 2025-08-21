use perl_parser::Parser;

fn main() {
    let input = "print \"hello\\n\";\n__DATA__\nnot perl code";

    println!("Parsing: {:?}", input);
    println!();

    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => {
            println!("AST S-expression:");
            println!("{}", ast.to_sexp());
            println!();
            println!("AST Debug:");
            println!("{:#?}", ast);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
