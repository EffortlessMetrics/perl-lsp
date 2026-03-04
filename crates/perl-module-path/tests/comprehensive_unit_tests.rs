//! Comprehensive unit tests for `perl-module-path`.
//!
//! Covers all four public functions plus internal helpers exposed through them:
//! - `normalize_package_separator`
//! - `module_name_to_path`
//! - `module_path_to_name`
//! - `file_path_to_module_name`

use perl_module_path::{
    file_path_to_module_name, module_name_to_path, module_path_to_name, normalize_package_separator,
};

// ── normalize_package_separator ─────────────────────────────────────────

#[test]
fn normalize_legacy_single_quote_separator() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bar");
    assert_eq!(result, "Foo::Bar");
    Ok(())
}

#[test]
fn normalize_already_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo::Bar");
    assert_eq!(result, "Foo::Bar");
    Ok(())
}

#[test]
fn normalize_multiple_legacy_separators() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("A'B'C'D");
    assert_eq!(result, "A::B::C::D");
    Ok(())
}

#[test]
fn normalize_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo::Bar'Baz");
    assert_eq!(result, "Foo::Bar::Baz");
    Ok(())
}

#[test]
fn normalize_no_separator() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("strict");
    assert_eq!(result, "strict");
    Ok(())
}

#[test]
fn normalize_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("");
    assert_eq!(result, "");
    Ok(())
}

#[test]
fn normalize_returns_borrowed_when_no_change() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo::Bar";
    let result = normalize_package_separator(input);
    // When no legacy separator is present, should return Cow::Borrowed
    assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn normalize_returns_owned_when_changed() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bar");
    assert!(matches!(result, std::borrow::Cow::Owned(_)));
    Ok(())
}

// ── module_name_to_path ─────────────────────────────────────────────────

#[test]
fn name_to_path_simple_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
    Ok(())
}

#[test]
fn name_to_path_single_word() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("strict"), "strict.pm");
    Ok(())
}

#[test]
fn name_to_path_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("App::Config::Loader::YAML"), "App/Config/Loader/YAML.pm");
    Ok(())
}

#[test]
fn name_to_path_legacy_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Legacy'Package"), "Legacy/Package.pm");
    Ok(())
}

#[test]
fn name_to_path_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path(""), ".pm");
    Ok(())
}

#[test]
fn name_to_path_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo::Bar'Baz"), "Foo/Bar/Baz.pm");
    Ok(())
}

#[test]
fn name_to_path_common_cpan_modules() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("File::Spec"), "File/Spec.pm");
    assert_eq!(module_name_to_path("LWP::UserAgent"), "LWP/UserAgent.pm");
    assert_eq!(module_name_to_path("Test::More"), "Test/More.pm");
    assert_eq!(module_name_to_path("Moose::Role"), "Moose/Role.pm");
    assert_eq!(module_name_to_path("DBI"), "DBI.pm");
    Ok(())
}

// ── module_path_to_name ─────────────────────────────────────────────────

#[test]
fn path_to_name_forward_slash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn path_to_name_backslash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(r"Foo\Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn path_to_name_pl_extension() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("script.pl"), "script");
    Ok(())
}

#[test]
fn path_to_name_no_extension() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Bar"), "Foo::Bar");
    Ok(())
}

#[test]
fn path_to_name_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("App/Config/Loader/YAML.pm"), "App::Config::Loader::YAML");
    Ok(())
}

#[test]
fn path_to_name_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(""), "");
    Ok(())
}

#[test]
fn path_to_name_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(r"Foo/Bar\Baz.pm"), "Foo::Bar::Baz");
    Ok(())
}

#[test]
fn path_to_name_only_extension_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(".pm"), "");
    Ok(())
}

#[test]
fn path_to_name_only_extension_pl() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(".pl"), "");
    Ok(())
}

#[test]
fn path_to_name_includes_lib_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // module_path_to_name does NOT strip lib/ — that's file_path_to_module_name's job
    assert_eq!(module_path_to_name("lib/Foo/Bar.pm"), "lib::Foo::Bar");
    Ok(())
}

// ── file_path_to_module_name ────────────────────────────────────────────

#[test]
fn file_path_with_lib_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/workspace/lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_lib_at_start() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("lib/My/App.pm"), "My::App");
    Ok(())
}

#[test]
fn file_path_windows_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"C:\workspace\lib\Foo\Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_fallback_to_stem_pl() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/workspace/scripts/sync_worker.pl"), "sync_worker");
    Ok(())
}

#[test]
fn file_path_fallback_to_stem_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("MyModule.pm"), "MyModule");
    Ok(())
}

