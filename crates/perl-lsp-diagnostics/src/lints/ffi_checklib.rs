//! FFI::CheckLib native-library validation hints
//!
//! This is a small, conservative diagnostics pass for the most common
//! `FFI::CheckLib` call shapes:
//!
//! - `find_lib(...)`
//! - `check_lib_or_exit(...)`
//! - fully-qualified `FFI::CheckLib::find_lib(...)`
//!
//! It recognizes literal `lib` / `libpath` arguments and checks for matching
//! library filenames in the explicit search paths plus a small set of common
//! fallback directories.

use std::fs;
use std::path::{Path, PathBuf};

use crate::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;

const CHECKLIB_SUPPORT_MODULES: &[&str] = &["FFI::CheckLib", "FFI::Platypus::Bundle"];
const QUALIFIED_CHECKLIB_CALLS: &[&str] = &[
    "FFI::CheckLib::find_lib",
    "FFI::CheckLib::check_lib_or_exit",
    "FFI::Platypus::Bundle::find_lib",
    "FFI::Platypus::Bundle::check_lib_or_exit",
];

pub fn check_ffi_checklib(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let has_support_module = has_checklib_support_module(node);

    walk_node(node, &mut |n| {
        if let NodeKind::FunctionCall { name, args } = &n.kind
            && is_checklib_call(name, has_support_module)
        {
            check_call(n, args, diagnostics);
        }
    });
}

fn has_checklib_support_module(node: &Node) -> bool {
    let mut found = false;
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind
            && CHECKLIB_SUPPORT_MODULES.contains(&module.as_str())
        {
            found = true;
        }
    });
    found
}

fn is_checklib_call(name: &str, has_support_module: bool) -> bool {
    if QUALIFIED_CHECKLIB_CALLS.contains(&name) {
        return true;
    }

    has_support_module && matches!(name, "find_lib" | "check_lib_or_exit")
}

fn check_call(node: &Node, args: &[Node], diagnostics: &mut Vec<Diagnostic>) {
    let (libs, libpaths) = collect_checklib_arguments(args);
    if libs.is_empty() {
        return;
    }

    let mut search_paths = collect_search_paths(&libpaths);
    if search_paths.is_empty() {
        search_paths = default_search_paths();
    }

    for lib in libs {
        if !library_exists(&lib, &search_paths) {
            diagnostics.push(Diagnostic {
                range: (node.location.start, node.location.end),
                severity: DiagnosticSeverity::Warning,
                code: None,
                message: format!(
                    "FFI::CheckLib could not find native library `{lib}` in the configured search paths"
                ),
                related_information: vec![],
                tags: vec![],
                suggestion: Some(
                    "Add a matching `libpath` entry or install the library development package"
                        .to_string(),
                ),
            });
        }
    }
}

fn collect_checklib_arguments(args: &[Node]) -> (Vec<String>, Vec<String>) {
    let mut libs = Vec::new();
    let mut libpaths = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let key = match literal_text(&args[index]) {
            Some(key) => key,
            None => {
                index += 1;
                continue;
            }
        };

        match key.as_str() {
            "lib" | "libpath" => {
                if index + 1 < args.len() {
                    if key == "lib" {
                        libs.extend(extract_literal_strings(&args[index + 1]));
                    } else {
                        libpaths.extend(extract_literal_strings(&args[index + 1]));
                    }
                    index += 2;
                    continue;
                }
            }
            _ => {}
        }

        index += 1;
    }

    dedup_strings(&mut libs);
    dedup_strings(&mut libpaths);
    (libs, libpaths)
}

fn extract_literal_strings(node: &Node) -> Vec<String> {
    match &node.kind {
        NodeKind::String { value, .. } => literal_text_from_raw(value).into_iter().collect(),
        NodeKind::Identifier { name } => vec![name.clone()],
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().flat_map(extract_literal_strings).collect()
        }
        _ => Vec::new(),
    }
}

fn literal_text(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, .. } => literal_text_from_raw(value),
        NodeKind::Identifier { name } => Some(name.clone()),
        _ => None,
    }
}

