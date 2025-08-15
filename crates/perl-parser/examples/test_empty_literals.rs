//! Test empty array and hash literals
use perl_parser::Parser;

fn main() {
    let tests = vec!["[]", "{}", "[1]", "{a => 1}", "my @empty = []", "my %empty = {}"];

    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
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
