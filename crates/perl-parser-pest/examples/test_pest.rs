//! Test the Pest parser directly

use tree_sitter_perl::PureRustParser;

fn main() {
    println!("Testing Pest parser...");

    let code = "my $x = 42; print $x;";
    #[allow(unused_mut)]
    let mut parser = PureRustParser::new();

    match parser.parse(code) {
        Ok(ast) => {
            println!("✅ Success! AST: {:?}", ast);
        }
        Err(e) => {
            println!("❌ Failed: {:?}", e);
        }
    }
}
