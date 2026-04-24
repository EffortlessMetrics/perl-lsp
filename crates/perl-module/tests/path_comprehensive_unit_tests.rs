//! Comprehensive unit tests for the `perl-module-path` crate.
//!
//! Covers all four public functions:
//! - `normalize_package_separator`
//! - `module_name_to_path`
//! - `module_path_to_name`
//! - `file_path_to_module_name`

use perl_module::path::{
    canonicalize_path_long_form, file_path_to_module_name, module_name_to_path,
    module_path_to_name, normalize_package_separator,
};
use std::path::PathBuf;

// ── normalize_package_separator ─────────────────────────────────────

#[test]
fn normalize_noop_when_already_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo::Bar");
    assert_eq!(result, "Foo::Bar");
    Ok(())
}

#[test]
fn normalize_single_legacy_tick() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bar");
    assert_eq!(result, "Foo::Bar");
    Ok(())
}

#[test]
fn normalize_multiple_legacy_ticks() -> Result<(), Box<dyn std::error::Error>> {
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
fn normalize_bare_module_name() -> Result<(), Box<dyn std::error::Error>> {
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
fn normalize_returns_borrowed_when_no_tick() -> Result<(), Box<dyn std::error::Error>> {
    let input = "Foo::Bar";
    let result = normalize_package_separator(input);
    // When no tick is present the result borrows from the input.
    assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    Ok(())
}

#[test]
fn normalize_returns_owned_when_tick_present() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bar");
    assert!(matches!(result, std::borrow::Cow::Owned(_)));
    Ok(())
}

// ── module_name_to_path ─────────────────────────────────────────────

#[test]
fn name_to_path_simple_two_part() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo::Bar"), "Foo/Bar.pm");
    Ok(())
}

#[test]
fn name_to_path_three_part() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("App::Config::Loader"), "App/Config/Loader.pm");
    Ok(())
}

#[test]
fn name_to_path_bare_pragma() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("strict"), "strict.pm");
    Ok(())
}

#[test]
fn name_to_path_legacy_tick_separator() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Legacy'Package"), "Legacy/Package.pm");
    Ok(())
}

#[test]
fn name_to_path_mixed_tick_and_colons() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("A::B'C::D"), "A/B/C/D.pm");
    Ok(())
}

#[test]
fn name_to_path_single_char_segments() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("A::B::C"), "A/B/C.pm");
    Ok(())
}

#[test]
fn name_to_path_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("A::B::C::D::E::F"), "A/B/C/D/E/F.pm");
    Ok(())
}

#[test]
fn name_to_path_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path(""), ".pm");
    Ok(())
}

// ── module_path_to_name ─────────────────────────────────────────────

#[test]
fn path_to_name_forward_slash_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn path_to_name_backslash_pm() -> Result<(), Box<dyn std::error::Error>> {
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
fn path_to_name_deeply_nested_forward() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("A/B/C/D/E.pm"), "A::B::C::D::E");
    Ok(())
}

#[test]
fn path_to_name_deeply_nested_backslash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(r"A\B\C\D\E.pm"), "A::B::C::D::E");
    Ok(())
}

#[test]
fn path_to_name_mixed_separators() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(r"A/B\C/D.pm"), "A::B::C::D");
    Ok(())
}

#[test]
fn path_to_name_bare_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Module.pm"), "Module");
    Ok(())
}

#[test]
fn path_to_name_bare_pl() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("runner.pl"), "runner");
    Ok(())
}

#[test]
fn path_to_name_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(""), "");
    Ok(())
}

#[test]
fn path_to_name_preserves_lib_prefix_in_path() -> Result<(), Box<dyn std::error::Error>> {
    // module_path_to_name does NOT strip lib/ — that is file_path_to_module_name's job
    assert_eq!(module_path_to_name("lib/Foo/Bar.pm"), "lib::Foo::Bar");
    Ok(())
}

// ── file_path_to_module_name ────────────────────────────────────────

