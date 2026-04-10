//! `SymbolDecl` — a projected declaration site derived from the Perl AST.
//!
//! [`SymbolDecl`] captures everything an IDE feature needs about a declaration:
//! its kind, unqualified name, package-qualified name, full node span, name
//! anchor span, and the containing package (if any).
//!
//! [`extract_symbol_decls`] performs a single recursive walk of the AST and
//! collects every named declaration into a flat `Vec<SymbolDecl>`.  It tracks
//! the *current package* as it descends so that subroutines and variables
//! declared inside a `package Foo { }` block are emitted with the correct
//! `container` and `qualified_name`.

use perl_ast::{Node, NodeKind};
use perl_symbol_types::{SymbolKind, VarKind};

// ── Public types ──────────────────────────────────────────────────────────────

/// A projected view of a single symbol declaration site in Perl source.
///
/// This struct gives IDE consumers a uniform representation of every
/// declaration, regardless of whether it originated from a `sub`, `package`,
/// `my $var`, `use constant`, or `class` keyword.
///
/// # Fields
///
/// - `kind` — the unified symbol kind (see [`SymbolKind`])
/// - `name` — the bare, unqualified name (e.g. `"greet"`)
/// - `qualified_name` — fully-qualified when inside a package (e.g.
///   `"Foo::greet"`); equals `name` at the top level
/// - `full_span` — byte range of the *entire* declaration node
///   `(start, end)` relative to the source string
/// - `anchor_span` — byte range of just the *name token*, used for
///   precise go-to-definition and rename anchors.  `None` when the AST
///   does not carry a `name_span` for the node (e.g. `use constant` or
///   `class`).
/// - `container` — unqualified name of the enclosing package, `None` at
///   the top level
/// - `declarator` — the scope keyword used to declare variables (`"my"`,
///   `"our"`, `"local"`, `"state"`), or `None` for non-variable declarations
///   such as subroutines, packages, and constants.  `"our"` variables are
///   package-scoped and visible across files; `"my"` variables are
///   lexically-scoped and file-local.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolDecl {
    /// Symbol classification.
    pub kind: SymbolKind,
    /// Unqualified name of the declared symbol.
    pub name: String,
    /// Package-qualified name (`Foo::bar`) or bare name at top level.
    pub qualified_name: String,
    /// Byte offsets `(start, end)` of the full declaration node.
    pub full_span: (usize, usize),
    /// Byte offsets `(start, end)` of the name token, if known.
    pub anchor_span: Option<(usize, usize)>,
    /// Enclosing package name, if the declaration is inside a `package`.
    pub container: Option<String>,
    /// Scope declarator for variable declarations: `"my"`, `"our"`, `"local"`,
    /// or `"state"`.  `None` for non-variable declarations (subroutines,
    /// packages, constants, classes).
    ///
    /// `"our"` variables are package-scoped and reachable from other files in
    /// the same package.  `"my"` variables are lexically-scoped and invisible
    /// outside their enclosing block.
    pub declarator: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Walk `root` and collect every named symbol declaration into a flat list.
///
/// `current_package` seeds the initial package context.  Pass `Some("main")`
/// to start under the implicit `main` package, or `None` if callers handle the
/// top-level context themselves.
///
/// # Declaration kinds produced
///
/// | AST node | `SymbolKind` |
/// |----------|-------------|
/// | `Package { name, .. }` | `Package` |
/// | `Class { name, .. }` | `Class` |
/// | `Subroutine { name: Some(..), .. }` | `Subroutine` |
/// | `Method { name, .. }` | `Method` |
/// | `VariableDeclaration { variable, .. }` | `Variable(VarKind)` |
/// | `Use { module: "constant", args, .. }` | `Constant` |
///
/// Anonymous subroutines (`name: None`) are skipped.
///
/// # Package context propagation
///
/// The walker tracks the innermost `package` declaration linearly (statement
/// order).  When a `Package { block: Some(..) }` node is encountered the
/// package scope applies only within that block; otherwise it applies to all
/// subsequent top-level statements.
pub fn extract_symbol_decls(root: &Node, current_package: Option<&str>) -> Vec<SymbolDecl> {
    let mut out = Vec::new();
    let mut ctx = WalkCtx { current_package: current_package.map(str::to_owned) };
    walk(root, &mut ctx, &mut out);
    out
}

// ── Internal walker ───────────────────────────────────────────────────────────

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

