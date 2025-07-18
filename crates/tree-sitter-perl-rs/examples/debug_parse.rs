#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{PerlParser, pure_rust_parser::Rule};
#[cfg(feature = "pure-rust")]
use pest::Parser;

#[cfg(feature = "pure-rust")]
fn main() {
    let input = r#""Hello, $name!""#;
    
    match PerlParser::parse(Rule::double_quoted_string, input) {
        Ok(pairs) => {
            for pair in pairs {
                debug_print_pair(&pair, 0);
            }
        }
        Err(e) => eprintln!("Parse error: {:?}", e),
    }
}

#[cfg(feature = "pure-rust")]
fn debug_print_pair(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}Rule: {:?}, Text: {:?}", indent_str, pair.as_rule(), pair.as_str());
    for inner in pair.clone().into_inner() {
        debug_print_pair(&inner, indent + 1);
    }
}

#[cfg(not(feature = "pure-rust"))]
fn main() {
    eprintln!("This example requires the 'pure-rust' feature");
}