//! Test the stateful parser

use tree_sitter_perl::stateful_parser::StatefulPerlParser;
use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

fn main() {
    let source = if let Some(file) = std::env::args().nth(1) {
        std::fs::read_to_string(file).expect("Failed to read file")
    } else {
        r#"my $text = <<EOF;
Hello
World
EOF
print $text;"#.to_string()
    };

    println!("=== Original Source ===");
    println!("{}", source);
    println!();

    let mut parser = StatefulPerlParser::new();
    match parser.parse(&source) {
        Ok(ast) => {
            println!("=== AST ===");
            println!("{:#?}", ast);
            println!();
            
            println!("=== S-expression ===");
            let sexp = PureRustPerlParser::new().to_sexp(&ast);
            println!("{}", sexp);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}