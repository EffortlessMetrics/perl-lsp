//! Test use statement parsing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "use strict;",
        "use warnings;",
        "use Data::Dumper;",
        "use Test::More;",
        "use File::Spec::Functions;",
        "use JSON::XS 3.0;",
        // TODO: These require qw() support
        // "use List::Util qw(max min);",
        // "use POSIX qw(strftime ceil floor);",
    ];

    for code in tests {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
