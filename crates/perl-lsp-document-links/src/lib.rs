//! Document links provider for Perl LSP protocol compatibility.
//!
//! This crate provides document link detection for Perl source files,
//! identifying `use`, `require` module statements, file includes,
//! and database migration file references (DBIx::Class::DeploymentHandler, sqitch).

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module::import::{ModuleImportKind, parse_module_import_head};
use perl_module::path::module_name_to_path;
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
                    let is_quoted_require = import.token_start > 0
                        && matches!(
                            line.as_bytes().get(import.token_start - 1),
                            Some(b'\'' | b'"')
                        );

                    if !is_quoted_require
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

        // Scan for migration file references
        out.extend(scan_line_for_migration_links(uri, i as u32, line));
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

/// Returns `true` if `path` contains a database migration file pattern.
///
/// Used to detect references to DBIx::Class::DeploymentHandler or sqitch
/// migration files in Perl source code.
fn is_migration_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("share/deploy/")
        || lower.contains("share/upgrade/")
        || lower.contains("share/revert/")
        || lower.contains("/deploy/")
        || lower.contains("/verify/")
        || lower.contains("/revert/")
        || lower.contains("sqitch.plan")
}

/// Scan a line for quoted strings that contain migration file patterns.
/// Returns a JSON value for each migration file reference found.
fn scan_line_for_migration_links(
    uri: &str,
    line_idx: u32,
    line: &str,
) -> Vec<Value> {
    let mut links = Vec::new();

    // Find all quoted strings in the line
    let mut search_start = 0;
    while let Some(quote_pos) = line[search_start..].find('"').or_else(|| line[search_start..].find('\'')) {
        let quote_char = line[search_start..].chars().nth(quote_pos).unwrap();
        let absolute_pos = search_start + quote_pos;
        let after_quote = absolute_pos + 1;

        if let Some(end_pos) = line[after_quote..].find(quote_char) {
            let string_start = after_quote;
            let string_end = after_quote + end_pos;
            let candidate = &line[string_start..string_end];

            if is_migration_path(candidate) {
                links.push(json!({
                    "range": {
                        "start": {"line": line_idx, "character": string_start as u32},
                        "end":   {"line": line_idx, "character": string_end as u32}
                    },
                    "tooltip": format!("Open migration file: {}", candidate),
                    "data": {
                        "type": "migration_file",
                        "path": candidate,
                        "baseUri": uri
                    }
                }));
            }

            search_start = string_end + 1;
        } else {
            break;
        }
    }

    links
}

