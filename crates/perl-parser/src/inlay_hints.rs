// crates/perl-parser/src/inlay_hints.rs
use crate::ast::{Node, NodeKind};
use serde_json::Value;
use serde_json::json;

pub fn parameter_hints(ast: &Node, to_pos16: &impl Fn(usize) -> (u32, u32)) -> Vec<Value> {
    let mut out = Vec::new();
    walk_ast(ast, &mut |node| {
        if let NodeKind::FunctionCall { name, args } = &node.kind {
            let labels: Option<&[&str]> = match name.as_str() {
                "substr" => Some(&["str", "offset", "len"]),
                "index" => Some(&["str", "substr", "pos"]),
                "rindex" => Some(&["str", "substr", "pos"]),
                "sprintf" => Some(&["format", "args..."]),
                "printf" => Some(&["format", "args..."]),
                "join" => Some(&["sep", "list"]),
                "split" => Some(&["pattern", "str", "limit"]),
                "splice" => Some(&["array", "offset", "length", "list"]),
                "unpack" => Some(&["template", "expr"]),
                "pack" => Some(&["template", "list"]),
                "grep" => Some(&["block", "list"]),
                "map" => Some(&["block", "list"]),
                "sort" => Some(&["block", "list"]),
                _ => None,
            };
            if let Some(sig) = labels {
                for (i, arg) in args.iter().enumerate() {
                    if i >= sig.len() {
                        break;
                    }
                    let (l, c) = to_pos16(arg.location.start);
                    out.push(json!({
                        "position": { "line": l, "character": c },
                        "label": format!("{}:", sig[i]),
                        "kind": 2, // parameter
                        "paddingLeft": false,
                        "paddingRight": true
                    }));
                }
            }
        }
        true
    });
    out
}

pub fn trivial_type_hints(ast: &Node, to_pos16: &impl Fn(usize) -> (u32, u32)) -> Vec<Value> {
    let mut out = Vec::new();
    walk_ast(ast, &mut |node| {
        let type_hint = match &node.kind {
            NodeKind::Number { .. } => Some("Num"),
            NodeKind::String { .. } => Some("Str"),
            NodeKind::HashLiteral { .. } => Some("Hash"),
            NodeKind::ArrayLiteral { .. } => Some("Array"),
            NodeKind::Regex { .. } => Some("Regex"),
            NodeKind::Subroutine { name: None, .. } => Some("CodeRef"),
            _ => None,
        };

        if let Some(hint) = type_hint {
            let (l, c) = to_pos16(node.location.end);
            out.push(json!({
                "position": {"line": l, "character": c},
                "label": format!(": {}", hint),
                "kind": 1, // type
                "paddingLeft": true,
                "paddingRight": false
            }));
        }
        true
    });
    out
}

fn walk_ast<F>(node: &Node, visitor: &mut F) -> bool
where
    F: FnMut(&Node) -> bool,
{
    if !visitor(node) {
        return false;
    }

    for child in crate::declaration::get_node_children(node) {
        if !walk_ast(child, visitor) {
            return false;
        }
    }

    true
}
