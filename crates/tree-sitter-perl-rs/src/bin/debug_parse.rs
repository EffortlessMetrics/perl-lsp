#[cfg(feature = "pure-rust")]
use pest::Parser;
#[cfg(feature = "pure-rust")]
use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};

fn main() {
    #[cfg(feature = "pure-rust")]
    {
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
    
    #[cfg(not(feature = "pure-rust"))]
    {
        println!("Debug parser requires pure-rust feature");
    }
}

#[cfg(feature = "pure-rust")]
fn print_pair(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}{:?}: '{}'", indent_str, pair.as_rule(), pair.as_str());
    
    for inner_pair in pair.clone().into_inner() {
        print_pair(&inner_pair, indent + 1);
    }
}