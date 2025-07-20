#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    #[test]
    fn test_statement_debug() {
        let input = "format STDOUT =\ntest\n.\n";
        
        // First test if it's being parsed as "for" + "mat"
        println!("Testing if 'format' is being mistaken for 'for':");
        match PerlParser::parse(Rule::for_statement, input) {
            Ok(_) => println!("  ✓ Parsed as for_statement (unexpected!)"),
            Err(e) => println!("  ✗ Not a for_statement: {:?}", e),
        }
        
        // Now test the full statement
        println!("\nTesting full statement parse:");
        match PerlParser::parse(Rule::statement, input) {
            Ok(pairs) => {
                println!("  ✓ SUCCESS! Parsed as statement");
                for pair in pairs {
                    println!("    Rule: {:?}", pair.as_rule());
                }
            }
            Err(e) => {
                println!("  ✗ Failed to parse as statement");
                println!("    Error: {:?}", e);
                println!("    Location: {:?}", e.location);
                println!("    Line/col: {:?}", e.line_col);
            }
        }
        
        // Test parsing just "format" as reserved word
        println!("\nTesting 'format' as reserved word:");
        match PerlParser::parse(Rule::reserved_word, "format") {
            Ok(pairs) => {
                println!("  ✓ 'format' is a reserved word");
                for pair in pairs {
                    println!("    Matched: '{}'", pair.as_str());
                }
            }
            Err(e) => println!("  ✗ Failed: {:?}", e),
        }
        
        // Test if "for" is matching instead
        println!("\nTesting if 'for' matches in 'format':");
        match PerlParser::parse(Rule::for_statement, "for") {
            Ok(_) => println!("  ✓ 'for' alone parses as for_statement"),
            Err(e) => println!("  ✗ 'for' alone doesn't parse: {:?}", e),
        }
    }
}