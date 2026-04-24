//! `SymbolRef` — a lightweight projected symbol-reference site from the Perl AST.
//!
//! Phase 1 intentionally keeps scope narrow and high-confidence:
//! variable references, function-call references, and package-qualified
//! bareword references where the AST is explicit.

use crate::types::{SymbolKind, VarKind};
use perl_ast::{Node, NodeKind};

/// A projected view of a single symbol reference (usage) site in Perl source.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRef {
    /// Symbol classification for the reference.
    pub kind: SymbolKind,
    /// Unqualified symbol name (e.g. `value`, `greet`).
    pub name: String,
    /// Fully-qualified symbol name when explicit in source; otherwise equals `name`.
    pub qualified_name: String,
    /// Byte offsets `(start, end)` of the full reference node.
    pub full_span: (usize, usize),
    /// Byte offsets `(start, end)` of the anchor token when available.
    pub anchor_span: Option<(usize, usize)>,
}

/// Walk `root` and collect high-confidence symbol references.
///
/// Phase-1 references emitted:
///
/// - `NodeKind::Variable` usages (outside declaration binding positions)
/// - `NodeKind::FunctionCall` callee names
/// - package-qualified `NodeKind::Identifier` names containing `::`
pub fn extract_symbol_refs(root: &Node) -> Vec<SymbolRef> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn walk(node: &Node, out: &mut Vec<SymbolRef>) {
    match &node.kind {
        NodeKind::VariableDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                walk(init, out);
            }
        }
        NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                walk(init, out);
            }
        }
        NodeKind::Variable { sigil, name } => {
            let kind = sigil_to_symbol_kind(sigil);
            let qualified_name = qualify_if_explicit(name);
            out.push(SymbolRef {
                kind,
                name: unqualified_name(name),
                qualified_name,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            });
        }
        NodeKind::FunctionCall { name, args } => {
            out.push(SymbolRef {
                kind: SymbolKind::Subroutine,
                name: unqualified_name(name),
                qualified_name: qualify_if_explicit(name),
                full_span: (node.location.start, node.location.end),
                anchor_span: None,
            });

            for arg in args {
                walk(arg, out);
            }
        }
        NodeKind::Identifier { name } if name.contains("::") => {
            out.push(SymbolRef {
                kind: SymbolKind::Package,
                name: unqualified_name(name),
                qualified_name: name.clone(),
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            });
        }
        _ => {
            node.for_each_child(|child| walk(child, out));
        }
    }
}

fn sigil_to_symbol_kind(sigil: &str) -> SymbolKind {
    match sigil {
        "@" => SymbolKind::Variable(VarKind::Array),
        "%" => SymbolKind::Variable(VarKind::Hash),
        _ => SymbolKind::Variable(VarKind::Scalar),
    }
}

fn unqualified_name(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_owned()
}

fn qualify_if_explicit(name: &str) -> String {
    name.to_owned()
}
