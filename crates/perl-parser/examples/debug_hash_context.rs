use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};
use std::collections::HashMap;

fn debug_hash_context(node: &Node, parent_map: &HashMap<*const Node, &Node>) -> bool {
    println!(
        "  Checking hash context for: {:?}",
        match &node.kind {
            NodeKind::Identifier { name } => format!("Identifier({})", name),
            NodeKind::Variable { sigil, name } => format!("Variable({}{})", sigil, name),
            _ => "Other".to_string(),
        }
    );

    let mut current = node as *const Node;
    let mut depth = 0;
    const MAX_TRAVERSAL_DEPTH: usize = 10;

    while let Some(parent) = parent_map.get(&current) {
        if depth > MAX_TRAVERSAL_DEPTH {
            break;
        }

        println!(
            "    Parent {}: {:?}",
            depth,
            match &parent.kind {
                NodeKind::Binary { op, .. } => format!("Binary(op: {})", op),
                NodeKind::Program { .. } => "Program".to_string(),
                NodeKind::VariableDeclaration { .. } => "VariableDeclaration".to_string(),
                NodeKind::ExpressionStatement { .. } => "ExpressionStatement".to_string(),
                NodeKind::FunctionCall { name, .. } => format!("FunctionCall({})", name),
                _ => "Other".to_string(),
            }
        );

        match &parent.kind {
            NodeKind::Binary { op, left: _, right } if op == "{}" => {
                if std::ptr::eq(right.as_ref(), current) {
                    println!("    -> Found hash key context at depth {}", depth);
                    return true;
                }
            }
            _ => {}
        }

        current = *parent as *const _;
        depth += 1;
    }

    println!("    -> No hash key context found");
    false
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"use strict;
my %h = ();
my $x = $h{key};
print FOO;"#;

    println!("Code:\n{}", code);

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Build parent map
    let mut parent_map: HashMap<*const Node, &Node> = HashMap::new();
    build_parent_map(&ast, None, &mut parent_map);

    // Find all identifier nodes
    let mut identifiers = Vec::new();
    find_identifiers(&ast, &mut identifiers);

    println!("Found {} identifiers:", identifiers.len());
    for (i, node) in identifiers.iter().enumerate() {
        if let NodeKind::Identifier { name } = &node.kind {
            println!("\n{}. Identifier: {}", i + 1, name);
            let is_hash_key = debug_hash_context(node, &parent_map);
            println!("   Is hash key: {}", is_hash_key);
        }
    }
    Ok(())
}

fn build_parent_map<'a>(
    node: &'a Node,
    parent: Option<&'a Node>,
    map: &mut HashMap<*const Node, &'a Node>,
) {
    if let Some(p) = parent {
        map.insert(node as *const _, p);
    }

    for child in get_children(node) {
        build_parent_map(child, Some(node), map);
    }
}

fn find_identifiers<'a>(node: &'a Node, identifiers: &mut Vec<&'a Node>) {
    if let NodeKind::Identifier { .. } = &node.kind {
        identifiers.push(node);
    }

    for child in get_children(node) {
        find_identifiers(child, identifiers);
    }
}

fn get_children(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } => statements.iter().collect(),
        NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut children = vec![variable.as_ref()];
            if let Some(init) = initializer {
                children.push(init.as_ref());
            }
            children
        }
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        NodeKind::FunctionCall { args, .. } => args.iter().collect(),
        _ => vec![],
    }
}