#[test]
fn file_path_strips_lib_prefix_unix() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/workspace/lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_strips_lib_prefix_windows() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"C:\workspace\lib\Foo\Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_lib_at_start() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("lib/My/App.pm"), "My::App");
    Ok(())
}

#[test]
fn file_path_nested_lib_takes_last() -> Result<(), Box<dyn std::error::Error>> {
    // When multiple lib/ segments exist, the *last* one is used.
    assert_eq!(file_path_to_module_name("/a/lib/outer/lib/Inner/Mod.pm"), "Inner::Mod");
    Ok(())
}

#[test]
fn file_path_falls_back_to_file_stem_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/workspace/scripts/sync_worker.pl"), "sync_worker");
    Ok(())
}

#[test]
fn file_path_bare_pm_file_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("MyModule.pm"), "MyModule");
    Ok(())
}

#[test]
fn file_path_bare_pl_file_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("tool.pl"), "tool");
    Ok(())
}

#[test]
fn file_path_no_extension_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/project/lib/Foo/Bar"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_no_extension_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/project/bin/script"), "script");
    Ok(())
}

#[test]
fn file_path_windows_backslash_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(r"C:\scripts\runner.pl"), "runner");
    Ok(())
}

#[test]
fn file_path_deeply_nested_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/opt/perl/local/lib/App/Service/Worker/Queue.pm"),
        "App::Service::Worker::Queue"
    );
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn canonicalize_long_form_is_noop_on_non_windows() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/workspace/lib/Foo/Bar.pm");
    let canonicalized = canonicalize_path_long_form(&input);
    assert_eq!(canonicalized, input);
    Ok(())
}

// ── Round-trip / integration tests ──────────────────────────────────

#[test]
fn round_trip_simple_module() -> Result<(), Box<dyn std::error::Error>> {
    let module = "MyApp::Service::Email";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_bare_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let module = "warnings";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    let module = "A::B::C::D::E";
    let path = module_name_to_path(module);
    assert_eq!(module_path_to_name(&path), module);
    Ok(())
}

#[test]
fn round_trip_legacy_tick_normalized() -> Result<(), Box<dyn std::error::Error>> {
    // Legacy tick converts to :: during name_to_path, so round-trip yields canonical form.
    let path = module_name_to_path("Foo'Bar");
    assert_eq!(module_path_to_name(&path), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_to_name_then_back_to_path() -> Result<(), Box<dyn std::error::Error>> {
    let name = file_path_to_module_name("/project/lib/My/App/Config.pm");
    assert_eq!(name, "My::App::Config");
    let path = module_name_to_path(&name);
    assert_eq!(path, "My/App/Config.pm");
    Ok(())
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn name_to_path_unicode_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo::Bär"), "Foo/Bär.pm");
    Ok(())
}

#[test]
fn path_to_name_unicode_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Bär.pm"), "Foo::Bär");
    Ok(())
}

#[test]
fn file_path_trailing_slash_no_lib() -> Result<(), Box<dyn std::error::Error>> {
    // Trailing slash produces an empty last segment; filter skips it, then
    // unwrap_or falls back to the full (extension-stripped) string.
    let result = file_path_to_module_name("/project/bin/");
    assert_eq!(result, "/project/bin/");
    Ok(())
}

#[test]
fn normalize_consecutive_ticks() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("A''B");
    assert_eq!(result, "A::::B");
    Ok(())
}

#[test]
fn name_to_path_with_consecutive_colons_beyond_two() -> Result<(), Box<dyn std::error::Error>> {
    // Unusual but deterministic: extra colons produce empty segments.
    let result = module_name_to_path("A::::B");
    assert_eq!(result, "A//B.pm");
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
fn file_path_lib_segment_in_filename_not_directory() -> Result<(), Box<dyn std::error::Error>> {
    // "lib" appearing as part of a filename, not as a directory segment.
    let result = file_path_to_module_name("/workspace/libutils.pm");
    assert_eq!(result, "libutils");
    Ok(())
}

#[test]
fn file_path_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = file_path_to_module_name("");
    assert_eq!(result, "");
    Ok(())
}
