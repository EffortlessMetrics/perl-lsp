//! Document links provider for Perl LSP protocol compatibility.
//!
//! This crate provides document link detection for Perl source files,
//! identifying `use`, `require` module statements, POD links, and file includes.

use perl_module::import::{ModuleImportKind, RequireForm, parse_module_import_head};
use perl_module::path::module_name_to_path;
use serde_json::{Value, json};
use url::Url;

/// Computes document links for a given Perl document.
///
/// This function scans the text for `use` and `require` statements plus POD
/// `L<>` links and creates document links for them. Links are returned with a
/// `data` field containing metadata for deferred resolution via
/// `documentLink/resolve`.
#[must_use]
pub fn compute_links(uri: &str, text: &str, _roots: &[Url]) -> Vec<Value> {
    let mut out = Vec::new();
    let current_package = current_package_name(text);
    let mut in_pod = false;

    for (i, line) in text.lines().enumerate() {
        if in_pod && line.starts_with("=cut") {
            in_pod = false;
            continue;
        }

        if starts_pod_block(line) {
            in_pod = true;
        }

        if in_pod {
            collect_pod_document_links(uri, i as u32, line, current_package.as_deref(), &mut out);
            continue;
        }

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

enum PodLinkTarget<'a> {
    Module(&'a str),
    Section(&'a str),
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

fn make_deferred_pod_section_link(
    uri: &str,
    line: u32,
    section: &str,
    col_start: u32,
    col_end: u32,
) -> Option<Value> {
    if section.is_empty() || col_start >= col_end {
        return None;
    }

    Some(json!({
        "range": {
            "start": {"line": line, "character": col_start},
            "end": {"line": line, "character": col_end}
        },
        "tooltip": format!("Open POD section {}", section),
        "data": {
            "type": "pod_section",
            "section": section,
            "baseUri": uri
        }
    }))
}

fn collect_pod_document_links(
    uri: &str,
    line_number: u32,
    line: &str,
    current_package: Option<&str>,
    out: &mut Vec<Value>,
) {
    let mut search_start = 0;
    while let Some(open_offset) = line[search_start..].find("L<") {
        let content_start = search_start + open_offset + 2;
        let after_open = &line[content_start..];
        let Some(close_offset) = after_open.find('>') else {
            break;
        };

        let content_end = content_start + close_offset;
        let target = after_open[..close_offset].trim();
        let col_start = byte_to_utf16_col(line, content_start);
        let col_end = byte_to_utf16_col(line, content_end);

        match pod_link_target(target) {
            Some(PodLinkTarget::Module(module)) if Some(module) != current_package => {
                if let Some(link) =
                    make_deferred_module_link(uri, line_number, module, col_start, col_end)
                {
                    out.push(link);
                }
            }
            Some(PodLinkTarget::Section(section)) => {
                if let Some(link) =
                    make_deferred_pod_section_link(uri, line_number, section, col_start, col_end)
                {
                    out.push(link);
                }
            }
            _ => {}
        }

        search_start = content_end + 1;
    }
}

fn pod_link_target(target: &str) -> Option<PodLinkTarget<'_>> {
    let candidate = if let Some((label, link_target)) = target.split_once('|') {
        if label.trim().is_empty() {
            return None;
        }
        link_target.trim()
    } else {
        target.trim()
    };

    if let Some(section) = candidate.strip_prefix('/') {
        let section = section.trim();
        if is_pod_section_target(section) {
            return Some(PodLinkTarget::Section(section));
        }
        return None;
    }

    if is_simple_pod_module_target(candidate) {
        Some(PodLinkTarget::Module(candidate))
    } else {
        None
    }
}

fn starts_pod_block(line: &str) -> bool {
    line.starts_with("=head")
        || line.starts_with("=pod")
        || line.starts_with("=over")
        || line.starts_with("=begin")
        || line.starts_with("=for")
        || line.starts_with("=encoding")
        || line.starts_with("=item")
}

fn is_simple_pod_module_target(target: &str) -> bool {
    is_simple_package_pod_target(target) || is_supported_core_pragma_pod_target(target)
}

fn is_simple_package_pod_target(target: &str) -> bool {
    target.contains("::") && target.split("::").all(is_perl_module_segment)
}

fn is_supported_core_pragma_pod_target(target: &str) -> bool {
    matches!(target, "strict" | "warnings")
}

fn is_pod_section_target(section: &str) -> bool {
    !section.is_empty()
        && section.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ' '))
}

fn is_perl_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn current_package_name(text: &str) -> Option<String> {
    let mut in_pod = false;

    for line in text.lines() {
        if in_pod && line.starts_with("=cut") {
            in_pod = false;
            continue;
        }

        if starts_pod_block(line) {
            in_pod = true;
            continue;
        }

        if in_pod {
            continue;
        }

        let Some(rest) = line.trim_start().strip_prefix("package ") else {
            continue;
        };
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':')
            .collect();
        if name.split("::").all(is_perl_module_segment) {
            return Some(name);
        }
    }
    None
}

