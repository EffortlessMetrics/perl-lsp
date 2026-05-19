//! Target classification for `goto` diagnostics.

use perl_parser_core::ast::{Node, NodeKind};

pub(crate) fn plain_label_name(target: &Node) -> Option<&str> {
    match &target.kind {
        NodeKind::Identifier { name } => Some(name.as_str()),
        _ => None,
    }
}
