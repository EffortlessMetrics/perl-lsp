use perl_parser::code_actions_enhanced::EnhancedCodeActionsProvider;
use perl_parser::parser::Parser;

fn main() {
    println!("Testing code actions...\n");

    // Test 1: Extract variable
    {
        let source = "my $x = length(\"hello\") + 10;";
        println!("Test 1 - Extract variable:");
        println!("Source: {}", source);

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                let provider = EnhancedCodeActionsProvider::new(source.to_string());
                let actions = provider.get_enhanced_refactoring_actions(&ast, (8, 23));

                println!("Found {} actions:", actions.len());
                for action in &actions {
                    println!("  - {}", action.title);
                }

                if actions.is_empty() {
                    // Debug: print AST
                    println!("AST: {}", ast.to_sexp());
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
        println!();
    }

    // Test 2: Add error checking
    {
        let source = "open my $fh, '<', 'file.txt';";
        println!("Test 2 - Add error checking:");
        println!("Source: {}", source);

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                let provider = EnhancedCodeActionsProvider::new(source.to_string());
                let actions = provider.get_enhanced_refactoring_actions(&ast, (0, 30));

                println!("Found {} actions:", actions.len());
                for action in &actions {
                    println!("  - {}", action.title);
                }

                if actions.is_empty() {
                    println!("AST: {}", ast.to_sexp());
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
        println!();
    }

    // Test 3: Convert to postfix
    {
        let source = r#"if ($debug) { print "Debug\n"; }"#;
        println!("Test 3 - Convert to postfix:");
        println!("Source: {}", source);

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                let provider = EnhancedCodeActionsProvider::new(source.to_string());
                let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

                println!("Found {} actions:", actions.len());
                for action in &actions {
                    println!("  - {}", action.title);
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
        println!();
    }

    // Test 4: Add missing pragmas
    {
        let source = "my $x = 42;\nprint $x;";
        println!("Test 4 - Add missing pragmas:");
        println!("Source: {}", source);

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                let provider = EnhancedCodeActionsProvider::new(source.to_string());
                let actions = provider.get_enhanced_refactoring_actions(&ast, (0, source.len()));

                println!("Found {} actions:", actions.len());
                for action in &actions {
                    println!("  - {}", action.title);
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
    }
}
