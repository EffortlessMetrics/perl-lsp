//! Test special variable parsing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "$_", "@_", "%_", "$!", "$@", "$$", "$?", "$0", "$1", "$^O", "@ARGV", "%ENV", "$_[0]",
        "@_[1..3]",
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
