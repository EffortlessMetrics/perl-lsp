//! Test example for formatting features
use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Tree-sitter Perl Formatting Demo ===\n");

    let test_code = r#"
sub hello_world {
    my $name = shift;
    print "Hello, $name!\n";
}

hello_world("World");
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(test_code) {
        Ok(ast) => {
            println!("Parsed successfully!");
            println!("AST structure:");
            print_ast(&ast, 0);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

fn print_ast(node: &AstNode, indent: usize) {
    let prefix = "  ".repeat(indent);

    match node {
        AstNode::Program(items) => {
            println!("{}Program ({} items)", prefix, items.len());
            for item in items {
                print_ast(item, indent + 1);
            }
        }
        AstNode::Statement(content) => {
            println!("{}Statement", prefix);
            print_ast(content, indent + 1);
        }
        AstNode::SubDeclaration { name, body, .. } => {
            println!("{}SubDeclaration: {}", prefix, name);
            print_ast(body, indent + 1);
        }
        AstNode::Block(statements) => {
            println!("{}Block ({} statements)", prefix, statements.len());
            for stmt in statements {
                print_ast(stmt, indent + 1);
            }
        }
        AstNode::VariableDeclaration { scope, variables, initializer } => {
            println!("{}VariableDeclaration: {}", prefix, scope);
            for var in variables {
                print_ast(var, indent + 1);
            }
            if let Some(init) = initializer {
                print_ast(init, indent + 1);
            }
        }
        AstNode::ForStatement { label, init, condition, update, block } => {
            println!("{}ForStatement", prefix);
            if let Some(label) = label {
                println!("{}  Label: {}", prefix, label);
            }
            if let Some(init) = init {
                print_ast(init, indent + 1);
            }
            if let Some(condition) = condition {
                print_ast(condition, indent + 1);
            }
            if let Some(update) = update {
                print_ast(update, indent + 1);
            }
            print_ast(block, indent + 1);
        }
        AstNode::FunctionCall { function, args } => {
            println!("{}FunctionCall", prefix);
            print_ast(function, indent + 1);
            for arg in args {
                print_ast(arg, indent + 1);
            }
        }
        AstNode::Identifier(name) => {
            println!("{}Identifier: {}", prefix, name);
        }
        AstNode::String(s) => {
            println!("{}String: \"{}\"", prefix, s);
        }
        AstNode::Number(n) => {
            println!("{}Number: {}", prefix, n);
        }
        AstNode::List(elements) => {
            println!("{}List ({} elements)", prefix, elements.len());
            for elem in elements {
                print_ast(elem, indent + 1);
            }
        }
        _ => {
            println!("{}Other node type", prefix);
        }
    }
}
