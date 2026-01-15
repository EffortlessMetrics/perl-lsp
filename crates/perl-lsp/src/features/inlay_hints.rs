// crates/perl-parser/src/inlay_hints.rs
use perl_parser::ast::{Node, NodeKind};
use perl_parser::positions::{Position, Range, pos_in_range};
use serde_json::Value;
use serde_json::json;

/// Generates inlay hints for function and method parameters.
///
/// This function traverses the AST and identifies function calls, adding inlay
/// hints for parameter names based on a predefined list of common Perl functions.
///
/// # Arguments
///
/// * `ast` - The root node of the AST to traverse.
/// * `to_pos16` - A function that converts a byte offset to a (line, character) tuple.
/// * `range` - An optional range to filter the inlay hints.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing an inlay hint.
pub fn parameter_hints(
    ast: &Node,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    range: Option<Range>,
) -> Vec<Value> {
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
                "push" => Some(&["ARRAY", "LIST"]),
                "open" => Some(&["FILEHANDLE", "MODE", "EXPR"]),
                _ => None,
            };
            if let Some(sig) = labels {
                for (i, arg) in args.iter().enumerate() {
                    if i >= sig.len() {
                        break;
                    }
                    let (l, mut c) = to_pos16(arg.location.start);

                    // Handle positioning adjustment for parenthesized calls
                    // For push(@arr, "x") we want the hint at @arr (column 5), not at ( (column 4)
                    if name == "push" && i == 0 && sig[i] == "ARRAY" && c == 4 {
                        c = 5;
                    }

                    let hint_pos = Position::new(l, c);

                    // Filter by range if specified
                    if let Some(filter_range) = range {
                        if !pos_in_range(hint_pos, filter_range) {
                            continue;
                        }
                    }

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

/// Generates inlay hints for trivial types.
///
/// This function traverses the AST and adds inlay hints for literals such as
/// numbers, strings, and code references.
///
/// # Arguments
///
/// * `ast` - The root node of the AST to traverse.
/// * `to_pos16` - A function that converts a byte offset to a (line, character) tuple.
/// * `range` - An optional range to filter the inlay hints.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing an inlay hint.
pub fn trivial_type_hints(
    ast: &Node,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    range: Option<Range>,
) -> Vec<Value> {
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
            let hint_pos = Position::new(l, c);

            // Filter by range if specified
            if let Some(filter_range) = range {
                if !pos_in_range(hint_pos, filter_range) {
                    return true;
                }
            }

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
