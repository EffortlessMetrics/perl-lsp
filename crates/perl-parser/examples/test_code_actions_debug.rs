use perl_parser::ast::{Node, NodeKind};
use perl_parser::code_actions_enhanced::EnhancedCodeActionsProvider;
use perl_parser::parser::Parser;

fn print_ast_with_ranges(node: &Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = &source[node.location.start..node.location.end.min(source.len())];
    let preview = if text.len() > 40 {
        format!("{}...", &text[..37])
    } else {
        text.to_string()
    };

    println!(
        "{}[{}-{}] {:?}: \"{}\"",
        indent,
        node.location.start,
        node.location.end,
        std::any::type_name_of_val(&node.kind)
            .split("::")
            .last()
            .unwrap_or("Unknown"),
        preview.replace('\n', "\\n")
    );

    // Print children
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                print_ast_with_ranges(stmt, source, depth + 1);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            print_ast_with_ranges(left, source, depth + 1);
            print_ast_with_ranges(right, source, depth + 1);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                print_ast_with_ranges(arg, source, depth + 1);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            print_ast_with_ranges(lhs, source, depth + 1);
            print_ast_with_ranges(rhs, source, depth + 1);
        }
        _ => {}
    }
}

fn main() {
    println!("Testing code actions with debug info...\n");

    // Test 1: Extract variable - find the right range for length("hello")
    {
        let source = "my $x = length(\"hello\") + 10;";
        println!("Test 1 - Extract variable:");
        println!("Source: {}", source);
        println!("\nAST with ranges:");

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                print_ast_with_ranges(&ast, source, 0);

                // Try different ranges to find what triggers extract variable
                let test_ranges = [
                    (8, 23), // length("hello")
                    (8, 28), // length("hello") + 10
                    (0, 29), // entire statement
                    (8, 24), // length("hello") with )
                ];

                for (start, end) in test_ranges {
                    println!(
                        "\nTrying range [{}-{}]: \"{}\"",
                        start,
                        end,
                        &source[start..end]
                    );
                    let provider = EnhancedCodeActionsProvider::new(source.to_string());
                    let actions = provider.get_enhanced_refactoring_actions(&ast, (start, end));

                    for action in &actions {
                        println!("  ✓ {}", action.title);
                    }
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
        println!("{}", "=".repeat(60));
    }

    // Test 2: Add error checking - find the right node for open
    {
        let source = "open my $fh, '<', 'file.txt';";
        println!("Test 2 - Add error checking:");
        println!("Source: {}", source);
        println!("\nAST with ranges:");

        let mut parser = Parser::new(source);
        match parser.parse() {
            Ok(ast) => {
                print_ast_with_ranges(&ast, source, 0);

                // Try different ranges
                let test_ranges = [
                    (0, 4),  // "open"
                    (0, 29), // entire statement
                    (0, 30), // entire statement with ;
                    (5, 11), // "my $fh"
                ];

                for (start, end) in test_ranges {
                    println!(
                        "\nTrying range [{}-{}]: \"{}\"",
                        start,
                        end,
                        &source[start..end.min(source.len())]
                    );
                    let provider = EnhancedCodeActionsProvider::new(source.to_string());
                    let actions = provider.get_enhanced_refactoring_actions(&ast, (start, end));

                    for action in &actions {
                        println!("  ✓ {}", action.title);
                    }
                }
            }
            Err(e) => println!("Parse error: {:?}", e),
        }
    }
}