/// Recursively walk `node`, emitting `SymbolDecl` values into `out`.
fn walk(node: &Node, ctx: &mut WalkCtx, out: &mut Vec<SymbolDecl>) {
    match &node.kind {
        // ── Package ────────────────────────────────────────────────────────
        NodeKind::Package { name, name_span, block } => {
            let anchor = Some((name_span.start, name_span.end));
            let container = ctx.current_package.clone();
            out.push(SymbolDecl {
                kind: SymbolKind::Package,
                name: name.clone(),
                qualified_name: name.clone(),
                full_span: (node.location.start, node.location.end),
                anchor_span: anchor,
                container,
                declarator: None,
            });

            // If the package has a block, descend with package context scoped
            // to just that block.
            if let Some(blk) = block {
                let saved = ctx.current_package.replace(name.clone());
                walk(blk, ctx, out);
                ctx.current_package = saved;
            } else {
                // Bare `package Foo;` — update context for subsequent siblings.
                // The caller's loop (Program / Block) must handle this linearly;
                // here we update the shared context directly.
                ctx.current_package = Some(name.clone());
            }
        }

        // ── Class ──────────────────────────────────────────────────────────
        NodeKind::Class { name, body, .. } => {
            let container = ctx.current_package.clone();
            out.push(SymbolDecl {
                kind: SymbolKind::Class,
                name: name.clone(),
                qualified_name: ctx.qualify(name),
                full_span: (node.location.start, node.location.end),
                anchor_span: None, // Class has no name_span in current AST
                container,
                declarator: None,
            });

            // Walk class body with the class name as the package context.
            let saved = ctx.current_package.replace(name.clone());
            walk(body, ctx, out);
            ctx.current_package = saved;
        }

        // ── Subroutine ─────────────────────────────────────────────────────
        NodeKind::Subroutine { name: Some(sub_name), name_span, body, .. } => {
            let anchor = name_span.as_ref().map(|s| (s.start, s.end));
            let container = ctx.current_package.clone();
            let qualified_name = ctx.qualify(sub_name);
            out.push(SymbolDecl {
                kind: SymbolKind::Subroutine,
                name: sub_name.clone(),
                qualified_name,
                full_span: (node.location.start, node.location.end),
                anchor_span: anchor,
                container,
                declarator: None,
            });
            // Walk the body — may contain nested subs or closures.
            walk(body, ctx, out);
        }

        // Anonymous subroutine — skip (no name to project).
        NodeKind::Subroutine { name: None, body, .. } => {
            walk(body, ctx, out);
        }

        // ── Method (Perl 5.38+ `use feature 'class'`) ─────────────────────
        NodeKind::Method { name: method_name, body, .. } => {
            let container = ctx.current_package.clone();
            let qualified_name = ctx.qualify(method_name);
            out.push(SymbolDecl {
                kind: SymbolKind::Method,
                name: method_name.clone(),
                qualified_name,
                full_span: (node.location.start, node.location.end),
                anchor_span: None, // Method has no name_span in current AST
                container,
                declarator: None,
            });
            walk(body, ctx, out);
        }

        // ── Variable declarations ──────────────────────────────────────────
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            if let Some(decl) = variable_decl_from_node(variable, node, ctx, declarator) {
                out.push(decl);
            }
            // Walk initializer for nested declarations (e.g. `my $x = sub { }`)
            if let Some(init) = initializer {
                walk(init, ctx, out);
            }
        }

        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            for var in variables {
                if let Some(decl) = variable_decl_from_node(var, node, ctx, declarator) {
                    out.push(decl);
                }
            }
            if let Some(init) = initializer {
                walk(init, ctx, out);
            }
        }

        // ── use constant NAME => value ─────────────────────────────────────
        NodeKind::Use { module, args, .. } if module == "constant" => {
            // `args` layout: [NAME, value, ...] or [NAME1, val1, NAME2, val2, ...]
            // The first arg is the constant name (or a hash-ref style with
            // multiple names — for MVP we take just the first string arg).
            if let Some(const_name) = args.first() {
                // Skip if it looks like a reference marker or is empty.
                if !const_name.is_empty() && !const_name.starts_with('{') {
                    let container = ctx.current_package.clone();
                    out.push(SymbolDecl {
                        kind: SymbolKind::Constant,
                        name: const_name.clone(),
                        qualified_name: ctx.qualify(const_name),
                        full_span: (node.location.start, node.location.end),
                        anchor_span: None, // No precise span available from Use node
                        container,
                        declarator: None,
                    });
                }
            }
        }

        // ── Containers: recurse into children ─────────────────────────────
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            walk_statements(statements, ctx, out);
        }

        NodeKind::ExpressionStatement { expression } => {
            walk(expression, ctx, out);
        }

        // All other nodes: no declaration to project.
        _ => {}
    }
}

/// Walk a slice of statement nodes linearly, so that `package Foo;` updates
/// the context before processing subsequent siblings.
fn walk_statements(statements: &[Node], ctx: &mut WalkCtx, out: &mut Vec<SymbolDecl>) {
    for stmt in statements {
        walk(stmt, ctx, out);
    }
}

/// Extract a `SymbolDecl` from a `Variable` node inside a `VariableDeclaration`.
///
/// `declarator` is the scope keyword (`"my"`, `"our"`, `"local"`, `"state"`)
/// from the enclosing declaration node; it is stored on the returned `SymbolDecl`
/// so consumers can distinguish package-scoped (`our`) from lexical (`my`) symbols.
///
/// Returns `None` for non-`Variable` children (e.g. `VariableWithAttributes`
/// wrapping — in that case the caller should unwrap further).
fn variable_decl_from_node(
    var_node: &Node,
    decl_node: &Node,
    ctx: &WalkCtx,
    declarator: &str,
) -> Option<SymbolDecl> {
    match &var_node.kind {
        NodeKind::Variable { sigil, name } => {
            let kind = sigil_to_symbol_kind(sigil);
            let anchor_span = Some((var_node.location.start, var_node.location.end));
            let container = ctx.current_package.clone();
            Some(SymbolDecl {
                kind,
                name: name.clone(),
                qualified_name: ctx.qualify(name),
                full_span: (decl_node.location.start, decl_node.location.end),
                anchor_span,
                container,
                declarator: Some(declarator.to_owned()),
            })
        }
        NodeKind::VariableWithAttributes { variable, .. } => {
            variable_decl_from_node(variable, decl_node, ctx, declarator)
        }
        _ => None,
    }
}

/// Map a Perl sigil string to the appropriate [`SymbolKind`].
fn sigil_to_symbol_kind(sigil: &str) -> SymbolKind {
    match sigil {
        "@" => SymbolKind::Variable(VarKind::Array),
        "%" => SymbolKind::Variable(VarKind::Hash),
        _ => SymbolKind::Variable(VarKind::Scalar),
    }
}
