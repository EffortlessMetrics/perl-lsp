//! Document links provider for LSP protocol compatibility.
//!
//! This module provides document link detection for Perl source files,
//! identifying `use`, `require` module statements, and file includes.

use serde_json::{Value, json};
use url::Url;

/// Computes document links for a given Perl document.
///
/// This function scans the text for `use` and `require` statements and creates
/// document links for them. Links are returned with a `data` field containing
/// metadata for deferred resolution via `documentLink/resolve`.
///
/// # Arguments
///
/// * `uri` - The URI of the document being processed.
/// * `text` - The content of the document.
/// * `roots` - A slice of workspace root URLs to resolve modules against.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing a document link.
pub fn compute_links(uri: &str, text: &str, _roots: &[Url]) -> Vec<Value> {
    let mut out = Vec::new();

    for (i, line) in text.lines().enumerate() {
        // "use Foo::Bar;" — defer resolution to documentLink/resolve
        if let Some(rest) = line.trim().strip_prefix("use ")
            && let Some(pkg) = rest.split_whitespace().next()
        {
            let pkg = pkg.trim_end_matches(';');
            // Skip pragmas and core modules
            if !is_pragma(pkg) {
                // Defer expensive resolution - use data field
                if let Some(link) = make_deferred_module_link(uri, i as u32, line, pkg) {
                    out.push(link);
                }
            }
        }

        // "require Module::Name" (module form)
        if let Some(rest) = line.trim().strip_prefix("require ")
            && let Some(pkg) = rest.split_whitespace().next()
        {
            let pkg = pkg.trim_end_matches(';');
            // Check if it's a module name (not a quoted file path)
            if !pkg.starts_with('"')
                && !pkg.starts_with('\'')
                && pkg.contains("::")
                && !is_pragma(pkg)
                && let Some(link) = make_deferred_module_link(uri, i as u32, line, pkg)
            {
                out.push(link);
            }
        }

        // naive "require 'Foo/Bar.pm';" or require "Foo/Bar.pm";
        if let Some(idx) = line.find("require ") {
            let rest = &line[idx + 8..];
            if let Some(start) = rest.find('"').or_else(|| rest.find('\'')) {
                // Safety: find returns byte offset, use get() for safe char access
                let quote_char = match rest.get(start..).and_then(|s| s.chars().next()) {
                    Some(c) => c,
                    None => continue, // Skip if invalid offset
                };
                let s = start + 1;
                if let Some(end) = rest[s..].find(quote_char) {
                    let req = &rest[s..s + end];
                    // Defer file resolution to documentLink/resolve
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

/// Create a document link with deferred target resolution
///
/// Returns a link structure with a `data` field that will be used
/// by `documentLink/resolve` to compute the actual target URI.
fn make_deferred_module_link(uri: &str, line: u32, line_text: &str, module: &str) -> Option<Value> {
    // Find the module name position in the line
    if let Some(start) = line_text.find(module) {
        let col_start = start as u32;
        let col_end = (start + module.len()) as u32;

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
    } else {
        None
    }
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

#[allow(dead_code)] // Reserved for future document link resolution
fn resolve_pkg(pkg: &str, roots: &[Url]) -> Option<String> {
    let rel = pkg.replace("::", "/") + ".pm";
    // Try each workspace root
    if let Some(base) = roots.first() {
        let mut u = base.clone();
        let mut p = u.path().to_string();
        if !p.ends_with('/') {
            p.push('/');
        }
        // Check common Perl lib paths - return first match
        if let Some(lib_dir) = ["lib/", "blib/lib/", ""].first() {
            let full_path = format!("{}{}{}", p, lib_dir, rel);
            u.set_path(&full_path);
            // In real implementation, check if file exists
            // For now, just return first possibility
            return Some(u.to_string());
        }
    }
    None
}

#[allow(dead_code)] // Reserved for future document link resolution
fn resolve_file(path: &str, roots: &[Url]) -> Option<String> {
    // Try to resolve relative to workspace roots - return first match
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

#[allow(dead_code)] // Reserved for future document link resolution
fn make_link(_src: &str, line: u32, line_text: &str, pkg: &str, target: String) -> Option<Value> {
    // Find the package name in the line to get exact column positions
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
