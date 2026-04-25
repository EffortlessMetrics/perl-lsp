//! `SymbolRef` — a projected symbol *reference/use* site from the Perl AST.
//!
//! This phase intentionally targets a narrow, high-confidence subset:
//! - variable references (`$x`, `@items`, `%opts`, `$#array`)
//! - subroutine call references (`foo(...)` / bareword calls via `NodeKind::FunctionCall`)
//! - package-qualified forms where the AST encodes them directly
//!   (`$Pkg::var`, `Pkg::func(...)`)
//!
//! # Phase-1 Intentional Exclusions
//!
//! The following reference types are **not** emitted in this phase:
//! - `&foo` / `\&foo` code-reference variables (sigil `&`) — `VarKind` has no `CodeRef` variant
//! - `*foo` typeglob variables (sigil `*`) — reserved for a future phase
//! - Method calls (`$obj->method(...)`, `NodeKind::MethodCall`) — method name is not a
//!   `Node`, so extraction requires a dedicated `MethodCallRef` kind (future phase)
//! - Indirect-object calls (`new Class @args`, `NodeKind::IndirectCall`) — same reason

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
    /// Variable sigil (`$`, `@`, `%`) for variable refs.  `&` and `*` sigils are
    /// not emitted in phase-1 (see module-level exclusion list).
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
            // The parser reuses FunctionCall for a few non-call constructs using
            // sentinel names that contain non-identifier characters or are reserved
            // keywords.  Filter them out so consumers never see synthetic nodes:
            //   "->()": anonymous coderef invocation `$ref->(args)` — no sub name
            //   "&{}":  coderef dereference
            //   "field": Perl 5.38+ OOP `field $x => accessor` form — a declaration,
            //            not a call; must not be reported as a SubroutineCall ref.
            let is_sentinel = matches!(name.as_str(), "->()" | "&{}" | "field");
            if !is_sentinel {
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
            }

            for arg in args {
                walk(arg, out);
            }
        }

        _ => {
            node.for_each_child(|child| walk(child, out));
        }
    }
}

/// Split a potentially package-qualified name into `(qualifier, bare, full)`.
///
/// Returns `(Some("Pkg::Sub"), "baz", "Pkg::Sub::baz")` for `"Pkg::Sub::baz"`.
/// Returns `(None, name, name)` for bare names and for degenerate forms like
/// `"Foo::"` (trailing `::`, empty bare component) or `"::bar"` (empty package).
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
        // `$#array` is the last-index sigil; the value is a scalar integer.
        "$#" => Some(VarKind::Scalar),
        "@" => Some(VarKind::Array),
        "%" => Some(VarKind::Hash),
        // `&` (code reference) and `*` (typeglob) are phase-1 exclusions.
        _ => None,
    }
}
