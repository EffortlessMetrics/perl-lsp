//! Test dereferencing edge cases

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Dereferencing ===\n");

    let test_cases = [
        ("Simple scalar deref", r#"my $value = $$ref;"#),
        ("Scalar deref with braces", r#"my $value = ${$scalar_ref};"#),
        ("Array deref", r#"my @arr = @{$array_ref};"#),
        ("Hash deref", r#"my %h = %{$hash_ref};"#),
        ("Code deref call", r#"$code_ref->();"#),
        ("Code deref with &", r#"&{$code_ref}();"#),
        ("Complex deref chain", r#"my $val = $hash->{key}->[0]->{nested};"#),
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
                let error_str = format!("{:?}", e);
                if error_str.contains("positives")
                    && let Some(start) = error_str.find("positives: [")
                    && let Some(end) = error_str.find("], negatives")
                {
                    let expected = &error_str[start + 12..end];
                    println!("Expected: {}", expected);
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
