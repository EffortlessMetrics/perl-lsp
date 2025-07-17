//! Test the Pest parser directly

use pest::Parser;
use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};

fn main() {
    let source = std::env::args()
        .nth(1)
        .map(|f| std::fs::read_to_string(f).expect("Failed to read file"))
        .unwrap_or_else(|| "my $x = 42;".to_string());
    
    println!("=== Source ===");
    println!("{}", source);
    println!();
    
    match PerlParser::parse(Rule::program, &source) {
        Ok(pairs) => {
            println!("=== Parse Tree ===");
            for pair in pairs {
                print_pair(&pair, 0);
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

fn print_pair(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}{:?} -> {:?}", indent_str, pair.as_rule(), pair.as_str());
    for inner in pair.clone().into_inner() {
        print_pair(&inner, indent + 1);
    }
}