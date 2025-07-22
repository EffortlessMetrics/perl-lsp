//! Simple showcase test
use perl_parser::Parser;

fn main() {
    let code = r#"package MyApp;
use strict;
my $x = 42;
print $x;"#;

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success!");
            println!("S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}