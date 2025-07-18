use pest::Parser;
use tree_sitter_perl::pure_rust_parser::{PerlParser, Rule};

fn main() {
    let input = "format STDOUT =\ntest\n.\n";
    
    // Try parsing format_declaration directly
    println!("Parsing format_declaration:");
    match PerlParser::parse(Rule::format_declaration, input) {
        Ok(pairs) => {
            for pair in pairs {
                println!("Success: {:?}", pair);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            println!("Details: {:?}", e);
        }
    }
    
    // Try parsing as a statement
    println!("\nParsing as statement:");
    match PerlParser::parse(Rule::statement, input) {
        Ok(pairs) => {
            for pair in pairs {
                println!("Success: {:?}", pair);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            println!("Details: {:?}", e);
        }
    }
    
    // Try each part separately
    println!("\nTesting parts:");
    println!("'format' matches: {:?}", PerlParser::parse(Rule::reserved_word, "format").is_ok());
    
    // Check if the literal "format" can be parsed
    println!("\nChecking literal parsing:");
    let format_only = "format";
    for rule in [Rule::identifier, Rule::reserved_word] {
        match PerlParser::parse(rule, format_only) {
            Ok(_) => println!("{:?} matches 'format'", rule),
            Err(_) => println!("{:?} does NOT match 'format'", rule),
        }
    }
}