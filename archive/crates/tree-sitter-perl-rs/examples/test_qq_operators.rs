//! Test qq and q operators
use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing qq and q Operators ===\n");

    let test_cases = [
        ("Simple q string", r#"my $str = q(Hello World);"#),
        ("Simple qq string", r#"my $str = qq(Hello World);"#),
        ("q with brackets", r#"my $str = q[Hello World];"#),
        ("qq with braces", r#"my $str = qq{Hello World};"#),
        ("qq with angles", r#"my $str = qq<Hello World>;"#),
        ("qq with custom delimiter", r#"my $str = qq|Hello World|;"#),
        ("qq with interpolation", r#"my $str = qq|Path: $ENV{PATH}|;"#),
        ("Nested delimiters", r#"my $str = q(Hello (nested) World);"#),
        ("q with escaped delimiter", r#"my $str = q(Hello \) World);"#),
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
