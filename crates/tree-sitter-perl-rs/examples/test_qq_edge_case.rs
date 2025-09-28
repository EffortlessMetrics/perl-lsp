//! Test specific qq edge case
use tree_sitter_perl::{pure_rust_parser::AstNode, EnhancedFullParser};

fn main() {
    let code = r#"
my $interpolated = qq{Hello $name};
my $literal = q{Hello $name};
my $custom = qq|Path: $ENV{PATH}|;
"#;

    println!("Testing qq and q operators edge case:");
    println!("Code: {}", code);

    let mut parser = EnhancedFullParser::new();
    match parser.parse(code) {
        Ok(ast) => {
            println!("✓ Parsed successfully");
            print_ast(&ast, 0);
        }
        Err(e) => {
            println!("✗ Failed to parse: {}", e);
            println!("Enhanced parser error: {:?}", e);
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
