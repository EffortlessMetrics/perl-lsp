#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    #[test]
    fn test_format_order() {
        // Test if format_declaration comes before expression_statement
        let input = "format STDOUT =\ntest\n.\n";
        
        // Try parsing as each statement type in order
        println!("Testing statement alternatives in order:");
        
        // Try as comment
        match PerlParser::parse(Rule::comment, input) {
            Ok(_) => println!("  ✓ Parsed as comment"),
            Err(_) => println!("  ✗ Not a comment"),
        }
        
        // Try as sub_declaration
        match PerlParser::parse(Rule::sub_declaration, input) {
            Ok(_) => println!("  ✓ Parsed as sub_declaration"),
            Err(_) => println!("  ✗ Not a sub_declaration"),
        }
        
        // Try as format_declaration
        match PerlParser::parse(Rule::format_declaration, input) {
            Ok(_) => println!("  ✓ Parsed as format_declaration"),
            Err(e) => println!("  ✗ Not a format_declaration: {:?}", e),
        }
        
        // Try as expression_statement
        match PerlParser::parse(Rule::expression_statement, input) {
            Ok(_) => println!("  ✓ Parsed as expression_statement"),
            Err(e) => println!("  ✗ Not an expression_statement: {:?}", e),
        }
        
        // Try parsing the first token as an expression
        println!("\nTrying to parse 'format' as expression components:");
        match PerlParser::parse(Rule::expression, "format") {
            Ok(_) => println!("  ✓ 'format' parsed as expression"),
            Err(e) => println!("  ✗ 'format' not an expression: {:?}", e),
        }
        
        match PerlParser::parse(Rule::primary_expression, "format") {
            Ok(_) => println!("  ✓ 'format' parsed as primary_expression"),
            Err(e) => println!("  ✗ 'format' not a primary_expression: {:?}", e),
        }
        
        match PerlParser::parse(Rule::identifier, "format") {
            Ok(_) => println!("  ✓ 'format' parsed as identifier"),
            Err(e) => println!("  ✗ 'format' not an identifier: {:?}", e),
        }
        
        match PerlParser::parse(Rule::qualified_name_or_identifier, "format") {
            Ok(_) => println!("  ✓ 'format' parsed as qualified_name_or_identifier"),
            Err(e) => println!("  ✗ 'format' not a qualified_name_or_identifier: {:?}", e),
        }
    }
}