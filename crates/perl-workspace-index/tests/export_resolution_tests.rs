//! Cross-module export symbol resolution tests for perl-workspace-index
//!
//! Validates that the workspace index can:
//! - Extract exported symbols from `@EXPORT`, `@EXPORT_OK`, and `%EXPORT_TAGS`
//! - Resolve exported symbols via `find_export(module, symbol)`
//! - Return all exports for a module via `get_exports_for_module(module)`
//!
//! AC1: Export extraction from @EXPORT and @EXPORT_OK
//! AC2: Cross-module symbol resolution via export table
//! AC3: use Module () does NOT trigger export resolution (handled at navigation layer)
//! AC4: %EXPORT_TAGS support

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

// ---------------------------------------------------------------------------
// Helper: parse a file:// URL
// ---------------------------------------------------------------------------
fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// ---------------------------------------------------------------------------
// Test: AC1 - Export extraction from @EXPORT
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_returns_location_for_exported_symbol() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();

    // Index a module that exports 'foo' and 'bar' via @EXPORT
    let module_uri = file_url("/lib/MyModule.pm")?;
    let module_code = r#"package MyModule;
use strict;
use warnings;

our @EXPORT = qw(foo bar);

sub foo {
    return 1;
}

sub bar {
    return 2;
}

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // find_export should return a location for the exported symbol 'foo'
    let export_location = index.find_export("MyModule", "foo");
    assert!(
        export_location.is_some(),
        "find_export(MyModule, foo) should return Some(Location), got None. \
         Export extraction may not be implemented yet."
    );

    let location = export_location.unwrap();
    assert!(
        location.uri.contains("MyModule.pm"),
        "Export location URI should point to MyModule.pm, got: {}",
        location.uri
    );

    // bar should also be exported
    let bar_location = index.find_export("MyModule", "bar");
    assert!(
        bar_location.is_some(),
        "find_export(MyModule, bar) should return Some(Location), got None"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: AC1 - Export extraction from @EXPORT_OK
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_returns_location_for_export_ok_symbol() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/OptionalExporter.pm")?;
    let module_code = r#"package OptionalExporter;
use strict;
use warnings;

our @EXPORT_OK = qw(util_a util_b);

sub util_a {
    return 'a';
}

sub util_b {
    return 'b';
}

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Symbols in @EXPORT_OK should be findable via find_export
    let util_a_location = index.find_export("OptionalExporter", "util_a");
    assert!(
        util_a_location.is_some(),
        "find_export(OptionalExporter, util_a) should return Some(Location), got None. \
         @EXPORT_OK extraction may not be implemented yet."
    );

    let util_b_location = index.find_export("OptionalExporter", "util_b");
    assert!(
        util_b_location.is_some(),
        "find_export(OptionalExporter, util_b) should return Some(Location), got None"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: AC4 - %EXPORT_TAGS support
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_resolves_symbols_from_export_tags() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/TagExporter.pm")?;
    let module_code = r#"package TagExporter;
use strict;
use warnings;

our %EXPORT_TAGS = (
    all => [qw(foo bar baz)],
    helpers => [qw(foo bar)],
);

our @EXPORT_OK = qw(foo bar);

sub foo { return 1; }
sub bar { return 2; }
sub baz { return 3; }

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Symbols in %EXPORT_TAGS should be findable via find_export
    // 'foo' is in :all tag and @EXPORT_OK
    let foo_location = index.find_export("TagExporter", "foo");
    assert!(
        foo_location.is_some(),
        "find_export(TagExporter, foo) should return Some(Location) - foo is in :all tag and @EXPORT_OK"
    );

    // 'bar' is in :all tag, :helpers tag, and @EXPORT_OK
    let bar_location = index.find_export("TagExporter", "bar");
    assert!(
        bar_location.is_some(),
        "find_export(TagExporter, bar) should return Some(Location) - bar is in :all tag and @EXPORT_OK"
    );

    // 'baz' is only in :all tag (not in @EXPORT_OK)
    let baz_location = index.find_export("TagExporter", "baz");
    assert!(
        baz_location.is_some(),
        "find_export(TagExporter, baz) should return Some(Location) - baz is in :all tag"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: AC1 - get_exports_for_module returns all exported symbols
// ---------------------------------------------------------------------------

#[test]
fn test_get_exports_for_module_returns_all_exports() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/MyUtils.pm")?;
    let module_code = r#"package MyUtils;
use strict;
use warnings;

our @EXPORT = qw(helper_a helper_b);
our @EXPORT_OK = qw(optional_c);

sub helper_a { 1 }
sub helper_b { 2 }
sub optional_c { 3 }

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // get_exports_for_module should return all exported symbols
    let exports = index.get_exports_for_module("MyUtils");
    assert!(
        !exports.is_empty(),
        "get_exports_for_module(MyUtils) should return non-empty Vec, got empty. \
         Export extraction may not be implemented yet."
    );

    let export_symbols: Vec<&str> = exports.iter().map(|e| e.symbol.as_str()).collect();
    assert!(
        export_symbols.contains(&"helper_a"),
        "Exports should contain 'helper_a' (from @EXPORT), got: {:?}",
        export_symbols
    );
    assert!(
        export_symbols.contains(&"helper_b"),
        "Exports should contain 'helper_b' (from @EXPORT), got: {:?}",
        export_symbols
    );
    assert!(
        export_symbols.contains(&"optional_c"),
        "Exports should contain 'optional_c' (from @EXPORT_OK), got: {:?}",
        export_symbols
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: Non-exported symbol returns None
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_returns_none_for_non_exported_symbol() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/PrivateModule.pm")?;
    let module_code = r#"package PrivateModule;
use strict;
use warnings;

our @EXPORT = qw(public_a);

sub public_a { 1 }
sub _private_b { 2 }  # Not exported

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // _private_b is NOT exported, so find_export should return None
    let private_location = index.find_export("PrivateModule", "_private_b");
    assert!(
        private_location.is_none(),
        "find_export(PrivateModule, _private_b) should return None (not exported), got: {:?}",
        private_location
    );

    // public_a IS exported
    let public_location = index.find_export("PrivateModule", "public_a");
    assert!(
        public_location.is_some(),
        "find_export(PrivateModule, public_a) should return Some(Location)"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: Symbol not from any module returns None
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_returns_none_for_unknown_module() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Don't index anything

    let location = index.find_export("NonExistentModule", "foo");
    assert!(
        location.is_none(),
        "find_export(NonExistentModule, foo) should return None, got: {:?}",
        location
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: Multiple modules exporting the same symbol
// ---------------------------------------------------------------------------

#[test]
fn test_find_export_ambiguous_when_multiple_modules_export_same_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index two modules that both export 'shared_func'
    let module_a_uri = file_url("/lib/ModuleA.pm")?;
    index.index_file(
        module_a_uri,
        r#"package ModuleA;
our @EXPORT = qw(shared_func);
sub shared_func { 1 }
1;
"#
        .to_string(),
    )?;

    let module_b_uri = file_url("/lib/ModuleB.pm")?;
    index.index_file(
        module_b_uri,
        r#"package ModuleB;
our @EXPORT = qw(shared_func);
sub shared_func { 2 }
1;
"#
        .to_string(),
    )?;

    // When multiple modules export the same symbol, find_export should still
    // return a location, but callers should be aware of ambiguity
    // (initial implementation may return None or the first match found)
    let location = index.find_export("ModuleA", "shared_func");
    assert!(
        location.is_some(),
        "find_export(ModuleA, shared_func) should return Some even when ModuleB also exports it"
    );

    let location_b = index.find_export("ModuleB", "shared_func");
    assert!(
        location_b.is_some(),
        "find_export(ModuleB, shared_func) should return Some even when ModuleA also exports it"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: get_exports_for_module returns empty for unknown module
// ---------------------------------------------------------------------------

#[test]
fn test_get_exports_for_module_empty_for_unknown_module() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();

    let exports = index.get_exports_for_module("NonExistentModule");
    assert!(
        exports.is_empty(),
        "get_exports_for_module(NonExistentModule) should return empty Vec, got: {:?}",
        exports
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: Export extraction with qw() array
// ---------------------------------------------------------------------------

#[test]
fn test_export_extraction_with_qw_syntax() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/QwExporter.pm")?;
    let module_code = r#"package QwExporter;
use strict;
use warnings;

# Various qw() syntax forms
our @EXPORT = qw(func_a func_b func_c);
our @EXPORT_OK = qw(opt_a opt_b);

sub func_a { }
sub func_b { }
sub func_c { }
sub opt_a { }
sub opt_b { }

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // All qw()-listed symbols should be findable
    for symbol in &["func_a", "func_b", "func_c", "opt_a", "opt_b"] {
        let location = index.find_export("QwExporter", symbol);
        assert!(
            location.is_some(),
            "find_export(QwExporter, {}) should return Some(Location) - extracted from qw()",
            symbol
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: find_auto_export only matches @EXPORT (ExportKind::Explicit)
// ---------------------------------------------------------------------------
// This is the Bug 2 regression guard: bare `use Module;` must NOT auto-import
// symbols from @EXPORT_OK or %EXPORT_TAGS. Only @EXPORT symbols are
// auto-imported.

#[test]
fn test_find_auto_export_only_matches_explicit_export() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/MixedExports.pm")?;
    let module_code = r#"package MixedExports;
use strict;
use warnings;

our @EXPORT = qw(auto_a auto_b);
our @EXPORT_OK = qw(opt_c opt_d);
our %EXPORT_TAGS = (
    all => [qw(tag_e)],
);

sub auto_a { }
sub auto_b { }
sub opt_c { }
sub opt_d { }
sub tag_e { }

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // @EXPORT symbols: find_auto_export should return Some
    assert!(
        index.find_auto_export("MixedExports", "auto_a").is_some(),
        "find_auto_export(MixedExports, auto_a) should return Some — auto_a is in @EXPORT"
    );
    assert!(
        index.find_auto_export("MixedExports", "auto_b").is_some(),
        "find_auto_export(MixedExports, auto_b) should return Some — auto_b is in @EXPORT"
    );

    // @EXPORT_OK symbols: find_auto_export should return None
    // (but find_export should still return Some — the existing method is kind-agnostic)
    assert!(
        index.find_auto_export("MixedExports", "opt_c").is_none(),
        "find_auto_export(MixedExports, opt_c) should return None — opt_c is only in @EXPORT_OK"
    );
    assert!(
        index.find_export("MixedExports", "opt_c").is_some(),
        "find_export(MixedExports, opt_c) should still return Some — opt_c is in the export table"
    );
    assert!(
        index.find_auto_export("MixedExports", "opt_d").is_none(),
        "find_auto_export(MixedExports, opt_d) should return None — opt_d is only in @EXPORT_OK"
    );

    // %EXPORT_TAGS-only symbols: find_auto_export should return None
    assert!(
        index.find_auto_export("MixedExports", "tag_e").is_none(),
        "find_auto_export(MixedExports, tag_e) should return None — tag_e is only in %EXPORT_TAGS"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: All three export sources (@EXPORT, @EXPORT_OK, %EXPORT_TAGS) work
// ---------------------------------------------------------------------------

#[test]
fn test_all_export_sources_accessible_via_find_export() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let module_uri = file_url("/lib/KindTest.pm")?;
    let module_code = r#"package KindTest;
use strict;
use warnings;

our @EXPORT = qw(explicit_a explicit_b);
our @EXPORT_OK = qw(ok_a ok_b);
our %EXPORT_TAGS = (
    all => [qw(tag_a tag_b)],
);

sub explicit_a { }
sub explicit_b { }
sub ok_a { }
sub ok_b { }
sub tag_a { }
sub tag_b { }

1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Verify all symbols are accessible via find_export regardless of source
    for symbol in &["explicit_a", "explicit_b", "ok_a", "ok_b", "tag_a", "tag_b"] {
        let location = index.find_export("KindTest", symbol);
        assert!(
            location.is_some(),
            "find_export(KindTest, {}) should return Some(Location) regardless of export source",
            symbol
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: exports from two files contributing to the same package are not
// overwritten when the second file is indexed.
// ---------------------------------------------------------------------------

#[test]
fn test_export_table_merges_entries_from_multiple_files_same_package()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Two files that extend the same package namespace
    let file_a_uri = file_url("/lib/MultiPkg/Part1.pm")?;
    let file_a_code = r#"package MultiPkg;
our @EXPORT = qw(from_part1);
sub from_part1 { 1 }
1;
"#;
    let file_b_uri = file_url("/lib/MultiPkg/Part2.pm")?;
    let file_b_code = r#"package MultiPkg;
our @EXPORT = qw(from_part2);
sub from_part2 { 2 }
1;
"#;

    index.index_file(file_a_uri, file_a_code.to_string())?;
    index.index_file(file_b_uri, file_b_code.to_string())?;

    // Both symbols must be present — second index must not overwrite first
    let loc1 = index.find_export("MultiPkg", "from_part1");
    let loc2 = index.find_export("MultiPkg", "from_part2");

    assert!(loc1.is_some(), "from_part1 exported from Part1.pm must survive indexing Part2.pm");
    assert!(loc2.is_some(), "from_part2 exported from Part2.pm must be present");

    Ok(())
}
