use std::env;
use perl_parser::{Parser, NodeKind};

fn main() {
    let input = env::args().nth(1).unwrap_or_else(|| "return if 1;".to_string());
    
    println!("Parsing: {}", input);
    let mut parser = Parser::new(&input);
    
    match parser.parse() {
        Ok(ast) => {
            println!("Success! AST:");
            print_ast(&ast, 0);
            println!("\nS-expression: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

fn print_ast(node: &perl_parser::Node, indent: usize) {
    let prefix = "  ".repeat(indent);
    match &node.kind {
        NodeKind::Program { statements } => {
            println!("{}Program with {} statements", prefix, statements.len());
            for stmt in statements {
                print_ast(stmt, indent + 1);
            }
        }
        NodeKind::Return { value } => {
            println!("{}Return", prefix);
            if let Some(val) = value {
                print_ast(val, indent + 1);
            }
        }
        NodeKind::Identifier { name } => {
            println!("{}Identifier: {}", prefix, name);
        }
        NodeKind::Number { value } => {
            println!("{}Number: {}", prefix, value);
        }
        _ => {
            println!("{}Node: {:?}", prefix, node.kind);
        }
    }
}