fn literal_text_from_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let first = bytes[0];
        let last = bytes[trimmed.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return Some(trimmed[1..trimmed.len() - 1].to_string());
        }
    }

    Some(trimmed.to_string())
}

fn collect_search_paths(libpaths: &[String]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = libpaths.iter().map(PathBuf::from).collect();

    for env_name in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(env_name) {
            paths.extend(std::env::split_paths(&value));
        }
    }

    dedup_paths(&mut paths);
    paths
}

fn default_search_paths() -> Vec<PathBuf> {
    let mut paths = collect_search_paths(&[]);

    #[cfg(target_os = "windows")]
    {
        paths.extend(
            ["C:\\Windows\\System32", "C:\\Windows\\SysWOW64", "C:\\Windows"]
                .iter()
                .map(PathBuf::from),
        );
    }

    #[cfg(target_os = "macos")]
    {
        paths.extend(
            ["/usr/lib", "/usr/local/lib", "/opt/homebrew/lib", "/opt/local/lib"]
                .iter()
                .map(PathBuf::from),
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        paths.extend(common_unix_search_paths().into_iter().map(PathBuf::from));
    }

    dedup_paths(&mut paths);
    paths
}

#[cfg(all(unix, not(target_os = "macos")))]
fn common_unix_search_paths() -> Vec<&'static str> {
    let mut paths =
        vec!["/lib", "/lib64", "/usr/lib", "/usr/lib64", "/usr/local/lib", "/opt/local/lib"];

    #[cfg(target_arch = "x86_64")]
    paths.extend(["/lib/x86_64-linux-gnu", "/usr/lib/x86_64-linux-gnu"]);
    #[cfg(target_arch = "aarch64")]
    paths.extend(["/lib/aarch64-linux-gnu", "/usr/lib/aarch64-linux-gnu"]);
    #[cfg(target_arch = "arm")]
    paths.extend(["/lib/arm-linux-gnueabihf", "/usr/lib/arm-linux-gnueabihf"]);
    #[cfg(target_arch = "x86")]
    paths.extend(["/lib/i386-linux-gnu", "/usr/lib/i386-linux-gnu"]);
    #[cfg(target_arch = "powerpc64")]
    paths.extend(["/lib/powerpc64le-linux-gnu", "/usr/lib/powerpc64le-linux-gnu"]);
    #[cfg(target_arch = "s390x")]
    paths.extend(["/lib/s390x-linux-gnu", "/usr/lib/s390x-linux-gnu"]);

    paths
}

fn library_exists(lib: &str, search_paths: &[PathBuf]) -> bool {
    let candidates = candidate_library_names(lib);
    candidates.iter().any(|candidate| library_exists_anywhere(candidate, search_paths))
}

fn library_exists_anywhere(candidate: &str, search_paths: &[PathBuf]) -> bool {
    let exact = Path::new(candidate);
    if exact.exists() {
        return true;
    }

    search_paths.iter().any(|dir| library_exists_in_dir(dir, candidate))
}

fn library_exists_in_dir(dir: &Path, candidate: &str) -> bool {
    if dir.join(candidate).exists() {
        return true;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        candidate_matches(&name, candidate)
    })
}

fn candidate_matches(file_name: &str, candidate: &str) -> bool {
    if file_name == candidate {
        return true;
    }

    (candidate.ends_with(".so") || candidate.ends_with(".dylib"))
        && file_name.starts_with(candidate)
}

