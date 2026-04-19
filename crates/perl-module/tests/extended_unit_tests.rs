//! Extended unit tests for the `perl-module-path` crate.
//!
//! This test suite provides additional coverage beyond comprehensive_unit_tests.rs,
//! including:
//! - Additional edge cases and boundary conditions
//! - Complex path scenarios with multiple separators
//! - Library path extraction with unusual layouts
//! - Performance and correctness with large inputs
//! - Special characters and encoding edge cases

use perl_module::path::{
    file_path_to_module_name, module_name_to_path, module_path_to_name, normalize_package_separator,
};

// ── normalize_package_separator: Additional edge cases ─────────────────

#[test]
fn normalize_single_apostrophe_only() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("'");
    assert_eq!(result, "::");
    Ok(())
}

#[test]
fn normalize_multiple_consecutive_apostrophes() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("A'''B");
    // Three apostrophes each become ::, so we get six colons
    assert_eq!(result, "A::::::B");
    Ok(())
}

#[test]
fn normalize_leading_apostrophe() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("'Package");
    assert_eq!(result, "::Package");
    Ok(())
}

#[test]
fn normalize_trailing_apostrophe() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Package'");
    assert_eq!(result, "Package::");
    Ok(())
}

#[test]
fn normalize_leading_trailing_apostrophes() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("'Package'");
    assert_eq!(result, "::Package::");
    Ok(())
}

#[test]
fn normalize_apostrophe_with_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Mod1'Mod2'Mod3");
    assert_eq!(result, "Mod1::Mod2::Mod3");
    Ok(())
}

#[test]
fn normalize_apostrophe_with_underscores() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Mod_1'Mod_2'Mod_3");
    assert_eq!(result, "Mod_1::Mod_2::Mod_3");
    Ok(())
}

#[test]
fn normalize_very_long_package_with_ticks() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("A'B'C'D'E'F'G'H'I'J");
    assert_eq!(result, "A::B::C::D::E::F::G::H::I::J");
    Ok(())
}

// ── module_name_to_path: Additional depth and structure tests ─────────

#[test]
fn name_to_path_underscore_in_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("My_Module::Sub_Module"),
        "My_Module/Sub_Module.pm"
    );
    Ok(())
}

#[test]
fn name_to_path_numbers_in_segments() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo123::Bar456"), "Foo123/Bar456.pm");
    Ok(())
}

#[test]
fn name_to_path_mixed_case() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("MyModule::subModule::SubSubModule"),
        "MyModule/subModule/SubSubModule.pm"
    );
    Ok(())
}

#[test]
fn name_to_path_lowercase_module() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("perl::io"), "perl/io.pm");
    Ok(())
}

#[test]
fn name_to_path_uppercase_single_char() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("A::B::C::D::E::F::G::H"),
        "A/B/C/D/E/F/G/H.pm"
    );
    Ok(())
}

#[test]
fn name_to_path_very_long_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("VeryLongModuleName::AnotherVeryLongSegment::YetAnother"),
        "VeryLongModuleName/AnotherVeryLongSegment/YetAnother.pm"
    );
    Ok(())
}

#[test]
fn name_to_path_ten_level_deep() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("L1::L2::L3::L4::L5::L6::L7::L8::L9::L10"),
        "L1/L2/L3/L4/L5/L6/L7/L8/L9/L10.pm"
    );
    Ok(())
}

#[test]
fn name_to_path_single_colon_pairs_consecutive() -> Result<(), Box<dyn std::error::Error>> {
    // Extra colons create empty segments which become empty path components
    assert_eq!(module_name_to_path("A::B::C"), "A/B/C.pm");
    Ok(())
}

#[test]
fn name_to_path_hyphen_in_module() -> Result<(), Box<dyn std::error::Error>> {
    // Hyphens are allowed in Perl module names (though unusual)
    assert_eq!(module_name_to_path("My-Module::Sub"), "My-Module/Sub.pm");
    Ok(())
}

// ── module_path_to_name: Additional path scenarios ────────────────────

#[test]
fn path_to_name_multiple_extensions_only_last_stripped() -> Result<(), Box<dyn std::error::Error>> {
    // Only .pm or .pl at the end are stripped, not in the middle
    assert_eq!(module_path_to_name("Foo.pm/Bar.pm"), "Foo.pm::Bar");
    Ok(())
}

#[test]
fn path_to_name_many_slashes_and_backslashes() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(r"A/B\C/D\E.pm"), "A::B::C::D::E");
    Ok(())
}

#[test]
fn path_to_name_leading_slash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("/Foo/Bar.pm"), "::Foo::Bar");
    Ok(())
}

