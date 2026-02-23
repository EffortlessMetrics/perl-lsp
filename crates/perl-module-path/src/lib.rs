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
    use super::{module_name_to_path, module_path_to_name};

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
}
