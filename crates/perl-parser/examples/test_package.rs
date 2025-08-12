//! Test package parsing
use perl_parser::Parser;

fn main() {
    let code = r#"
package Test;
use strict;
sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}
1;"#;

    println!("Testing package code:");
    println!("{}", code);
    println!();

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success!");
            println!("S-expression: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}
