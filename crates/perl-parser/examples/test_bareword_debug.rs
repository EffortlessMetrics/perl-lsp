use perl_parser::{NodeKind, Parser, PragmaTracker};

fn main() {
    let source = r#"
use strict;

print FOO;  # Bareword not allowed
"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("AST parsed successfully");

            // Build pragma map
            let pragma_map = PragmaTracker::build(&ast);
            println!("\nPragma map:");
            for (range, state) in &pragma_map {
                println!("  Range {:?}: strict_subs={}", range, state.strict_subs);
            }

            // Walk the AST and check pragma state for each node
            println!("\nWalking AST nodes:");
            walk_ast(&ast, &pragma_map, 0);
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

fn walk_ast(
    node: &perl_parser::Node,
    pragma_map: &[(std::ops::Range<usize>, perl_parser::PragmaState)],
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    let pragma_state = PragmaTracker::state_for_offset(pragma_map, node.location.start);

    match &node.kind {
        NodeKind::Identifier { name } => {
            println!(
                "{}Identifier '{}' at offset {}: strict_subs={}",
                indent, name, node.location.start, pragma_state.strict_subs
            );
        }
        NodeKind::FunctionCall { name, args } => {
            println!(
                "{}FunctionCall '{}' at offset {}: strict_subs={}",
                indent, name, node.location.start, pragma_state.strict_subs
            );
            for arg in args {
                walk_ast(arg, pragma_map, depth + 1);
            }
        }
        NodeKind::Use { module, .. } => {
            println!(
                "{}Use '{}' at offset {}",
                indent, module, node.location.start
            );
        }
        _ => {
            // Recursively walk children
            for child in node.children() {
                walk_ast(child, pragma_map, depth + 1);
            }
        }
    }
}
