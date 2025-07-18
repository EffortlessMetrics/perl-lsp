use pest::Parser;
use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};

fn main() {
    let input = "5 + 3;";
    println!("Parsing: {}", input);
    
    match PerlParser::parse(Rule::program, input) {
        Ok(pairs) => {
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
    println!("{}{:?}: '{}'", indent_str, pair.as_rule(), pair.as_str());
    
    for inner_pair in pair.clone().into_inner() {
        print_pair(&inner_pair, indent + 1);
    }
}