fn candidate_library_names(lib: &str) -> Vec<String> {
    let trimmed = lib.trim().trim_matches(|c| c == '\'' || c == '"');
    let mut candidates = vec![trimmed.to_string()];
    let stem = trimmed.strip_prefix("lib").unwrap_or(trimmed);

    #[cfg(target_os = "windows")]
    {
        candidates.push(format!("{stem}.dll"));
        candidates.push(format!("lib{stem}.dll"));
        candidates.push(format!("{stem}.lib"));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(format!("lib{stem}.dylib"));
        candidates.push(format!("lib{stem}.so"));
        candidates.push(format!("lib{stem}.a"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        candidates.push(format!("lib{stem}.so"));
        candidates.push(format!("lib{stem}.so.1"));
        candidates.push(format!("lib{stem}.a"));
    }

    dedup_strings(&mut candidates);
    candidates
}

fn dedup_strings(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn dedup_paths(values: &mut Vec<PathBuf>) {
    values.sort();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use perl_parser::Parser;
    use perl_tdd_support::must;
    use tempfile::tempdir;

    fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diagnostics = Vec::new();
        check_ffi_checklib(&ast, &mut diagnostics);
        diagnostics
    }

    fn write_library(dir: &Path, lib: &str) {
        for candidate in candidate_library_names(lib) {
            let path = dir.join(candidate);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, b"").unwrap();
        }
    }

    #[test]
    fn missing_library_emits_diagnostic() {
        let diags =
            diagnostics_for("use FFI::CheckLib;\nfind_lib(lib => 'ffi_checklib_missing_3574');\n");
        assert!(
            diags.iter().any(|d| d.message.contains("ffi_checklib_missing_3574")),
            "expected a missing-library diagnostic, got: {diags:?}"
        );
    }

    #[test]
    fn explicit_libpath_suppresses_missing_library() {
        let tempdir = tempdir().unwrap();
        write_library(tempdir.path(), "ffi_checklib_present_3574");

        let source = format!(
            "use FFI::CheckLib;\nfind_lib(lib => 'ffi_checklib_present_3574', libpath => '{}');\n",
            tempdir.path().display()
        );

        let diags = diagnostics_for(&source);
        assert!(diags.is_empty(), "expected no diagnostics for a present library, got: {diags:?}");
    }

    #[test]
    fn array_library_list_reports_only_missing_entries() {
        let tempdir = tempdir().unwrap();
        write_library(tempdir.path(), "ffi_checklib_present_3574");

        let source = format!(
            "use FFI::CheckLib;\ncheck_lib_or_exit(lib => ['ffi_checklib_present_3574', 'ffi_checklib_missing_3574'], libpath => '{}');\n",
            tempdir.path().display()
        );

        let diags = diagnostics_for(&source);
        assert_eq!(diags.len(), 1, "expected one missing library diagnostic, got: {diags:?}");
        assert!(
            diags[0].message.contains("ffi_checklib_missing_3574"),
            "expected the missing library to be named in the diagnostic"
        );
    }

    #[test]
    fn qualified_call_is_detected_without_import() {
        let tempdir = tempdir().unwrap();
        write_library(tempdir.path(), "ffi_checklib_present_3574");

        let source = format!(
            "FFI::CheckLib::find_lib(lib => 'ffi_checklib_present_3574', libpath => '{}');\n",
            tempdir.path().display()
        );

        let diags = diagnostics_for(&source);
        assert!(diags.is_empty(), "qualified FFI::CheckLib call should be handled, got: {diags:?}");
    }

    #[test]
    fn unrelated_qualified_find_lib_is_ignored() {
        let diags = diagnostics_for("Vendor::find_lib(lib => 'ffi_checklib_missing_3574');\n");
        assert!(
            diags.is_empty(),
            "non-FFI qualified find_lib calls should not trigger CheckLib diagnostics, got: {diags:?}"
        );
    }

    #[test]
    fn platypus_bundle_import_is_treated_as_supporting_context() {
        let tempdir = tempdir().unwrap();
        write_library(tempdir.path(), "ffi_checklib_present_3574");

        let source = format!(
            "use FFI::Platypus::Bundle;\nfind_lib(lib => 'ffi_checklib_present_3574', libpath => '{}');\n",
            tempdir.path().display()
        );

        let diags = diagnostics_for(&source);
        assert!(
            diags.is_empty(),
            "FFI::Platypus::Bundle should participate in the same CheckLib hinting, got: {diags:?}"
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn default_search_paths_include_common_unix_multiarch_roots() {
        let search_paths = default_search_paths();

        assert!(
            search_paths.contains(&PathBuf::from("/usr/lib64")),
            "expected default search paths to include /usr/lib64, got: {search_paths:?}"
        );

        #[cfg(target_arch = "x86_64")]
        assert!(
            search_paths.contains(&PathBuf::from("/usr/lib/x86_64-linux-gnu")),
            "expected default search paths to include Debian/Ubuntu x86_64 multiarch dirs"
        );
    }
}
