//! `SymbolRef` — a projected symbol *reference/use* site from the Perl AST.
//!
//! This phase intentionally targets a narrow, high-confidence subset:
//! - variable references (`$x`, `@items`, `%opts`, `$#array`)
//! - subroutine call references (`foo(...)` / bareword calls via `NodeKind::FunctionCall`)
//! - method call references (`$obj->method(...)`, `Pkg->method(...)`)
//! - coderef and typeglob boundary references (`&foo`, `\&foo`, `*alias`)
//! - package-qualified forms where the AST encodes them directly
//!   (`$Pkg::var`, `Pkg::func(...)`)
//!
//! # Phase-1 Intentional Exclusions
//!
//! The following reference types are **not** emitted in this phase:
//! - Indirect-object calls (`new Class @args`, `NodeKind::IndirectCall`) — same reason
//! - Subroutine signature parameter bindings (`MandatoryParameter`, `SlurpyParameter`,
//!   `NamedParameter`) — these are declaration sites, not reference sites.  Optional
//!   parameter *default values* are still walked because they are expressions.

use crate::types::VarKind;
use perl_ast::{Node, NodeKind};

/// Classification for projected symbol references.
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolRefKind {
    /// Variable usage (`$x`, `@items`, `%opts`).
    Variable(VarKind),
    /// Subroutine invocation (`foo(...)`).
    SubroutineCall,
    /// Instance method invocation (`$obj->method(...)`, `$self->method(...)`).
    MethodCall,
    /// Static/package method invocation (`Package->method(...)`).
    StaticMethodCall,
    /// Coderef-oriented reference (`&foo`, `\&foo`, `goto &foo`).
    CoderefReference,
    /// Typeglob reference or alias boundary (`*foo`, `*alias = ...`).
    TypeglobReference,
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
    /// Variable sigil (`$`, `@`, `%`) for variable refs, or boundary sigils
    /// (`&`, `*`) for coderef/typeglob refs.
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

        // Signature parameter nodes bind variables — skip the variable (it is a
        // declaration site), but walk the default-value expression for optional
        // parameters because it is evaluated in the caller's scope.
        NodeKind::MandatoryParameter { .. }
        | NodeKind::SlurpyParameter { .. }
        | NodeKind::NamedParameter { .. } => {
            // Nothing to walk: the bound variable is a declaration, not a ref.
        }
        NodeKind::OptionalParameter { default_value, .. } => {
            // The default expression may reference other variables.
            walk(default_value, out);
        }

        NodeKind::Goto { target } => {
            if !push_coderef_target(target, (node.location.start, node.location.end), out) {
                walk(target, out);
            }
        }

        NodeKind::Unary { op, operand } if op == "\\" => {
            if !push_coderef_target(operand, (node.location.start, node.location.end), out) {
                walk(operand, out);
            }
        }

        NodeKind::Variable { sigil, name } => {
            push_variable_like_ref(node, sigil, name, out);
        }

        NodeKind::Typeglob { name } => {
            // Dynamic typeglob `*{$expr}`: the parser preserves the brace
            // syntax verbatim in the name field (e.g. `name = "{$var}"`).
            // Such names do not correspond to any static symbol in the Perl
            // symbol table and must not be emitted as a concrete reference —
            // downstream providers (goto-def, rename) would otherwise treat
            // `{$var}` as a real symbol name.  Skip and return silently.
            if is_dynamic_typeglob_name(name) {
                return;
            }
            let (package_qualifier, bare_name, qualified_name) = split_qualified_name(name);
            out.push(SymbolRef {
                kind: SymbolRefKind::TypeglobReference,
                name: bare_name,
                qualified_name,
                sigil: Some("*".to_string()),
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: Some((node.location.start, node.location.end)),
            });
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

        NodeKind::MethodCall { object, method, args } => {
            let (package_qualifier, qualified_name, kind) = static_method_target(object, method)
                .map(|(package, qualified)| {
                    (Some(package), qualified, SymbolRefKind::StaticMethodCall)
                })
                .unwrap_or_else(|| (None, method.clone(), SymbolRefKind::MethodCall));

            out.push(SymbolRef {
                kind,
                name: method.clone(),
                qualified_name,
                sigil: None,
                package_qualifier,
                full_span: (node.location.start, node.location.end),
                anchor_span: None,
            });

            walk(object, out);
            for arg in args {
                walk(arg, out);
            }
        }

        _ => {
            node.for_each_child(|child| walk(child, out));
        }
    }
}

fn static_method_target(object: &Node, method: &str) -> Option<(String, String)> {
    let NodeKind::Identifier { name } = &object.kind else {
        return None;
    };
    if name.is_empty() || method.is_empty() {
        return None;
    }
    Some((name.clone(), format!("{name}::{method}")))
}

fn push_variable_like_ref(node: &Node, sigil: &str, name: &str, out: &mut Vec<SymbolRef>) {
    let kind = match sigil {
        "&" => SymbolRefKind::CoderefReference,
        "*" => SymbolRefKind::TypeglobReference,
        _ => {
            let Some(var_kind) = var_kind_from_sigil(sigil) else {
                return;
            };
            SymbolRefKind::Variable(var_kind)
        }
    };
    let (package_qualifier, bare_name, qualified_name) = split_qualified_name(name);
    out.push(SymbolRef {
        kind,
        name: bare_name,
        qualified_name,
        sigil: Some(sigil.to_string()),
        package_qualifier,
        full_span: (node.location.start, node.location.end),
        anchor_span: Some((node.location.start, node.location.end)),
    });
}