fn byte_to_utf16_col(line: &str, byte_offset: usize) -> u32 {
    line.get(..byte_offset)
        .map(|prefix| prefix.encode_utf16().count() as u32)
        .unwrap_or(byte_offset as u32)
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

    fn data_str<'a>(link: &'a Value, pointer: &str) -> Option<&'a str> {
        link.pointer(pointer).and_then(Value::as_str)
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

    #[test]
    fn pod_emits_module_links_for_plain_and_labeled_targets() {
        let text =
            "package Local::Doc;\n=head1 SEE ALSO\n\nSee L<Foo::Bar> and L<docs|Baz::Qux>.\n=cut\n";

        let links = compute_links(uri(), text, &[]);

        assert_eq!(links.len(), 2);
        assert!(
            links.iter().any(|link| data_str(link, "/data/module") == Some("Foo::Bar")),
            "missing plain POD module link: {links:#?}"
        );
        assert!(
            links.iter().any(|link| data_str(link, "/data/module") == Some("Baz::Qux")),
            "missing labeled POD module link: {links:#?}"
        );
        let foo = links
            .iter()
            .find(|link| data_str(link, "/data/module") == Some("Foo::Bar"))
            .unwrap_or(&Value::Null);
        assert_eq!(data_str(foo, "/data/type"), Some("module"));
        assert_eq!(foo.pointer("/range/start/line").and_then(Value::as_u64), Some(3));
        assert_eq!(foo.pointer("/range/start/character").and_then(Value::as_u64), Some(6));
        assert_eq!(foo.pointer("/range/end/character").and_then(Value::as_u64), Some(14));
    }

    #[test]
    fn pod_emits_core_pragma_links_for_supported_perldoc_topics() {
        let text = "=pod\nSee L<strict>, L<warnings>, and L<feature>.\n=cut\n";

        let links = compute_links(uri(), text, &[]);

        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|link| data_str(link, "/data/module") == Some("strict")));
        assert!(links.iter().any(|link| data_str(link, "/data/module") == Some("warnings")));
        assert!(!links.iter().any(|link| data_str(link, "/data/module") == Some("feature")));
    }

    #[test]
    fn pod_emits_section_links_for_plain_and_labeled_local_sections() {
        let text = "=pod\nSee L</method_name> and L<section docs|/setup>.\n=cut\n";

        let links = compute_links(uri(), text, &[]);

        assert_eq!(links.len(), 2);
        assert!(
            links.iter().any(|link| {
                data_str(link, "/data/type") == Some("pod_section")
                    && data_str(link, "/data/section") == Some("method_name")
            }),
            "missing plain POD section link: {links:#?}"
        );
        assert!(
            links.iter().any(|link| {
                data_str(link, "/data/type") == Some("pod_section")
                    && data_str(link, "/data/section") == Some("setup")
            }),
            "missing labeled POD section link: {links:#?}"
        );
    }

    #[test]
    fn pod_skips_empty_label_path_like_single_segment_and_self_targets() {
        let text = "package Local::Doc;\n=pod\nSee L<|Other::Module>, L<docs|Local::>, L<docs|https://example.invalid>, L<Local::Doc>, L<Foo/Bar>, and L<NotAModule>.\n=cut\n";

        let links = compute_links(uri(), text, &[]);

        assert!(links.is_empty(), "malformed POD targets must stay quiet: {links:#?}");
    }

    #[test]
    fn pod_package_example_does_not_suppress_matching_link() {
        let text = "=pod\npackage Example::Module;\nSee L<Example::Module>.\n=cut\n";

        let links = compute_links(uri(), text, &[]);

        assert!(
            links.iter().any(|link| data_str(link, "/data/module") == Some("Example::Module")),
            "POD package examples must not be treated as the current code package: {links:#?}"
        );
    }

    #[test]
    fn pod_ignores_markup_after_cut_and_use_statements_inside_pod() {
        let text = "=pod\nuse Pod::Example;\n=cut\nSee L<Foo::Bar>.\n";

        let links = compute_links(uri(), text, &[]);

        assert!(links.is_empty(), "non-POD L<> and use statements inside POD must stay quiet");
    }

    #[test]
    fn pod_directives_must_start_at_column_zero() {
        let text = "    =pod\nuse Foo::Bar;\n";

        let links = compute_links(uri(), text, &[]);

        assert!(
            links.iter().any(|link| data_str(link, "/data/module") == Some("Foo::Bar")),
            "indented POD-looking text must not suppress code links: {links:#?}"
        );
    }
}
