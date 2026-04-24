//! `SymbolRef` — a projected reference/use site derived from the Perl AST.
//!
//! This is the phase-1 reference projection and intentionally covers only
//! high-confidence cases:
//! - variable references (`$x`, `@items`, `%opts`)
//! - subroutine call references (`foo(...)`)
//! - package-qualified symbol references where the AST text is explicit
//!   (for example `Foo::bar(...)`, `$Foo::value`, `*Foo::glob`)

use crate::types::VarKind;
use perl_ast::{Node, NodeKind};

/// Narrow, high-confidence reference categories for phase-1 extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolRefKind {
    /// Unqualified variable usage.
    Variable(VarKind),
    /// Unqualified subroutine/function call usage.
    SubroutineCall,
    /// Package-qualified symbol usage where qualification is explicit in AST text.
    QualifiedSymbol,
}

/// A projected view of a single symbol reference/use site in Perl source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    /// Reference category.
    pub kind: SymbolRefKind,
    /// Referenced symbol name as represented by AST.
    pub name: String,
    /// Byte offsets `(start, end)` of the full reference node.
    pub full_span: (usize, usize),
    /// Byte offsets `(start, end)` of the reference anchor token.
    pub anchor_span: (usize, usize),
}

/// Walk `root` and collect phase-1 symbol references into a flat list.
pub fn extract_symbol_refs(root: &Node) -> Vec<SymbolRef> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn walk(node: &Node, out: &mut Vec<SymbolRef>) {
    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            let kind = if name.contains("::") {
                SymbolRefKind::QualifiedSymbol
            } else if let Some(var_kind) = var_kind_from_sigil(sigil) {
                SymbolRefKind::Variable(var_kind)
            } else {
                SymbolRefKind::QualifiedSymbol
            };

            out.push(SymbolRef {
                kind,
                name: name.clone(),
                full_span: (node.location.start, node.location.end),
                anchor_span: (node.location.start, node.location.end),
            });
        }

        NodeKind::FunctionCall { name, .. } => {
            let kind = if name.contains("::") {
                SymbolRefKind::QualifiedSymbol
            } else {
                SymbolRefKind::SubroutineCall
            };

            out.push(SymbolRef {
                kind,
                name: name.clone(),
                full_span: (node.location.start, node.location.end),
                anchor_span: (node.location.start, node.location.end),
            });
        }

        NodeKind::Typeglob { name } if name.contains("::") => {
            out.push(SymbolRef {
                kind: SymbolRefKind::QualifiedSymbol,
                name: name.clone(),
                full_span: (node.location.start, node.location.end),
                anchor_span: (node.location.start, node.location.end),
            });
        }

        _ => {}
    }

    for child in node.children() {
        walk(child, out);
    }
}

fn var_kind_from_sigil(sigil: &str) -> Option<VarKind> {
    match sigil {
        "$" => Some(VarKind::Scalar),
        "@" => Some(VarKind::Array),
        "%" => Some(VarKind::Hash),
        _ => None,
    }
}
