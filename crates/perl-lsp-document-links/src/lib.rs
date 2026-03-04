//! Document links provider for LSP protocol compatibility.
//!
//! This crate provides document link detection for Perl source files,
//! identifying `use`, `require` module statements, and file includes.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_import::{ModuleImportKind, parse_module_import_head};
use serde_json::{Value, json};
use url::Url;

/// Computes document links for a given Perl document.
///
/// This function scans the text for `use` and `require` statements and creates
/// document links for them. Links are returned with a `data` field containing
/// metadata for deferred resolution via `documentLink/resolve`.
#[must_use]
pub fn compute_links(uri: &str, text: &str, _roots: &[Url]) -> Vec<Value> {
    let mut out = Vec::new();

    for (i, line) in text.lines().enumerate() {
        if let Some(import) = parse_module_import_head(line) {
            match import.kind {
                ModuleImportKind::Use => {
                    if !is_pragma(import.token)
                        && let Some(link) = make_deferred_module_link(
                            uri,
                            i as u32,
                            import.token,
                            import.token_start as u32,
                            import.token_end as u32,
                        )
                    {
                        out.push(link);
                    }
                }
                ModuleImportKind::Require => {
                    if !import.token.starts_with('"')
                        && !import.token.starts_with('\'')
                        && import.token.contains("::")
                        && !is_pragma(import.token)
                        && let Some(link) = make_deferred_module_link(
                            uri,
                            i as u32,
                            import.token,
                            import.token_start as u32,
                            import.token_end as u32,
                        )
                    {
                        out.push(link);
                    }
                }
                ModuleImportKind::UseParent | ModuleImportKind::UseBase => {}
            }
        }

        if let Some(idx) = line.find("require ") {
            let rest = &line[idx + 8..];
            if let Some(start) = rest.find('"').or_else(|| rest.find('\'')) {
                let quote_char = match rest.get(start..).and_then(|s| s.chars().next()) {
                    Some(c) => c,
                    None => continue,
                };
                let s = start + 1;
                if let Some(end) = rest[s..].find(quote_char) {
                    let req = &rest[s..s + end];
                    let col_start = (idx + 8 + start + 1) as u32;
                    let col_end = (idx + 8 + start + 1 + end) as u32;
                    out.push(json!({
                        "range": {
                            "start": {"line": i as u32, "character": col_start},
                            "end":   {"line": i as u32, "character": col_end}
                        },
                        "tooltip": format!("Open {}", req),
                        "data": {
                            "type": "file",
                            "path": req,
                            "baseUri": uri
                        }
                    }));
                }
            }
        }
    }
    out
}

fn make_deferred_module_link(
    uri: &str,
    line: u32,
    module: &str,
    col_start: u32,
    col_end: u32,
) -> Option<Value> {
    if module.is_empty() || col_start >= col_end {
        return None;
    }

    Some(json!({
        "range": {
            "start": {"line": line, "character": col_start},
            "end": {"line": line, "character": col_end}
        },
        "tooltip": format!("Open {}", module),
        "data": {
            "type": "module",
            "module": module,
            "baseUri": uri
        }
    }))
}

fn is_pragma(pkg: &str) -> bool {
    matches!(
        pkg,
        "strict"
            | "warnings"
            | "utf8"
            | "bytes"
            | "integer"
            | "feature"
            | "constant"
            | "lib"
            | "vars"
            | "subs"
            | "overload"
            | "parent"
            | "base"
            | "fields"
            | "if"
            | "attributes"
            | "autouse"
            | "autodie"
            | "bigint"
            | "bignum"
            | "bigrat"
            | "blib"
            | "charnames"
            | "diagnostics"
            | "encoding"
            | "filetest"
            | "locale"
            | "open"
            | "ops"
            | "re"
            | "sigtrap"
            | "sort"
            | "threads"
            | "vmsish"
    )
}

#[cfg(test)]
mod tests {
    use super::compute_links;
    use serde_json::Value;

    #[test]
    fn emits_module_link_for_use_statement() {
        let links = compute_links("file:///workspace/test.pl", "use Foo::Bar;\n", &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("Foo::Bar"));
        }
    }

    #[test]
    fn emits_module_link_for_module_form_require_statement() {
        let links = compute_links("file:///workspace/test.pl", "require Foo::Bar;\n", &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("Foo::Bar"));
        }
    }

    #[test]
    fn does_not_emit_module_link_for_use_parent_statement() {
        let links = compute_links("file:///workspace/test.pl", "use parent 'Foo::Bar';\n", &[]);
        assert!(links.is_empty());
    }
}
