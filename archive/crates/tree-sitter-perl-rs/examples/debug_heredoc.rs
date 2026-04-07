//! Debug heredoc parsing to understand AST structure

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Heredoc Debugging ===\n");

    let code = r#"
my $text = <<EOF;
This is a heredoc
with multiple lines
EOF
print $text;
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(code) {
        Ok(ast) => {
            println!("Parse successful!");
            println!("\nFull AST structure:");
            print_full_ast(&ast, 0);

            println!("\n\nSearching for heredoc content...");
            find_string_content(&ast, 0);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

fn print_full_ast(node: &AstNode, depth: usize) {
    let indent = "  ".repeat(depth);

    match node {
        AstNode::Program(items) => {
            println!("{}Program ({} items)", indent, items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}[{}]:", indent, i);
                print_full_ast(item, depth + 1);
            }
        }
        AstNode::Statement(content) => {
            println!("{}Statement", indent);
            print_full_ast(content, depth + 1);
        }
        AstNode::VariableDeclaration { scope, variables, initializer } => {
            println!("{}VariableDeclaration", indent);
            println!("{}  scope: {}", indent, scope);
            println!("{}  variables: {} items", indent, variables.len());
            if let Some(init) = initializer {
                println!("{}  initializer:", indent);
                print_full_ast(init, depth + 2);
            }
        }
        AstNode::String(s) => {
            println!("{}String: {:?}", indent, s);
            if s.contains('\n') {
                println!("{}  (multiline string detected!)", indent);
            }
        }
        AstNode::Identifier(name) => {
            println!("{}Identifier: {}", indent, name);
        }
        AstNode::FunctionCall { function, args } => {
            println!("{}FunctionCall", indent);
            println!("{}  function:", indent);
            print_full_ast(function, depth + 2);
            println!("{}  args: {} items", indent, args.len());
            for arg in args {
                print_full_ast(arg, depth + 2);
            }
        }
        AstNode::Heredoc { marker, indented, quoted, content } => {
            println!("{}Heredoc", indent);
            println!("{}  marker: {}", indent, marker);
            println!("{}  indented: {}", indent, indented);
            println!("{}  quoted: {}", indent, quoted);
            println!("{}  content: {:?}", indent, content);
        }
        _ => {
            println!("{}{:?}", indent, node);
        }
    }
}

fn find_string_content(node: &AstNode, depth: usize) {
    let indent = "  ".repeat(depth);

    match node {
        AstNode::String(s) => {
            if s.contains('\n') {
                println!("{}Found multiline string:", indent);
                println!("{}Content: {:?}", indent, s);
                println!("{}Length: {} chars", indent, s.len());
                println!("{}Lines: {}", indent, s.lines().count());
            }
        }
        AstNode::Program(items) => {
            for item in items {
                find_string_content(item, depth);
            }
        }
        AstNode::Statement(content) => {
            find_string_content(content, depth);
        }
        AstNode::VariableDeclaration { initializer: Some(init), .. } => {
            find_string_content(init, depth);
        }
        AstNode::VariableDeclaration { .. } => {}
        AstNode::FunctionCall { args, .. } => {
            for arg in args {
                find_string_content(arg, depth);
            }
        }
        _ => {}
    }
}