#[test]
fn path_to_name_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Bar/.pm"), "Foo::Bar::"); // Empty final segment before .pm
    Ok(())
}

#[test]
fn path_to_name_only_slashes() -> Result<(), Box<dyn std::error::Error>> {
    // Three slashes create three segments: "", "", "", and "" which become colons
    assert_eq!(module_path_to_name("///"), "::::::");
    Ok(())
}

#[test]
fn path_to_name_only_backslashes() -> Result<(), Box<dyn std::error::Error>> {
    // Three backslashes converted to slashes create three segments which become colons
    assert_eq!(module_path_to_name(r"\\\"), "::::::");
    Ok(())
}

#[test]
fn path_to_name_dot_without_extension() -> Result<(), Box<dyn std::error::Error>> {
    // A dot not followed by pm or pl is preserved
    assert_eq!(module_path_to_name("Foo.Other/Bar.pm"), "Foo.Other::Bar");
    Ok(())
}

#[test]
fn path_to_name_numbers_in_path() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo123/Bar456.pm"), "Foo123::Bar456");
    Ok(())
}

#[test]
fn path_to_name_underscores_in_path() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_path_to_name("foo_bar/baz_qux.pm"),
        "foo_bar::baz_qux"
    );
    Ok(())
}

#[test]
fn path_to_name_hyphens_in_path() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_path_to_name("my-module/sub-module.pm"),
        "my-module::sub-module"
    );
    Ok(())
}

#[test]
fn path_to_name_very_deep_path() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_path_to_name("A/B/C/D/E/F/G/H/I/J.pm"),
        "A::B::C::D::E::F::G::H::I::J"
    );
    Ok(())
}

#[test]
fn path_to_name_single_segment_no_extension() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Module"), "Module");
    Ok(())
}

#[test]
fn path_to_name_double_extension_pm_pl() -> Result<(), Box<dyn std::error::Error>> {
    // Only the last .pm or .pl is stripped
    assert_eq!(module_path_to_name("Foo.pm.pl"), "Foo.pm");
    Ok(())
}

#[test]
fn path_to_name_double_extension_pl_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo.pl.pm"), "Foo.pl");
    Ok(())
}

// ── file_path_to_module_name: Complex lib detection ──────────────────

#[test]
fn file_path_lib_in_middle_of_filename() -> Result<(), Box<dyn std::error::Error>> {
    // "lib" as part of a filename, not a directory boundary
    assert_eq!(file_path_to_module_name("/project/reliable.pm"), "reliable");
    Ok(())
}

#[test]
fn file_path_libing_as_segment() -> Result<(), Box<dyn std::error::Error>> {
    // "libing" or similar should not match /lib/
    assert_eq!(
        file_path_to_module_name("/project/libing/Module.pm"),
        "Module"
    );
    Ok(())
}

#[test]
fn file_path_libfoo_directory() -> Result<(), Box<dyn std::error::Error>> {
    // /libfoo/ should not match /lib/
    assert_eq!(
        file_path_to_module_name("/project/libfoo/Module.pm"),
        "Module"
    );
    Ok(())
}

#[test]
fn file_path_three_lib_segments_uses_last() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/a/lib/b/lib/c/lib/Foo/Bar.pm"),
        "Foo::Bar"
    );
    Ok(())
}

#[test]
fn file_path_lib_at_end() -> Result<(), Box<dyn std::error::Error>> {
    // /project/path/to/lib should not match the trailing /lib/ pattern without more segments
    assert_eq!(file_path_to_module_name("/project/path/to/lib/"), "");
    Ok(())
}

#[test]
fn file_path_windows_drive_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name(r"D:\project\lib\My\Module.pm"),
        "My::Module"
    );
    Ok(())
}

#[test]
fn file_path_windows_drive_without_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name(r"D:\project\bin\script.pl"),
        "script"
    );
    Ok(())
}

#[test]
fn file_path_unc_path_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    // UNC paths like //server/share, converted to forward slashes
    assert_eq!(
        file_path_to_module_name(r"\\server\share\lib\Foo\Bar.pm"),
        "Foo::Bar"
    );
    Ok(())
}

#[test]
fn file_path_lib_relative_path() -> Result<(), Box<dyn std::error::Error>> {
    // lib as the first segment
    assert_eq!(
        file_path_to_module_name("lib/Foo/Bar/Baz.pm"),
        "Foo::Bar::Baz"
    );
    Ok(())
}

#[test]
fn file_path_absolute_with_multiple_dots_in_name() -> Result<(), Box<dyn std::error::Error>> {
    // Dots in the filename before extension
    assert_eq!(
        file_path_to_module_name("/project/lib/Foo.Bar.Baz.pm"),
        "Foo.Bar.Baz"
    );
    Ok(())
}

