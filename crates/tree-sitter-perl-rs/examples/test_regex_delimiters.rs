//! Test regex with various delimiters
use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Regex with Various Delimiters ===\n");

    let test_cases = [
        ("Standard slash", r#"$text =~ /pattern/;"#),
        ("Match with m//", r#"$text =~ m/pattern/;"#),
        ("Match with m!!", r#"$text =~ m!pattern!;"#),
        ("Match with m{}", r#"$text =~ m{pattern};"#),
        ("Match with m[]", r#"$text =~ m[pattern];"#),
        ("Match with m<>", r#"$text =~ m<pattern>;"#),
        ("Match with m||", r#"$text =~ m|pattern|;"#),
        ("Substitution s///", r#"$text =~ s/old/new/;"#),
        ("Substitution s!!!", r#"$text =~ s!old!new!;"#),
        ("Substitution s{}{}", r#"$text =~ s{old}{new};"#),
        ("Translation tr///", r#"$text =~ tr/a-z/A-Z/;"#),
        ("Translation tr!!!", r#"$text =~ tr!a-z!A-Z!;"#),
        ("Regex with modifiers", r#"$text =~ m/pattern/gims;"#),
        ("Regex with interpolation", r#"$text =~ /$pattern/;"#),
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
