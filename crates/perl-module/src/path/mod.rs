//! Perl module name/path conversion helpers.
//!
//! Provides a small, focused API for converting between canonical Perl module
//! names (e.g., `Foo::Bar`) and module file paths (e.g., `Foo/Bar.pm`).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

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

/// Normalize Windows paths to their long-form representation.
///
/// On non-Windows platforms this is a no-op and returns `path.to_path_buf()`.
#[must_use]
pub fn canonicalize_path_long_form(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        windows_long_path(path).unwrap_or_else(|| path.to_path_buf())
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
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

    path.rfind("/lib/").map(|lib_idx| &path[lib_idx + "/lib/".len()..])
}

#[cfg(windows)]
fn windows_long_path(path: &Path) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    use windows_sys::Win32::Storage::FileSystem::GetLongPathNameW;

    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // SAFETY: `path_wide` is NUL-terminated and pointers are valid for this call.
    let required_len = unsafe { GetLongPathNameW(path_wide.as_ptr(), std::ptr::null_mut(), 0) };
    if required_len == 0 {
        return None;
    }

    let mut output = vec![0_u16; required_len as usize];
    // SAFETY: input is NUL-terminated and `output` is allocated with `required_len` UTF-16 units.
    let written_len =
        unsafe { GetLongPathNameW(path_wide.as_ptr(), output.as_mut_ptr(), required_len) };
    if written_len == 0 {
        return None;
    }

    Some(PathBuf::from(OsString::from_wide(&output[..written_len as usize])))
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
