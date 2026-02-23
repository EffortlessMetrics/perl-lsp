//! Perl module name/path conversion helpers.
//!
//! This crate provides a small, focused API for converting between canonical
//! Perl module names (for example, `Foo::Bar`) and module file paths
//! (for example, `Foo/Bar.pm`).

/// Convert a module name into a relative Perl module path.
///
/// # Examples
///
/// ```
/// use perl_module_path::module_name_to_path;
///
/// assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
/// assert_eq!(module_name_to_path("strict"), "strict.pm");
/// ```
#[must_use]
pub fn module_name_to_path(module_name: &str) -> String {
    format!("{}.pm", module_name.replace("::", "/"))
}

/// Convert a module path/key into a module name.
///
/// Handles both `/` and `\\` separators and strips `.pm`/`.pl` suffixes.
///
/// # Examples
///
/// ```
/// use perl_module_path::module_path_to_name;
///
/// assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
/// assert_eq!(module_path_to_name(r"Foo\Bar.pm"), "Foo::Bar");
/// assert_eq!(module_path_to_name("script.pl"), "script");
/// ```
#[must_use]
pub fn module_path_to_name(module_path: &str) -> String {
    let normalized = module_path.replace('\\', "/");
    let without_ext = strip_perl_extension(&normalized);
    without_ext.replace('/', "::")
}

/// Convert a filesystem source path into a likely module name.
///
/// This is intended for file-rename workflows where a concrete source path
/// needs to map back to the module import name. It follows these rules:
///
/// 1. Strip `.pm` or `.pl` suffix
/// 2. If a `lib/` segment exists, use everything after the last `lib/`
/// 3. Otherwise, fall back to the file stem
///
/// # Examples
///
/// ```
/// use perl_module_path::file_path_to_module_name;
///
/// assert_eq!(file_path_to_module_name("/workspace/lib/Foo/Bar.pm"), "Foo::Bar");
/// assert_eq!(file_path_to_module_name("/workspace/script.pl"), "script");
/// assert_eq!(file_path_to_module_name(r"C:\workspace\lib\Foo\Bar.pm"), "Foo::Bar");
/// ```
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

fn strip_perl_extension(path: &str) -> &str {
    if let Some(stripped) = path.strip_suffix(".pm") {
        stripped
    } else if let Some(stripped) = path.strip_suffix(".pl") {
        stripped
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::{file_path_to_module_name, module_name_to_path, module_path_to_name};

    #[test]
    fn converts_module_name_to_path() {
        assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
        assert_eq!(module_name_to_path("App::Config::Loader"), "App/Config/Loader.pm");
    }

    #[test]
    fn converts_module_path_to_name() {
        assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
        assert_eq!(module_path_to_name("lib/Foo/Bar.pm"), "lib::Foo::Bar");
    }

    #[test]
    fn converts_windows_module_path_to_name() {
        assert_eq!(module_path_to_name(r"Foo\Bar.pm"), "Foo::Bar");
        assert_eq!(module_path_to_name(r"lib\Foo\Bar.pm"), "lib::Foo::Bar");
    }

    #[test]
    fn strips_perl_extensions() {
        assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
        assert_eq!(module_path_to_name("script.pl"), "script");
    }

    #[test]
    fn round_trips_common_module_name() {
        let module = "MyApp::Service::Email";
        let path = module_name_to_path(module);
        assert_eq!(module_path_to_name(&path), module);
    }

    #[test]
    fn converts_filesystem_path_with_lib_segment_to_module_name() {
        assert_eq!(file_path_to_module_name("/workspace/lib/Foo/Bar.pm"), "Foo::Bar");
        assert_eq!(file_path_to_module_name("lib/My/App.pm"), "My::App");
    }

    #[test]
    fn converts_windows_filesystem_path_with_lib_segment_to_module_name() {
        assert_eq!(file_path_to_module_name(r"C:\workspace\lib\Foo\Bar.pm"), "Foo::Bar");
    }

    #[test]
    fn falls_back_to_file_stem_when_lib_segment_missing() {
        assert_eq!(file_path_to_module_name("/workspace/scripts/sync_worker.pl"), "sync_worker");
        assert_eq!(file_path_to_module_name("MyModule.pm"), "MyModule");
    }
}
