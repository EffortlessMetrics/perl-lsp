//! `SymbolRef` — a projected symbol reference/use site derived from the Perl AST.
//!
//! Phase-1 extraction is intentionally narrow and high-confidence:
//! - variable references (`$foo`, `@items`, `%opts`)
//! - subroutine call references (`foo()`, `Pkg::foo()`)
//! - explicit package-qualified references where the AST already encodes `::`

use crate::types::{SymbolKind, VarKind};
use perl_ast::{Node, NodeKind};

/// A projected view of a single symbol reference/use site in Perl source.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRef {
    /// Symbol classification for the reference target.
    pub kind: SymbolKind,
    /// Unqualified symbol name.
    pub name: String,
    /// Package-qualified symbol name when known.
    pub qualified_name: String,
    /// Byte offsets `(start, end)` of the reference node.
    pub span: (usize, usize),
    /// Enclosing package name, if known from walk context.
    pub container: Option<String>,
    /// Explicit package qualifier parsed from the reference itself (e.g. `Foo` in `Foo::bar`).
    pub referenced_package: Option<String>,
}

/// Walk `root` and collect high-confidence symbol references into a flat list.
///
/// `current_package` seeds the initial package context used to qualify unqualified
/// references encountered during traversal.
pub fn extract_symbol_refs(root: &Node, current_package: Option<&str>) -> Vec<SymbolRef> {
    let mut out = Vec::new();
    let mut ctx = WalkCtx { current_package: current_package.map(str::to_owned) };
    walk(root, &mut ctx, &mut out);
    out
}

struct WalkCtx {
    current_package: Option<String>,
}

impl WalkCtx {
    fn qualify(&self, name: &str) -> String {
        match &self.current_package {
            Some(pkg) => format!("{}::{}", pkg, name),
            None => name.to_owned(),
        }
    }
}

fn walk(node: &Node, ctx: &mut WalkCtx, out: &mut Vec<SymbolRef>) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                walk(stmt, ctx, out);
            }
        }

        NodeKind::Package { name, block, .. } => {
            if let Some(blk) = block {
                let saved = ctx.current_package.replace(name.clone());
                walk(blk, ctx, out);
                ctx.current_package = saved;
            } else {
                ctx.current_package = Some(name.clone());
            }
        }

        NodeKind::Class { name, body, .. } => {
            let saved = ctx.current_package.replace(name.clone());
            walk(body, ctx, out);
            ctx.current_package = saved;
        }

        NodeKind::VariableDeclaration { initializer, .. }
        | NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                walk(init, ctx, out);
            }
        }

        NodeKind::MandatoryParameter { .. }
        | NodeKind::SlurpyParameter { .. }
        | NodeKind::NamedParameter { .. } => {}

        NodeKind::OptionalParameter { default_value, .. } => {
            walk(default_value, ctx, out);
        }

        NodeKind::Variable { sigil, name } => {
            if let Some(var_kind) = var_kind_from_sigil(sigil) {
                let (base_name, qualified_name, referenced_package) = resolve_name(name, ctx);
                out.push(SymbolRef {
                    kind: SymbolKind::Variable(var_kind),
                    name: base_name,
                    qualified_name,
                    span: (node.location.start, node.location.end),
                    container: ctx.current_package.clone(),
                    referenced_package,
                });
            }
        }

        NodeKind::FunctionCall { name, args } => {
            let (base_name, qualified_name, referenced_package) = resolve_name(name, ctx);
            out.push(SymbolRef {
                kind: SymbolKind::Subroutine,
                name: base_name,
                qualified_name,
                span: (node.location.start, node.location.end),
                container: ctx.current_package.clone(),
                referenced_package,
            });

            for arg in args {
                walk(arg, ctx, out);
            }
        }

        _ => {
            node.for_each_child(|child| walk(child, ctx, out));
        }
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

fn resolve_name(name: &str, ctx: &WalkCtx) -> (String, String, Option<String>) {
    if let Some((package, bare_name)) = name.rsplit_once("::") {
        return (
            bare_name.to_owned(),
            name.to_owned(),
            if package.is_empty() { None } else { Some(package.to_owned()) },
        );
    }

    (name.to_owned(), ctx.qualify(name), None)
}
