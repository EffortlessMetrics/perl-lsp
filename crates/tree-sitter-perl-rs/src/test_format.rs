#[cfg(test)]
mod tests {
    use crate::pure_rust_parser::{PerlParser, Rule};
    use pest::Parser;

    #[test]
    fn test_format_keyword() {
        // Test just the keyword
        let result = PerlParser::parse(Rule::reserved_word, "format");
        println!("Reserved word 'format': {:?}", result);

        // Test format as a literal in a simple rule
        // Create a test rule that just matches "format"
        println!("\nDirect parsing tests:");

        // Test if it's being parsed as "for"
        let for_result = PerlParser::parse(Rule::for_statement, "format STDOUT =");
        println!("As for_statement: {:?}", for_result.err());
    }

    #[test]
    fn test_format_parsing_debug() {
        // First test if reserved_word works
        println!("Testing reserved_word for 'format':");
        match PerlParser::parse(Rule::reserved_word, "format") {
            Ok(pairs) => println!("  SUCCESS: {:?}", pairs.collect::<Vec<_>>()),
            Err(e) => println!("  FAILED: {:?}", e),
        }

        // Test minimal format declaration
        println!("\nTesting minimal format declaration:");
        let minimal = "format\n.\n";
        match PerlParser::parse(Rule::format_declaration, minimal) {
            Ok(pairs) => println!("  SUCCESS: {:?}", pairs.collect::<Vec<_>>()),
            Err(e) => println!("  FAILED: {:?}", e),
        }

        // Test format with space
        println!("\nTesting 'format ' as format_declaration start:");
        let with_space = "format \n.\n";
        match PerlParser::parse(Rule::format_declaration, with_space) {
            Ok(pairs) => println!("  SUCCESS: {:?}", pairs.collect::<Vec<_>>()),
            Err(e) => println!("  FAILED: {:?}", e),
        }

        // Test with equals
        println!("\nTesting with equals:");
        let with_equals = "format =\n.\n";
        match PerlParser::parse(Rule::format_declaration, with_equals) {
            Ok(pairs) => println!("  SUCCESS: {:?}", pairs.collect::<Vec<_>>()),
            Err(e) => println!("  FAILED: {:?}", e),
        }

        // Test the actual format declaration
        let format_decl = "format STDOUT =\ntest\n.\n";
        println!("\nTesting complete format declaration:");
        match PerlParser::parse(Rule::format_declaration, format_decl) {
            Ok(pairs) => {
                println!("SUCCESS! Parsed as format_declaration");
                for pair in pairs {
                    println!("  {:?}", pair);
                }
            }
            Err(e) => {
                println!("FAILED to parse as format_declaration");
                println!("  Error: {:?}", e);
            }
        }
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

        // Try parsing just the first word
        println!("\nTrying to parse 'format' as identifier:");
        let id_result = PerlParser::parse(Rule::identifier, "format");
        println!("  Identifier result: {:?}", id_result);

        // Try parsing format_declaration directly
        println!("\nTrying format_declaration directly:");
        let fd_result = PerlParser::parse(Rule::format_declaration, input);
        println!("  Format declaration result: {:?}", fd_result);

        // Try parsing just the first part
        println!("\nTrying to parse just 'format STDOUT =':");
        let partial = "format STDOUT =\n";
        let partial_result = PerlParser::parse(Rule::format_declaration, partial);
        println!("  Partial result: {:?}", partial_result);

        let pairs_res = PerlParser::parse(Rule::format_declaration, input);
        assert!(pairs_res.is_ok(), "Failed to parse format declaration: {:?}", pairs_res.err());

        use perl_tdd_support::must;
        let pairs = must(pairs_res);
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

        println!("Testing format in program context:");
        match PerlParser::parse(Rule::program, input) {
            Ok(pairs) => {
                println!("SUCCESS parsing as program!");
                for pair in pairs {
                    println!("Top level: {:?}", pair.as_rule());
                    for inner in pair.into_inner() {
                        println!("  Statement: {:?}", inner.as_rule());
                        if inner.as_rule() == Rule::statements {
                            for stmt in inner.into_inner() {
                                println!("    Statement type: {:?}", stmt.as_rule());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("FAILED: {:?}", e);
            }
        }
    }
}