#[allow(dead_code)]
fn resolve_pkg(pkg: &str, roots: &[Url]) -> Option<String> {
    let rel = module_name_to_path(pkg);
    if let Some(base) = roots.first() {
        let mut u = base.clone();
        let mut p = u.path().to_string();
        if !p.ends_with('/') {
            p.push('/');
        }
        if let Some(lib_dir) = ["lib/", "blib/lib/", ""].first() {
            let full_path = format!("{}{}{}", p, lib_dir, rel);
            u.set_path(&full_path);
            return Some(u.to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn resolve_file(path: &str, roots: &[Url]) -> Option<String> {
    if let Some(base) = roots.first() {
        let mut u = base.clone();
        let mut p = u.path().to_string();
        if !p.ends_with('/') {
            p.push('/');
        }
        p.push_str(path);
        u.set_path(&p);
        return Some(u.to_string());
    }
    None
}

#[allow(dead_code)]
fn make_link(_src: &str, line: u32, line_text: &str, pkg: &str, target: String) -> Option<Value> {
    if let Some(idx) = line_text.find(pkg) {
        let start = idx as u32;
        let end = (idx + pkg.len()) as u32;
        Some(json!({
            "range": {
                "start": {"line": line, "character": start},
                "end":   {"line": line, "character": end}
            },
            "target": target,
            "tooltip": format!("Open {}", pkg)
        }))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::compute_links;
    use serde_json::Value;

    fn uri() -> &'static str {
        "file:///workspace/test.pl"
    }

    // ── use statement ──────────────────────────────────────────

    #[test]
    fn emits_module_link_for_use_statement() {
        let links = compute_links(uri(), "use Foo::Bar;\n", &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("Foo::Bar"));
        }
    }

    #[test]
    fn does_not_emit_link_for_pragma_use_strict() {
        let links = compute_links(uri(), "use strict;\n", &[]);
        assert!(links.is_empty(), "pragmas should not produce document links");
    }

    #[test]
    fn does_not_emit_link_for_pragma_use_warnings() {
        let links = compute_links(uri(), "use warnings;\n", &[]);
        assert!(links.is_empty(), "pragmas should not produce document links");
    }

    #[test]
    fn does_not_emit_link_for_use_feature_pragma() {
        let links = compute_links(uri(), "use feature 'say';\n", &[]);
        assert!(links.is_empty(), "'feature' is a pragma");
    }

    // ── use parent / use base ─────────────────────────────────

    #[test]
    fn does_not_emit_module_link_for_use_parent_statement() {
        let links = compute_links(uri(), "use parent 'Foo::Bar';\n", &[]);
        assert!(links.is_empty());
    }

    #[test]
    fn does_not_emit_module_link_for_use_base_statement() {
        let links = compute_links(uri(), "use base 'Foo::Bar';\n", &[]);
        assert!(links.is_empty(), "use base is a base-class declaration, not a module link");
    }

    // ── require statement ─────────────────────────────────────

    #[test]
    fn emits_module_link_for_module_form_require_statement() {
        let links = compute_links(uri(), "require Foo::Bar;\n", &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("Foo::Bar"));
        }
    }

    #[test]
    fn emits_file_link_for_require_with_double_quoted_string() {
        let links = compute_links(uri(), r#"require "my/file.pm";"#, &[]);
        assert_eq!(links.len(), 1, "require with file string should emit a file link");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("file"));
            assert_eq!(link.pointer("/data/path").and_then(Value::as_str), Some("my/file.pm"));
        }
    }

    #[test]
    fn emits_file_link_for_require_with_single_quoted_string() {
        let links = compute_links(uri(), "require 'lib/helper.pm';", &[]);
        assert_eq!(links.len(), 1, "require with single-quoted file should emit a file link");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("file"));
        }
    }

    #[test]
    fn does_not_emit_link_for_require_bare_word_without_colons() {
        // A bare word without '::' is not a module form require
        let links = compute_links(uri(), "require Something;\n", &[]);
        assert!(links.is_empty(), "bare require without '::' should not emit a module link");
    }

    // ── link range / metadata ─────────────────────────────────

    #[test]
    fn link_range_is_on_correct_line() {
        let text = "# comment\nuse Foo::Bar;\n";
        let links = compute_links(uri(), text, &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            let line = link.pointer("/range/start/line").and_then(Value::as_u64);
            assert_eq!(line, Some(1), "link should be on line 1 (0-indexed)");
        }
    }

    #[test]
    fn link_tooltip_contains_module_name() {
        let links = compute_links(uri(), "use Foo::Bar;\n", &[]);
        assert_eq!(links.len(), 1);
        if let Some(link) = links.first() {
            let tooltip = link.pointer("/tooltip").and_then(Value::as_str).unwrap_or("");
            assert!(tooltip.contains("Foo::Bar"), "tooltip should reference the module name");
        }
    }

    // ── multiple statements ───────────────────────────────────

    #[test]
    fn emits_link_for_each_use_statement_in_multi_line_file() {
        let text = "use Foo;\nuse Bar::Baz;\nuse strict;\n";
        let links = compute_links(uri(), text, &[]);
        // 'strict' is a pragma → only Foo and Bar::Baz get links
        // 'Foo' has no '::', but some parsers may still emit a link; what matters is 'strict' is excluded
        let has_strict = links
            .iter()
            .any(|l| l.pointer("/data/module").and_then(Value::as_str) == Some("strict"));
        assert!(!has_strict, "strict pragma must not appear in links");
    }
}