#[test]
fn file_path_no_extension_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/tmp/myscript"), "myscript");
    Ok(())
}

#[test]
fn file_path_multiple_lib_segments_uses_last() -> Result<(), Box<dyn std::error::Error>> {
    // When multiple lib/ segments exist, rfind picks the last one
    assert_eq!(file_path_to_module_name("/home/lib/vendor/lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_deeply_nested_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/opt/project/lib/App/Config/Loader/YAML.pm"),
        "App::Config::Loader::YAML"
    );
    Ok(())
}

#[test]
fn file_path_windows_backslash_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"C:\scripts\worker.pl"), "worker");
    Ok(())
}

#[test]
fn file_path_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(""), "");
    Ok(())
}

#[test]
fn file_path_bare_lib_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // "lib/" at start — strip_prefix("lib/") fires
    assert_eq!(file_path_to_module_name("lib/Foo.pm"), "Foo");
    Ok(())
}

#[test]
fn file_path_lib_with_single_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/project/lib/DBI.pm"), "DBI");
    Ok(())
}

// ── round-trip tests ────────────────────────────────────────────────────

#[test]
fn round_trip_simple() -> Result<(), Box<dyn std::error::Error>> {
    let module = "Foo::Bar";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_single_word() -> Result<(), Box<dyn std::error::Error>> {
    let module = "strict";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    let module = "My::Very::Deep::Package::Name";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_legacy_normalizes_on_forward_pass() -> Result<(), Box<dyn std::error::Error>> {
    // Legacy separator normalizes to :: in the name→path→name cycle
    let path = module_name_to_path("Foo'Bar");
    assert_eq!(path, "Foo/Bar.pm");
    assert_eq!(module_path_to_name(&path), "Foo::Bar");
    Ok(())
}

#[test]
fn round_trip_common_cpan_modules() -> Result<(), Box<dyn std::error::Error>> {
    let modules = [
        "File::Spec",
        "LWP::UserAgent",
        "Test::More",
        "Moose::Role",
        "DBI",
        "Carp",
        "POSIX",
        "Scalar::Util",
        "List::Util",
        "JSON::XS",
    ];
    for module in modules {
        let path = module_name_to_path(module);
        assert_eq!(module_path_to_name(&path), module, "round-trip failed for {module}");
    }
    Ok(())
}

// ── edge cases ──────────────────────────────────────────────────────────

#[test]
fn name_to_path_unicode_module_name() -> Result<(), Box<dyn std::error::Error>> {
    // Perl technically supports unicode identifiers
    assert_eq!(module_name_to_path("Ünïcödé::Módule"), "Ünïcödé/Módule.pm");
    Ok(())
}

#[test]
fn path_to_name_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    // Unusual input — trailing slash produces trailing ::
    let result = module_path_to_name("Foo/Bar/");
    assert_eq!(result, "Foo::Bar::");
    Ok(())
}

#[test]
fn file_path_lib_directory_only() -> Result<(), Box<dyn std::error::Error>> {
    // Edge: path is exactly "lib/" with nothing after
    let result = file_path_to_module_name("lib/");
    assert_eq!(result, "");
    Ok(())
}

#[test]
fn normalize_consecutive_legacy_separators() -> Result<(), Box<dyn std::error::Error>> {
    // Two consecutive quotes — unusual but should still normalize
    let result = normalize_package_separator("A''B");
    assert_eq!(result, "A::::B");
    Ok(())
}

#[test]
fn name_to_path_single_char_segments() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("A::B::C"), "A/B/C.pm");
    Ok(())
}

#[test]
fn file_path_relative_with_dot_segments() -> Result<(), Box<dyn std::error::Error>> {
    // Dotted path segments — lib/ detection still works
    assert_eq!(file_path_to_module_name("./lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_lib_inside_module_name() -> Result<(), Box<dyn std::error::Error>> {
    // "lib" appears as part of a module name, not as a directory
    // No /lib/ directory separator, so fallback kicks in
    assert_eq!(file_path_to_module_name("libfoo.pm"), "libfoo");
    Ok(())
}

#[test]
fn path_to_name_pm_in_middle_of_name() -> Result<(), Box<dyn std::error::Error>> {
    // ".pm" only stripped from the end — interior ".pm" is kept verbatim
    assert_eq!(module_path_to_name("Foo.pm/Bar.pm"), "Foo.pm::Bar");
    Ok(())
}

#[test]
fn file_path_windows_drive_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"D:\projects\myapp\lib\Net\HTTP.pm"), "Net::HTTP");
    Ok(())
}

#[test]
fn file_path_unc_path_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"\\server\share\lib\Foo\Bar.pm"), "Foo::Bar");
    Ok(())
}
