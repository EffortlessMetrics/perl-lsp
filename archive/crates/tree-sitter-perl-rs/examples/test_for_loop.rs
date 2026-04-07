//! Test for loop parsing

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing For Loops ===\n");

    let test_cases = [
        (
            "Simple for with range",
            r#"
for my $i (0..10) {
    print $i;
}
"#,
        ),
        (
            "For without my",
            r#"
for $i (0..10) {
    print $i;
}
"#,
        ),
        (
            "For with list",
            r#"
for my $item (@list) {
    print $item;
}
"#,
        ),
        (
            "For with expression",
            r#"
for my $x (1, 2, 3, 4, 5) {
    print $x;
}
"#,
        ),
        (
            "C-style for",
            r#"
for (my $i = 0; $i < 10; $i++) {
    print $i;
}
"#,
        ),
        (
            "For without variable",
            r#"
for (1..10) {
    print $_;
}
"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);
        println!("Code: {}", code.trim());

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                println!("✓ Parsed successfully");
                print_ast(&ast, 0);
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
                // Extract positives from error
                let error_str = format!("{:?}", e);
                if let Some(start) = error_str.find("positives: [")
                    && let Some(end) = error_str.find("], negatives")
                {
                    let expected = &error_str[start + 12..end];
                    println!("Expected tokens: {}", expected);
                }
                if let Some(loc) = error_str.find("line_col: Pos(")
                    && let Some(end) = error_str[loc..].find(")")
                {
                    let pos = &error_str[loc + 14..loc + end];
                    println!("Error position: {}", pos);
                }
            }
        }
        println!();
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
        AstNode::Statement(content) => {
            println!("{}Statement", indent);
            print_ast(content, depth + 1);
        }
        _ => {
            println!("{}{:?}", indent, node);
        }
    }
}
