//! Demonstration of the token-based parser

#[cfg(test)]
mod tests {
    use crate::simple_parser_v2::SimpleParser;

    #[test]
    fn demo_basic_parsing() {
        let input = "my $x = 42;";
        let mut parser = SimpleParser::new(input);

        match parser.parse() {
            Ok(ast) => {
                println!("Successfully parsed: {}", input);
                println!("AST: {:#?}", ast);
                assert_eq!(ast.node_type, "program");
                assert_eq!(ast.children.len(), 1);
                assert_eq!(ast.children[0].node_type, "my_declaration");
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    #[test]
    fn demo_expression_parsing() {
        let input = "$a + $b * $c;";
        let mut parser = SimpleParser::new(input);

        match parser.parse() {
            Ok(ast) => {
                println!("Successfully parsed: {}", input);
                println!("AST: {:#?}", ast);

                // Verify correct precedence: + at top, * below
                assert_eq!(ast.children[0].node_type, "binary_expression");
                assert_eq!(ast.children[0].value.as_ref().map(|s| s.as_ref()), Some("Plus"));
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    #[test]
    fn demo_slash_disambiguation() {
        // Test division
        let input1 = "my $x = 10 / 2;";
        let mut parser1 = SimpleParser::new(input1);

        match parser1.parse() {
            Ok(ast) => {
                println!("Division example parsed: {}", input1);
                let expr = &ast.children[0].children[1]; // The expression in the declaration
                assert_eq!(expr.node_type, "binary_expression");
                assert_eq!(expr.value.as_ref().map(|s| s.as_ref()), Some("Divide"));
            }
            Err(e) => panic!("Parse error: {}", e),
        }

        // Test regex (in a conditional context)
        let input2 = "if ($str =~ /test/) { print; }";
        let mut parser2 = SimpleParser::new(input2);

        match parser2.parse() {
            Ok(ast) => {
                println!("Regex example parsed: {}", input2);
                assert_eq!(ast.children[0].node_type, "if_statement");
                let condition = &ast.children[0].children[0];
                assert_eq!(condition.node_type, "regex_match");
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    #[test]
    fn demo_complex_expression() {
        let input = "my $result = ($a + $b) * $c - $d / 2;";
        let mut parser = SimpleParser::new(input);

        match parser.parse() {
            Ok(ast) => {
                println!("Complex expression parsed: {}", input);
                println!("AST structure:");
                print_ast(&ast, 0);
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    #[test]
    fn demo_control_flow() {
        let input = r#"
if ($x > 0) {
    print "positive";
} elsif ($x < 0) {
    print "negative";
} else {
    print "zero";
}
"#;
        let mut parser = SimpleParser::new(input);

        match parser.parse() {
            Ok(ast) => {
                println!("Control flow parsed successfully");
                println!("Number of statements: {}", ast.children.len());
                assert_eq!(ast.children[0].node_type, "if_statement");
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    #[test]
    fn demo_subroutine() {
        let input = r#"
sub hello {
    my $name = shift;
    return "Hello, $name!";
}
"#;
        let mut parser = SimpleParser::new(input);

        match parser.parse() {
            Ok(ast) => {
                println!("Subroutine parsed successfully");
                assert_eq!(ast.children[0].node_type, "subroutine");
                assert_eq!(ast.children[0].value.as_ref().map(|s| s.as_ref()), Some("Identifier"));
            }
            Err(e) => panic!("Parse error: {}", e),
        }
    }

    fn print_ast(node: &crate::token_ast::AstNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        print!("{}{}", prefix, node.node_type);
        if let Some(ref value) = node.value {
            print!(" [{}]", value);
        }
        println!();

        for child in &node.children {
            print_ast(child, indent + 1);
        }
    }
}
