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

use crate::types::{SymbolKind, VarKind};
use perl_ast::{Node, NodeKind};

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
    let mut ctx = WalkCtx {
        current_package: current_package.map(str::to_owned),
        const_fast_enabled: false,
        readonly_enabled: false,
    };
    walk(root, &mut ctx, &mut out);
    out
}

// ── Internal walker ───────────────────────────────────────────────────────────

struct WalkCtx {
    current_package: Option<String>,
    const_fast_enabled: bool,
    readonly_enabled: bool,
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
            for const_name in constant_names_from_use_args(args) {
                let container = ctx.current_package.clone();
                out.push(SymbolDecl {
                    kind: SymbolKind::Constant,
                    name: const_name.clone(),
                    qualified_name: ctx.qualify(&const_name),
                    full_span: (node.location.start, node.location.end),
                    anchor_span: None, // No precise span available from Use node
                    container,
                    declarator: None,
                });
            }
        }

        NodeKind::Use { module, .. } if module == "Const::Fast" => {
            ctx.const_fast_enabled = true;
        }

        NodeKind::Use { module, .. } if module == "Readonly" => {
            ctx.readonly_enabled = true;
        }

        NodeKind::FunctionCall { name, args } if ctx.const_fast_enabled && name == "const" => {
            for arg in args {
                push_const_fast_decl(arg, node, ctx, out);
            }
        }

        NodeKind::FunctionCall { name, args } if ctx.readonly_enabled && name == "Readonly" => {
            for arg in args {
                push_readonly_decl(arg, node, ctx, out);
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

fn constant_names_from_use_args(args: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    let mut brace_depth = 0usize;
    let mut fallback_name: Option<String> = None;

    for (idx, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "{" => {
                brace_depth += 1;
                continue;
            }
            "}" => {
                brace_depth = brace_depth.saturating_sub(1);
                continue;
            }
            "+" | "," => continue,
            _ => {}
        }

        if let Some(qw_names) = qw_names(arg) {
            for name in qw_names {
                push_unique(&mut names, name);
            }
            continue;
        }

        if is_constant_name_candidate(arg) {
            if brace_depth == 1 && args.get(idx + 1).is_some_and(|next| next == "=>") {
                push_unique(&mut names, arg.clone());
                continue;
            }

            if brace_depth == 0 && fallback_name.is_none() {
                fallback_name = Some(arg.clone());
            }
        }
    }

    if names.is_empty() {
        if let Some(name) = fallback_name {
            names.push(name);
        }
    }

    names
}

fn qw_names(arg: &str) -> Option<Vec<String>> {
    let rest = arg.strip_prefix("qw")?.trim_start();
    let content = rest
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .or_else(|| rest.strip_prefix('[').and_then(|s| s.strip_suffix(']')))
        .or_else(|| rest.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .or_else(|| rest.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .or_else(|| rest.strip_prefix('/').and_then(|s| s.strip_suffix('/')));

    content.map(|text| {
        text.split_whitespace().filter(|name| !name.is_empty()).map(str::to_owned).collect()
    })
}

fn is_constant_name_candidate(arg: &str) -> bool {
    !arg.is_empty()
        && arg != "=>"
        && !arg.starts_with('{')
        && !arg.starts_with('}')
        && !arg.starts_with('-')
        && !arg.starts_with('$')
        && !arg.starts_with('@')
        && !arg.starts_with('%')
}

fn push_unique(names: &mut Vec<String>, name: String) {
    if !names.iter().any(|existing| existing == &name) {
        names.push(name);
    }
}

fn push_const_fast_decl(arg: &Node, call_node: &Node, ctx: &WalkCtx, out: &mut Vec<SymbolDecl>) {
    match &arg.kind {
        NodeKind::VariableDeclaration { variable, .. } => {
            if let Some(decl) = constant_wrapper_decl_from_node(variable, call_node, ctx, "const") {
                out.push(decl);
            }
        }
        NodeKind::VariableListDeclaration { variables, .. } => {
            for variable in variables {
                if let Some(decl) =
                    constant_wrapper_decl_from_node(variable, call_node, ctx, "const")
                {
                    out.push(decl);
                }
            }
        }
        _ => {}
    }
}

fn push_readonly_decl(arg: &Node, call_node: &Node, ctx: &WalkCtx, out: &mut Vec<SymbolDecl>) {
    match &arg.kind {
        NodeKind::VariableDeclaration { variable, .. } => {
            if let Some(decl) =
                constant_wrapper_decl_from_node(variable, call_node, ctx, "Readonly")
            {
                out.push(decl);
            }
        }
        NodeKind::VariableListDeclaration { variables, .. } => {
            for variable in variables {
                if let Some(decl) =
                    constant_wrapper_decl_from_node(variable, call_node, ctx, "Readonly")
                {
                    out.push(decl);
                }
            }
        }
        _ => {}
    }
}

fn constant_wrapper_decl_from_node(
    var_node: &Node,
    call_node: &Node,
    ctx: &WalkCtx,
    declarator: &str,
) -> Option<SymbolDecl> {
    match &var_node.kind {
        NodeKind::Variable { name, .. } => {
            let anchor_span = Some((var_node.location.start, var_node.location.end));
            let container = ctx.current_package.clone();
            Some(SymbolDecl {
                kind: SymbolKind::Constant,
                name: name.clone(),
                qualified_name: ctx.qualify(name),
                full_span: (call_node.location.start, call_node.location.end),
                anchor_span,
                container,
                declarator: Some(declarator.to_string()),
            })
        }
        NodeKind::VariableWithAttributes { variable, .. } => {
            constant_wrapper_decl_from_node(variable, call_node, ctx, declarator)
        }
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::{constant_names_from_use_args, is_constant_name_candidate, qw_names};

    #[test]
    fn constant_names_extract_hash_style_pairs() {
        let args = vec![
            "{".to_string(),
            "FOO".to_string(),
            "=>".to_string(),
            "1".to_string(),
            ",".to_string(),
            "BAR".to_string(),
            "=>".to_string(),
            "2".to_string(),
            "}".to_string(),
        ];

        assert_eq!(constant_names_from_use_args(&args), vec!["FOO".to_string(), "BAR".to_string()]);
    }

    #[test]
    fn constant_names_falls_back_to_first_top_level_candidate() {
        let args = vec!["ANSWER".to_string(), "=>".to_string(), "42".to_string()];

        assert_eq!(constant_names_from_use_args(&args), vec!["ANSWER".to_string()]);
    }

    #[test]
    fn constant_names_supports_qw_and_deduplicates_entries() {
        let args = vec!["qw(ONE TWO ONE)".to_string()];

        assert_eq!(constant_names_from_use_args(&args), vec!["ONE".to_string(), "TWO".to_string()]);
    }

    #[test]
    fn qw_names_support_multiple_delimiters() {
        assert_eq!(qw_names("qw(one two)"), Some(vec!["one".to_string(), "two".to_string()]));
        assert_eq!(qw_names("qw[one two]"), Some(vec!["one".to_string(), "two".to_string()]));
        assert_eq!(qw_names("qw{one two}"), Some(vec!["one".to_string(), "two".to_string()]));
        assert_eq!(qw_names("qw<one two>"), Some(vec!["one".to_string(), "two".to_string()]));
        assert_eq!(qw_names("qw/one two/"), Some(vec!["one".to_string(), "two".to_string()]));
        assert_eq!(qw_names("qw [one two]"), Some(vec!["one".to_string(), "two".to_string()]));
    }

    #[test]
    fn constant_name_candidate_rejects_non_names() {
        assert!(is_constant_name_candidate("VALID_NAME"));
        assert!(!is_constant_name_candidate(""));
        assert!(!is_constant_name_candidate("=>"));
        assert!(!is_constant_name_candidate("$scalar"));
        assert!(!is_constant_name_candidate("@array"));
        assert!(!is_constant_name_candidate("%hash"));
        assert!(!is_constant_name_candidate("-flag"));
    }
}
