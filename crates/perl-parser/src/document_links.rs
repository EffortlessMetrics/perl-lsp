// crates/perl-parser/src/document_links.rs
use serde_json::{json, Value};
use url::Url;

pub fn compute_links(uri: &str, text: &str, roots: &[Url]) -> Vec<Value> {
    let mut out = Vec::new();
    
    for (i, line) in text.lines().enumerate() {
        // "use Foo::Bar;" — resolve Foo::Bar -> Foo/Bar.pm
        if let Some(rest) = line.trim().strip_prefix("use ") {
            if let Some(pkg) = rest.split_whitespace().next() {
                let pkg = pkg.trim_end_matches(';');
                // Skip pragmas and core modules
                if !is_pragma(pkg) {
                    if let Some(target) = resolve_pkg(pkg, roots) {
                        if let Some(link) = make_link(uri, i as u32, line, pkg, target) {
                            out.push(link);
                        }
                    } else {
                        // Link to metacpan for CPAN modules
                        let target = format!("https://metacpan.org/pod/{}", pkg);
                        if let Some(link) = make_link(uri, i as u32, line, pkg, target) {
                            out.push(link);
                        }
                    }
                }
            }
        }
        
        // naive "require 'Foo/Bar.pm';" or require "Foo/Bar.pm";
        if let Some(idx) = line.find("require ") {
            let rest = &line[idx+8..];
            if let Some(start) = rest.find('"').or_else(|| rest.find('\'')) {
                let quote_char = rest.chars().nth(start).unwrap();
                let s = start + 1;
                if let Some(end) = rest[s..].find(quote_char) {
                    let req = &rest[s..s+end];
                    // Try to resolve as file path
                    if let Some(target) = resolve_file(req, roots) {
                        let col_start = (idx + 8 + start + 1) as u32;
                        let col_end = (idx + 8 + start + 1 + end) as u32;
                        out.push(json!({
                            "range": { 
                                "start": {"line": i as u32, "character": col_start},
                                "end":   {"line": i as u32, "character": col_end} 
                            },
                            "target": target
                        }));
                    }
                }
            }
        }
    }
    out
}

fn is_pragma(pkg: &str) -> bool {
    matches!(pkg, "strict" | "warnings" | "utf8" | "bytes" | "integer" | 
             "feature" | "constant" | "lib" | "vars" | "subs" | "overload" |
             "parent" | "base" | "fields" | "if" | "attributes" | "autouse" |
             "autodie" | "bigint" | "bignum" | "bigrat" | "blib" | "charnames" |
             "diagnostics" | "encoding" | "filetest" | "locale" | "open" | 
             "ops" | "re" | "sigtrap" | "sort" | "threads" | "vmsish")
}

fn resolve_pkg(pkg: &str, roots: &[Url]) -> Option<String> {
    let rel = pkg.replace("::", "/") + ".pm";
    // Try each workspace root
    for base in roots {
        let mut u = base.clone();
        let mut p = u.path().to_string();
        if !p.ends_with('/') { 
            p.push('/'); 
        }
        // Check common Perl lib paths
        for lib_dir in &["lib/", "blib/lib/", ""] {
            let full_path = format!("{}{}{}", p, lib_dir, rel);
            u.set_path(&full_path);
            // In real implementation, check if file exists
            // For now, just return first possibility
            return Some(u.to_string());
        }
    }
    None
}

fn resolve_file(path: &str, roots: &[Url]) -> Option<String> {
    // Try to resolve relative to workspace roots
    for base in roots {
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