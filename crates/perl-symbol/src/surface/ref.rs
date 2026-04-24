//! `SymbolRef` — a projected symbol *reference/use* site from the Perl AST.
//!
//! This phase intentionally targets a narrow, high-confidence subset:
//! - variable references (`$x`, `@items`, `%opts`)
//! - subroutine call references (`foo(...)` / bareword calls)
//! - package-qualified forms where the AST encodes them directly
//!   (`$Pkg::var`, `Pkg::func(...)`)

use crate::types::VarKind;
use perl_ast::{Node, NodeKind};

/// Classification for projected symbol references.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolRefKind {
    /// Variable usage (`$x`, `@items`, `%opts`).
    Variable(VarKind),
    /// Subroutine invocation (`foo(...)`).
    SubroutineCall,
}

/// A projected view of a symbol reference/use site in Perl source.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRef {
    /// Reference classification.
    pub kind: SymbolRefKind,
    /// Unqualified symbol name.
    pub name: String,
    /// Package-qualified name when syntactically explicit, else bare `name`.
    pub qualified_name: String,
    /// Variable sigil (`$`, `@`, `%`, `&`, `*`) for variable refs.
    pub sigil: Option<String>,
    /// Explicit package qualifier from syntax (for example `Some("Pkg")` for
    /// `Pkg::func` or `$Pkg::var`).
    pub package_qualifier: Option<String>,
    /// Byte offsets `(start, end)` for the whole reference node.
    pub full_span: (usize, usize),
    /// Byte offsets for the reference anchor token.
    pub anchor_span: Option<(usize, usize)>,
}

/// Walk `root` and collect a flat list of high-confidence symbol references.
pub fn extract_symbol_refs(root: &Node) -> Vec<SymbolRef> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn walk(node: &Node, out: &mut Vec<SymbolRef>) {
    match &node.kind {
        // Skip declaration targets; only walk initializer expressions.
        NodeKind::VariableDeclaration { initializer, .. }
        | NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                walk(init, out);
            }
        }

        NodeKind::Variable { sigil, name } => {
            if let Some(var_kind) = var_kind_from_sigil(sigil) {
                let (package_qualifier, bare_name, qualified_name) = split_qualified_name(name);
                out.push(SymbolRef {
                    kind: SymbolRefKind::Variable(var_kind),
                    name: bare_name,
                    qualified_name,
                    sigil: Some(sigil.clone()),
                    package_qualifier,
                    full_span: (node.location.start, node.location.end),
                    anchor_span: Some((node.location.start, node.location.end)),
                });
            }
        }

        NodeKind::FunctionCall { name, args } => {
            let (package_qualifier, bare_name, qualified_name) = split_qualified_name(name);
            out.push(SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: bare_name,
                qualified_name,
                sigil: None,
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            });

            for arg in args {
                walk(arg, out);
            }
        }

        _ => {
            node.for_each_child(|child| walk(child, out));
        }
    }
}

fn split_qualified_name(name: &str) -> (Option<String>, String, String) {
    if let Some((package, bare)) = name.rsplit_once("::")
        && !package.is_empty()
        && !bare.is_empty()
    {
        return (Some(package.to_owned()), bare.to_owned(), name.to_owned());
    }

    (None, name.to_owned(), name.to_owned())
}

fn var_kind_from_sigil(sigil: &str) -> Option<VarKind> {
    match sigil {
        "$" => Some(VarKind::Scalar),
        "@" => Some(VarKind::Array),
        "%" => Some(VarKind::Hash),
        _ => None,
    }
}
