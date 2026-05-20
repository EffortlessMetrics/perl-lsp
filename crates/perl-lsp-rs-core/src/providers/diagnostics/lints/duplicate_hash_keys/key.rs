use perl_parser_core::ast::{Node, NodeKind};

/// Extract the static string value of a hash key node, if statically known.
///
/// Returns `Some(key)` for `NodeKind::String` and `NodeKind::Number` keys.
/// Returns `None` for variable keys and other dynamic expressions.
pub(super) fn static_key_value(key: &Node) -> Option<String> {
    match &key.kind {
        NodeKind::String { value, .. } => Some(value.clone()),
        NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}