fn push_coderef_target(node: &Node, full_span: (usize, usize), out: &mut Vec<SymbolRef>) -> bool {
    let Some(name) = coderef_target_name(node) else {
        return false;
    };
    let (package_qualifier, bare_name, qualified_name) = split_qualified_name(name);
    out.push(SymbolRef {
        kind: SymbolRefKind::CoderefReference,
        name: bare_name,
        qualified_name,
        sigil: Some("&".to_string()),
        package_qualifier,
        full_span,
        anchor_span: Some((node.location.start, node.location.end)),
    });
    true
}

fn coderef_target_name(node: &Node) -> Option<&str> {
    match &node.kind {
        NodeKind::Variable { sigil, name } if sigil == "&" => Some(name),
        // The parser lowers source forms like `\&foo` and `goto &foo` into a
        // zero-argument FunctionCall target whose span still covers the leading
        // ampersand. Keep ordinary `\foo()` / `goto foo()` as call expressions.
        NodeKind::FunctionCall { name, args }
            if args.is_empty() && has_parser_ampersand_span(node, name) =>
        {
            Some(name)
        }
        _ => None,
    }
}

fn has_parser_ampersand_span(node: &Node, name: &str) -> bool {
    node.location.end.saturating_sub(node.location.start) == name.len() + 1
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
        _ => None,
    }
}

/// Return `true` when a typeglob name is dynamic and cannot be statically
/// resolved.  Dynamic names start with `{`, as the parser preserves the
/// brace syntax verbatim (e.g. `*{$var}` → `name = "{$var}"`).
fn is_dynamic_typeglob_name(name: &str) -> bool {
    name.starts_with('{')
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_ast::{Node, NodeKind, SourceLocation};

    fn loc(start: usize, end: usize) -> SourceLocation {
        SourceLocation { start, end }
    }

    /// Regression test: `*{$var}` used to produce a SymbolRef with a literal-
    /// brace name such as `"{$var}"`, which is not a real Perl symbol.
    ///
    /// After the fix the dynamic form must be silently skipped — no SymbolRef
    /// with a brace-containing name should be emitted.
    #[test]
    fn dynamic_typeglob_variable_expr_is_not_emitted_as_static_symbol()
    -> Result<(), Box<dyn std::error::Error>> {
        // The parser encodes `*{$var}` as NodeKind::Typeglob { name: "{$var}" }.
        // Before the fix this would emit SymbolRef { name: "{$var}", kind: TypeglobReference }.
        let node = Node::new(NodeKind::Typeglob { name: "{$var}".to_string() }, loc(0, 8));
        let program = Node::new(NodeKind::Program { statements: vec![node] }, loc(0, 8));

        let refs = extract_symbol_refs(&program);

        let brace_refs: Vec<_> = refs.iter().filter(|r| r.name.starts_with('{')).collect();
        assert!(
            brace_refs.is_empty(),
            "dynamic typeglob *{{...}} must not produce a TypeglobReference \
             with a literal-brace name; got: {brace_refs:?}"
        );
        Ok(())
    }

    /// Edge case: `*{}` (empty braces) is also a dynamic form — do not emit.
    #[test]
    fn dynamic_typeglob_empty_braces_is_not_emitted() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::Typeglob { name: "{}".to_string() }, loc(0, 3));
        let program = Node::new(NodeKind::Program { statements: vec![node] }, loc(0, 3));

        let refs = extract_symbol_refs(&program);

        assert!(
            refs.is_empty(),
            "dynamic typeglob *{{}} must not produce any SymbolRef; got: {refs:?}"
        );
        Ok(())
    }

    /// Sanity: static typeglob `*foo` must still be emitted after the fix.
    #[test]
    fn static_typeglob_is_still_emitted() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc(0, 4));
        let program = Node::new(NodeKind::Program { statements: vec![node] }, loc(0, 4));

        let refs = extract_symbol_refs(&program);

        assert_eq!(refs.len(), 1, "static typeglob *foo must still produce a SymbolRef");
        assert_eq!(refs[0].kind, SymbolRefKind::TypeglobReference);
        assert_eq!(refs[0].name, "foo");
        assert_eq!(refs[0].sigil.as_deref(), Some("*"));
        Ok(())
    }

    /// Edge case: `*{"literal"}` — string-literal dynamic form — also a brace
    /// name and must not be emitted as a static symbol.
    #[test]
    fn dynamic_typeglob_string_literal_is_not_emitted() -> Result<(), Box<dyn std::error::Error>> {
        // The parser may encode *{"name"} as Typeglob { name: "{\"name\"}" }
        // (brace-delimited string).  Same dynamic-boundary rule applies.
        let node = Node::new(NodeKind::Typeglob { name: "{\"name\"}".to_string() }, loc(0, 10));
        let program = Node::new(NodeKind::Program { statements: vec![node] }, loc(0, 10));

        let refs = extract_symbol_refs(&program);

        let brace_refs: Vec<_> = refs.iter().filter(|r| r.name.starts_with('{')).collect();
        assert!(
            brace_refs.is_empty(),
            "dynamic typeglob *{{\"literal\"}} must not produce a literal-brace SymbolRef; \
             got: {brace_refs:?}"
        );
        Ok(())
    }
}
