use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};

fn print_ast_structure(node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match &node.kind {
        NodeKind::Binary { op, left, right } => {
            println!("{}Binary {{ op: {:?}, left: ..., right: ... }}", indent, op);
            print_ast_structure(left, depth + 1);
            print_ast_structure(right, depth + 1);
        }
        NodeKind::Variable { sigil, name } => {
            println!("{}Variable {{ sigil: {:?}, name: {:?} }}", indent, sigil, name);
        }
        NodeKind::Identifier { name } => {
            println!("{}Identifier {{ name: {:?} }}", indent, name);
        }
        NodeKind::Program { statements } => {
            println!("{}Program {{ {} statements }}", indent, statements.len());
            for stmt in statements {
                print_ast_structure(stmt, depth + 1);
            }
        }
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            println!("{}VariableDeclaration {{ declarator: {:?} }}", indent, declarator);
            print_ast_structure(variable, depth + 1);
            if let Some(init) = initializer {
                println!("{}Initializer:", indent);
                print_ast_structure(init, depth + 1);
            }
        }
        _ => {
            println!("{}Other: {:?}", indent, std::mem::discriminant(&node.kind));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = $h{key};";
    println!("Code: {}", code);

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    println!("S-expression: {}", ast.to_sexp());
    println!("AST structure:");
    print_ast_structure(&ast, 0);
    Ok(())
}
