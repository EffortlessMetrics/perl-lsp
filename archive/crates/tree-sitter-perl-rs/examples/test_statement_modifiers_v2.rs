//! Test statement modifiers
use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Statement Modifiers ===\n");

    let test_cases = [
        ("Simple if modifier", "print 'Hello' if $condition;"),
        ("Simple unless modifier", "die 'Error' unless $valid;"),
        ("Simple while modifier", "next while $iterator->has_next;"),
        ("Simple until modifier", "sleep 1 until $ready;"),
        ("For modifier", "$count++ for @items;"),
        ("Complex expression", "$hash{$key} = $value if defined $value && $value ne '';"),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);
        println!("Code: {}", code);

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                println!("✓ Parsed successfully");
                print_ast(&ast, 0);
                println!();
            }
            Err(e) => {
                println!("✗ Failed to parse: {}", e);
                println!();
            }
        }
    }
}

fn print_ast(node: &AstNode, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        AstNode::Program(items) => {
            println!("{}Program ({} items)", indent, items.len());
            for item in items {
                print_ast(item, depth + 1);
            }
        }
        _ => {
            println!("{}{:?}", indent, node);
        }
    }
}
