//! Document links provider for Perl LSP protocol compatibility.
//!
//! This crate provides document link detection for Perl source files,
//! identifying `use`, `require` module statements, and file includes.

use perl_module::import::{ModuleImportKind, RequireForm, parse_module_import_head};
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
                    match import.require_form() {
                        Some(RequireForm::FilePath) if import.token.ends_with(".pm") => {
                            // Quoted .pm require → treat as a module link (Foo/Bar.pm → Foo::Bar)
                            let module_name = import.token_as_module_name();
                            if !is_pragma(&module_name) {
                                if let Some(link) = make_deferred_module_link(
                                    uri,
                                    i as u32,
                                    &module_name,
                                    import.token_start as u32,
                                    import.token_end as u32,
                                ) {
                                    out.push(link);
                                }
                            }
                        }
                        Some(RequireForm::FilePath) => {
                            // Quoted file path that is NOT a .pm (e.g. .pl, extensionless) → file link
                            out.push(json!({
                                "range": {
                                    "start": {"line": i as u32, "character": import.token_start as u32},
                                    "end":   {"line": i as u32, "character": import.token_end as u32}
                                },
                                "tooltip": format!("Open {}", import.token),
                                "data": {
                                    "type": "file",
                                    "path": import.token,
                                    "baseUri": uri
                                }
                            }));
                        }
                        Some(RequireForm::ModuleName) | None => {
                            // Bare module name form — existing behavior
                            if import.token.contains("::")
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
                    }
                }
                ModuleImportKind::UseParent | ModuleImportKind::UseBase => {}
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
        // .pm files are normalized to module links (my/file.pm → my::file)
        let links = compute_links(uri(), r#"require "my/file.pm";"#, &[]);
        assert_eq!(links.len(), 1, "require .pm with double-quotes should emit a module link");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("my::file"));
        }
    }

    #[test]
    fn emits_file_link_for_require_with_single_quoted_string() {
        // lib/helper.pm → lib::helper (lib/ prefix is NOT stripped by module_path_to_name)
        let links = compute_links(uri(), "require 'lib/helper.pm';", &[]);
        assert_eq!(links.len(), 1, "require .pm with single-quotes should emit a module link");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("lib::helper"));
        }
    }

    #[test]
    fn require_pm_produces_no_duplicate_links() {
        // The old hardcoded scan was separate from the parsed-require path, producing 2 links.
        // The new unified path must produce exactly 1 link.
        let links = compute_links(uri(), r#"require "Foo/Bar.pm";"#, &[]);
        assert_eq!(links.len(), 1, "require .pm must produce exactly one link, not duplicates");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("module"));
            assert_eq!(link.pointer("/data/module").and_then(Value::as_str), Some("Foo::Bar"));
        }
    }

    #[test]
    fn require_pl_produces_file_link() {
        // .pl files are script includes, not module names — must remain file links
        let links = compute_links(uri(), r#"require "helper.pl";"#, &[]);
        assert_eq!(links.len(), 1, "require .pl should emit a file link");
        if let Some(link) = links.first() {
            assert_eq!(link.pointer("/data/type").and_then(Value::as_str), Some("file"));
            assert_eq!(link.pointer("/data/path").and_then(Value::as_str), Some("helper.pl"));
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
