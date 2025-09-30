use perl_parser::{
    DiagnosticsProvider, Parser,
    ast::{Node, NodeKind},
};

fn trace_ast_nodes(node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match &node.kind {
        NodeKind::Identifier { name } => {
            println!(
                "{}Identifier: {} at range {}..{}",
                indent, name, node.location.start, node.location.end
            );
        }
        NodeKind::Variable { sigil, name } => {
            println!(
                "{}Variable: {}{} at range {}..{}",
                indent, sigil, name, node.location.start, node.location.end
            );
        }
        NodeKind::Program { statements } => {
            println!("{}Program with {} statements", indent, statements.len());
            for stmt in statements {
                trace_ast_nodes(stmt, depth + 1);
            }
        }
        NodeKind::Use { module, .. } => {
            println!("{}Use: {}", indent, module);
        }
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            println!("{}VariableDeclaration: {}", indent, declarator);
            trace_ast_nodes(variable, depth + 1);
            if let Some(init) = initializer {
                trace_ast_nodes(init, depth + 1);
            }
        }
        NodeKind::Binary { op, left, right } => {
            println!(
                "{}Binary: {} at range {}..{}",
                indent, op, node.location.start, node.location.end
            );
            trace_ast_nodes(left, depth + 1);
            trace_ast_nodes(right, depth + 1);
        }
        NodeKind::ExpressionStatement { expression } => {
            println!("{}ExpressionStatement", indent);
            trace_ast_nodes(expression, depth + 1);
        }
        NodeKind::FunctionCall { name, args } => {
            println!("{}FunctionCall: {} with {} args", indent, name, args.len());
            for arg in args {
                trace_ast_nodes(arg, depth + 1);
            }
        }
        _ => {
            println!("{}Other: {:?}", indent, std::mem::discriminant(&node.kind));
        }
    }
}

fn main() {
    let code = r#"use strict;
my %h = ();
my $x = $h{key};
print FOO;"#;

    println!("Code:\n{}", code);

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    println!("\n=== AST Structure ===");
    trace_ast_nodes(&ast, 0);

    println!("\n=== S-expression ===");
    println!("{}", ast.to_sexp());

    println!("\n=== Diagnostics ===");
    let diagnostics_provider = DiagnosticsProvider::new(&ast, code.to_string());
    let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], code);

    println!("Found {} diagnostics:", diagnostics.len());
    for diagnostic in &diagnostics {
        println!(
            "  - {:?}: {} at range {:?}",
            diagnostic.code, diagnostic.message, diagnostic.range
        );
    }
}