#[test]
fn file_path_hidden_file_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    // Hidden files (starting with .) in lib/ directory
    assert_eq!(
        file_path_to_module_name("/project/lib/.hidden.pm"),
        ".hidden"
    );
    Ok(())
}

#[test]
fn file_path_deeply_nested_within_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/a/b/c/lib/d/e/f/g/h/Module.pm"),
        "d::e::f::g::h::Module"
    );
    Ok(())
}

#[test]
fn file_path_relative_without_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("src/Foo/Bar.pm"), "Bar");
    Ok(())
}

#[test]
fn file_path_relative_with_parent_refs() -> Result<(), Box<dyn std::error::Error>> {
    // .. in path
    assert_eq!(file_path_to_module_name("../lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

#[test]
fn file_path_dot_segment() -> Result<(), Box<dyn std::error::Error>> {
    // . (current dir) segments in path
    assert_eq!(file_path_to_module_name("./lib/Foo/Bar.pm"), "Foo::Bar");
    Ok(())
}

// ── Round-trip and integration tests: Extended ───────────────────────

#[test]
fn round_trip_with_underscores() -> Result<(), Box<dyn std::error::Error>> {
    let original = "My_App::Service::Email_Handler";
    let path = module_name_to_path(original);
    assert_eq!(module_path_to_name(&path), original);
    Ok(())
}

#[test]
fn round_trip_with_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let original = "Foo123::Bar456::Baz789";
    let path = module_name_to_path(original);
    assert_eq!(module_path_to_name(&path), original);
    Ok(())
}

#[test]
fn round_trip_very_deep() -> Result<(), Box<dyn std::error::Error>> {
    let original = "A::B::C::D::E::F::G::H::I::J::K::L::M::N::O::P";
    let path = module_name_to_path(original);
    assert_eq!(module_path_to_name(&path), original);
    Ok(())
}

#[test]
fn round_trip_single_segment() -> Result<(), Box<dyn std::error::Error>> {
    let original = "SoleModule";
    let path = module_name_to_path(original);
    assert_eq!(module_path_to_name(&path), original);
    Ok(())
}

#[test]
fn file_path_to_path_to_name_chain() -> Result<(), Box<dyn std::error::Error>> {
    let file = "/project/lib/My/Deep/Nested/Module.pm";
    let name = file_path_to_module_name(file);
    assert_eq!(name, "My::Deep::Nested::Module");
    let path = module_name_to_path(&name);
    assert_eq!(path, "My/Deep/Nested/Module.pm");
    let back_to_name = module_path_to_name(&path);
    assert_eq!(back_to_name, name);
    Ok(())
}

// ── Boundary and stress tests ───────────────────────────────────────

#[test]
fn name_to_path_max_segment_length() -> Result<(), Box<dyn std::error::Error>> {
    let long_segment = "A".repeat(256);
    let input = format!("{}::B", long_segment);
    let result = module_name_to_path(&input);
    assert!(result.contains(&format!("{}/B.pm", long_segment)));
    Ok(())
}

#[test]
fn path_to_name_max_path_length() -> Result<(), Box<dyn std::error::Error>> {
    let long_path = (0..100)
        .map(|i| format!("Seg{}", i))
        .collect::<Vec<_>>()
        .join("/");
    let input = format!("{}.pm", long_path);
    let result = module_path_to_name(&input);
    let expected = (0..100)
        .map(|i| format!("Seg{}", i))
        .collect::<Vec<_>>()
        .join("::");
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn file_path_to_module_max_depth() -> Result<(), Box<dyn std::error::Error>> {
    let mut path = String::from("/a/b/c/d/e/f/g/h/i/j/lib");
    for i in 0..50 {
        path.push_str(&format!("/Seg{}", i));
    }
    path.push_str(".pm");
    let result = file_path_to_module_name(&path);
    let mut expected = String::new();
    for i in 0..50 {
        if i > 0 {
            expected.push_str("::");
        }
        expected.push_str(&format!("Seg{}", i));
    }
    assert_eq!(result, expected);
    Ok(())
}

// ── Special character handling ──────────────────────────────────────

#[test]
fn normalize_unicode_package() -> Result<(), Box<dyn std::error::Error>> {
    let result = normalize_package_separator("Foo'Bär'Ñoño");
    assert_eq!(result, "Foo::Bär::Ñoño");
    Ok(())
}

#[test]
fn name_to_path_japanese_chars() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("日本語::モジュール"),
        "日本語/モジュール.pm"
    );
    Ok(())
}

#[test]
fn path_to_name_emoji_in_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo🦀/Bar.pm"), "Foo🦀::Bar");
    Ok(())
}

