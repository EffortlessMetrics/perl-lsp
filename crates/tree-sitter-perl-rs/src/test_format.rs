#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    #[test]
    fn test_format_declaration() {
        let input = r#"format STDOUT =
test line
.
"#;
        
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