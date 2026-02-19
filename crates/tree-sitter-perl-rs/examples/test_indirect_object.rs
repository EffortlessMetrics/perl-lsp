//! Test indirect object syntax
use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Indirect Object Syntax ===\n");

    let test_cases = [
        ("Simple new", r#"my $obj = new Class;"#),
        ("Qualified new", r#"my $obj = new Class::Name;"#),
        ("New with args", r#"my $obj = new Class::Name($arg1, $arg2);"#),
        ("Method on object", r#"my $result = method $obj;"#),
        ("Method with args", r#"my $result = method $obj @args;"#),
        ("Print to filehandle", r#"print FH "data";"#),
        ("Open file", r#"open FH, '<', 'file.txt';"#),
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
                println!("Enhanced parser error: {:?}", e);
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