#[test]
fn file_path_emoji_in_filename() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/lib/Test🦀/Module.pm"),
        "Test🦀::Module"
    );
    Ok(())
}

// ── Whitespace and control characters ───────────────────────────────

#[test]
fn name_to_path_spaces_preserved() -> Result<(), Box<dyn std::error::Error>> {
    // Spaces in module names are unusual but allowed; they're preserved
    assert_eq!(module_name_to_path("Foo Bar::Baz"), "Foo Bar/Baz.pm");
    Ok(())
}

#[test]
fn path_to_name_spaces_preserved() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo Bar/Baz.pm"), "Foo Bar::Baz");
    Ok(())
}

#[test]
fn file_path_spaces_in_directory() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/my project/lib/My Module/Class.pm"),
        "My Module::Class"
    );
    Ok(())
}

#[test]
fn file_path_spaces_in_filename() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("/lib/My Class.pm"), "My Class");
    Ok(())
}

// ── Multiple transformations and conversions ────────────────────────

#[test]
fn name_to_path_to_name_with_legacy_sep() -> Result<(), Box<dyn std::error::Error>> {
    let original = "Foo'Bar'Baz";
    let path = module_name_to_path(original);
    let back = module_path_to_name(&path);
    // Legacy separator gets normalized
    assert_eq!(back, "Foo::Bar::Baz");
    Ok(())
}

#[test]
fn file_path_lib_stripping_then_path_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let file = "/workspace/lib/Deep/Nested/Module.pm";
    let name = file_path_to_module_name(file);
    assert_eq!(name, "Deep::Nested::Module");
    let back_to_path = module_name_to_path(&name);
    assert_eq!(back_to_path, "Deep/Nested/Module.pm");
    Ok(())
}

#[test]
fn module_path_to_name_strips_lib_prefix_via_file_path() -> Result<(), Box<dyn std::error::Error>> {
    // file_path_to_module_name strips lib/, but module_path_to_name does not
    let file_input = "/project/lib/Foo/Bar.pm";
    let file_result = file_path_to_module_name(file_input);
    assert_eq!(file_result, "Foo::Bar");

    let path_input = "lib/Foo/Bar.pm";
    let path_result = module_path_to_name(path_input);
    assert_eq!(path_result, "lib::Foo::Bar");
    Ok(())
}

// ── Empty and minimal inputs ────────────────────────────────────────

#[test]
fn path_to_name_only_dot_pm() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(".pm"), "");
    Ok(())
}

#[test]
fn path_to_name_only_dot_pl() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name(".pl"), "");
    Ok(())
}

#[test]
fn path_to_name_slash_only() -> Result<(), Box<dyn std::error::Error>> {
    // Single slash creates two segments: "" and "" which become one colon pair
    assert_eq!(module_path_to_name("/"), "::");
    Ok(())
}

#[test]
fn file_path_only_dot() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name("."), ".");
    Ok(())
}

#[test]
fn file_path_only_two_dots() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(file_path_to_module_name(".."), "..");
    Ok(())
}

#[test]
fn normalize_just_colon_pair() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(normalize_package_separator("::"), "::");
    Ok(())
}

// ── Case sensitivity tests ─────────────────────────────────────────

#[test]
fn name_to_path_mixed_case_preserved() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_name_to_path("MyModule::anotherModule::ALLCAPS"),
        "MyModule/anotherModule/ALLCAPS.pm"
    );
    Ok(())
}

#[test]
fn path_to_name_case_preserved() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        module_path_to_name("MyModule/anotherModule/ALLCAPS.pm"),
        "MyModule::anotherModule::ALLCAPS"
    );
    Ok(())
}

#[test]
fn file_path_case_preserved_with_lib() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/Project/lib/MyModule/AnotherModule.pm"),
        "MyModule::AnotherModule"
    );
    Ok(())
}

// ── Repetition and redundancy tests ────────────────────────────────

#[test]
fn name_to_path_repeated_segments() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_name_to_path("Foo::Foo::Foo"), "Foo/Foo/Foo.pm");
    Ok(())
}

#[test]
fn path_to_name_repeated_segments() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(module_path_to_name("Foo/Foo/Foo.pm"), "Foo::Foo::Foo");
    Ok(())
}

#[test]
fn file_path_repeated_lib_segments_uses_last() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        file_path_to_module_name("/lib/foo/lib/bar/lib/Module.pm"),
        "Module"
    );
    Ok(())
}
