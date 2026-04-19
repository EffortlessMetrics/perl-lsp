//! Perl module name/path conversion helpers.
//!
//! Provides a small, focused API for converting between canonical Perl module
//! names (e.g., `Foo::Bar`) and module file paths (e.g., `Foo/Bar.pm`).

use std::borrow::Cow;

/// Normalize legacy package separator `'` to canonical `::`.
#[must_use]
pub fn normalize_package_separator(module_name: &str) -> Cow<'_, str> {
    crate::name::normalize_package_separator(module_name)
}

/// Convert a module name into a relative Perl module path.
#[must_use]
pub fn module_name_to_path(module_name: &str) -> String {
    let normalized = normalize_package_separator(module_name);
    format!("{}.pm", normalized.replace("::", "/"))
}

/// Convert a module path/key into a module name.
///
/// Handles both `/` and `\\` separators and strips `.pm`/`.pl` suffixes.
#[must_use]
pub fn module_path_to_name(module_path: &str) -> String {
    let normalized = module_path.replace('\\', "/");
    let without_ext = strip_perl_extension(&normalized);
    without_ext.replace('/', "::")
}

/// Convert a filesystem source path into a likely module name.
///
/// Rules:
/// 1. Strip `.pm` or `.pl` suffix
/// 2. If a `lib/` segment exists, use everything after the last `lib/`
/// 3. Otherwise, fall back to the file stem
#[must_use]
pub fn file_path_to_module_name(file_path: &str) -> String {
    let normalized = file_path.replace('\\', "/");
    let without_ext = strip_perl_extension(&normalized);

    if let Some(relative_module_path) = strip_to_lib_relative_path(without_ext) {
        return module_path_to_name(relative_module_path);
    }

    without_ext
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(without_ext)
        .to_string()
}

fn strip_to_lib_relative_path(path: &str) -> Option<&str> {
    if let Some(stripped) = path.strip_prefix("lib/") {
        return Some(stripped);
    }

    path.rfind("/lib/")
        .map(|lib_idx| &path[lib_idx + "/lib/".len()..])
}

fn strip_perl_extension(path: &str) -> &str {
    if let Some(stripped) = path.strip_suffix(".pm") {
        stripped
    } else if let Some(stripped) = path.strip_suffix(".pl") {
        stripped
    } else {
        path
    }
}
