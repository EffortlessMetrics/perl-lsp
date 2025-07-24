use perl_parser::Parser;

fn main() {
    let code = r#"format STDOUT =
@<<<<<<< @<<<<<<< @<<<<<<<<<<<<<<
$first,  $second, $third
.
"#;

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => println!("Success! S-exp: {}", ast.to_sexp()),
        Err(e) => println!("Error: {}", e),
    }
}