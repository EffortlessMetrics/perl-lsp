//! Test the enhanced parser

use tree_sitter_perl::enhanced_parser::EnhancedPerlParser;

fn main() {
    let source = std::env::args()
        .nth(1)
        .map(|f| std::fs::read_to_string(f).expect("Failed to read file"))
        .unwrap_or_else(|| "my $x = 42;".to_string());
    
    println!("=== Source ===");
    println!("{}", source);
    println!();
    
    let parser = EnhancedPerlParser::new();
    match parser.parse(&source) {
        Ok(ast) => {
            println!("=== AST ===");
            println!("{:#?}", ast);
            println!();
            
            println!("=== S-expression ===");
            let sexp = parser.to_sexp(&ast);
            println!("{}", sexp);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}