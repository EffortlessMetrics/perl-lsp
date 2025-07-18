#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    #[test]
    fn test_format_keyword() {
        // Test just the keyword
        let result = PerlParser::parse(Rule::reserved_word, "format");
        println!("Reserved word 'format': {:?}", result);
    }
    
    #[test] 
    fn test_format_name() {
        let result = PerlParser::parse(Rule::format_name, "STDOUT");
        println!("Format name 'STDOUT': {:?}", result);
        assert!(result.is_ok(), "Failed to parse format name");
    }
    
    #[test]
    fn test_format_lines() {
        let input = "test line\n";
        let result = PerlParser::parse(Rule::format_lines, input);
        println!("Format lines: {:?}", result);
        assert!(result.is_ok(), "Failed to parse format lines");
    }
    
    #[test]
    fn test_format_end() {
        let result = PerlParser::parse(Rule::format_end, ".\n");
        println!("Format end: {:?}", result);
        assert!(result.is_ok(), "Failed to parse format end");
    }

    #[test]
    fn test_format_declaration() {
        let input = r#"format STDOUT =
test line
.
"#;
        
        // Test individual components first
        println!("Testing components:");
        println!("  format keyword: {:?}", PerlParser::parse(Rule::reserved_word, "format"));
        println!("  format_name: {:?}", PerlParser::parse(Rule::format_name, "STDOUT"));
        println!("  format_end: {:?}", PerlParser::parse(Rule::format_end, ".\n"));
        
        // Try parsing each line separately
        println!("\nTrying first line:");
        let first_line = "format STDOUT =\n";
        println!("  Input: {:?}", first_line);
        
        // Try parsing statement
        println!("\nTrying as statement:");
        let stmt_result = PerlParser::parse(Rule::statement, input);
        println!("  Statement result: {:?}", stmt_result);
        
        let pairs = PerlParser::parse(Rule::format_declaration, input);
        assert!(pairs.is_ok(), "Failed to parse format declaration: {:?}", pairs.err());
        
        let pairs = pairs.unwrap();
        for pair in pairs {
            println!("Rule: {:?}, Text: {}", pair.as_rule(), pair.as_str());
            for inner in pair.into_inner() {
                println!("  Inner - Rule: {:?}, Text: {}", inner.as_rule(), inner.as_str());
            }
        }
    }
    
    #[test]
    fn test_format_in_program() {
        let input = r#"format STDOUT =
test line
.
"#;
        
        let pairs = PerlParser::parse(Rule::program, input);
        assert!(pairs.is_ok(), "Failed to parse program with format: {:?}", pairs.err());
    }
}