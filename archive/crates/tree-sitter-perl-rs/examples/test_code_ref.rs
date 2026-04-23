//! Test code reference parsing specifically

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Code References ===\n");

    let test_cases = [
        ("Simple function reference", r#"my $ref = \&function;"#),
        ("Qualified function reference", r#"my $ref = \&Module::function;"#),
        ("Reference without &", r#"my $ref = \function;"#),
        ("Reference in expression", r#"my $ref = \&{"function"};"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);
        println!("Code: {}", code);

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                println!("✓ Parsed successfully");
                print_ast(&ast, 0);
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
                // Extract more details from error
                let error_str = format!("{:?}", e);
                println!("Debug: {}", error_str);
            }
        }
        println!();
    }
}

fn print_ast(node: &AstNode, depth: usize) {
    let indent = "  ".repeat(depth);

    match node {
        AstNode::Program(items) => {
            println!("{}Program", indent);
            for item in items {
                print_ast(item, depth + 1);
            }
        }
        AstNode::Statement(content) => {
            println!("{}Statement", indent);
            print_ast(content, depth + 1);
        }
        AstNode::VariableDeclaration { scope, initializer, .. } => {
            println!("{}VarDecl: {}", indent, scope);
            if let Some(init) = initializer {
                print_ast(init, depth + 1);
            }
        }
        _ => {
            println!("{}{:?}", indent, node);
        }
    }
}
