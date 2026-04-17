//! Red Tests for Exporter Symbol Resolution
//!
//! These tests verify the workspace-wide export symbol table implementation
//! that enables go-to-definition and completion for default-imported symbols
//! from Exporter-based Perl modules.
//!
//! These tests will FAIL until the export table is implemented because they
//! call `symbol_at_cursor` expecting it to resolve symbols via the export table,
//! but the current implementation doesn't query the export table for default imports.
//!
//! See: ADR-2025 Export Symbol Table for Exporter-Based Module Resolution
//! See: Issue #3409 Import/Export Gap: Exporter 'import' pattern not analyzed

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::declaration::symbol_at_cursor;

/// Helper function to find symbol at cursor position and return its package
fn parse_and_symbol_at(code: &str, needle: &str) -> Option<String> {
    // Find the byte offset of needle in the code
    let offset = code.find(needle)?;
    let mut parser = Parser::new(code);
    let ast = parser.parse().ok()?;
    let key = symbol_at_cursor(&ast, offset, "main")?;
    Some(key.pkg.to_string())
}

// -----------------------------------------------------------------------------
// AC2: Go-to-Definition for Default Exports
// Tests that symbol_at_cursor resolves exported symbols via export table
// -----------------------------------------------------------------------------

/// AC2: Given module My::Loader that exports load_data via @EXPORT
/// And a file containing use My::Loader; load_data();
/// When the user triggers go-to-definition on load_data
/// Then symbol_at_cursor should return Some("My::Loader")
///
/// CURRENT STATUS: FAILS because symbol_at_cursor doesn't query export table
/// for default imports (Module->import() with no args returns false).
#[test]
fn test_ac2_goto_definition_default_export_via_use() {
    let code = r#"
require My::Loader;
My::Loader->import();  # Default import - loads @EXPORT
my $x = load_data();
"#;
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Loader"),
        "load_data() should resolve to My::Loader via default @EXPORT import, got: {pkg:?}"
    );
}

/// AC2 variant: Using 'use' statement with no args
/// CURRENT STATUS: FAILS because extract_import_map returns early for no args
#[test]
fn test_ac2_goto_definition_use_module_no_args() {
    let code = r#"
use My::Utils;
process();  # Should resolve to My::Utils::process via @EXPORT
"#;
    let pkg = parse_and_symbol_at(code, "process()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Utils"),
        "process() should resolve to My::Utils via use + @EXPORT, got: {pkg:?}"
    );
}

/// AC2: Multiple symbols from same module
#[test]
fn test_ac2_goto_definition_multiple_exports() {
    let code = r#"
use My::Module;
My::Module->import();  # Default import
alpha();
beta();
gamma();
"#;
    let pkg_alpha = parse_and_symbol_at(code, "alpha()");
    let pkg_beta = parse_and_symbol_at(code, "beta()");
    let pkg_gamma = parse_and_symbol_at(code, "gamma()");

    assert_eq!(
        pkg_alpha.as_deref(),
        Some("My::Module"),
        "alpha() should resolve to My::Module"
    );
    assert_eq!(
        pkg_beta.as_deref(),
        Some("My::Module"),
        "beta() should resolve to My::Module"
    );
    assert_eq!(
        pkg_gamma.as_deref(),
        Some("My::Module"),
        "gamma() should resolve to My::Module"
    );
}

// -----------------------------------------------------------------------------
// AC3: Completion for Default Exports
// Tests that use Module; with no args should include @EXPORT symbols
// -----------------------------------------------------------------------------

/// AC3: The extract_import_map function should NOT return early for no args
/// Instead it should mark the module as needing default export lookup
/// CURRENT STATUS: FAILS because extract_import_map returns early for no args
#[test]
fn test_ac3_completion_no_args_import_map_entry() {
    // This test verifies the ImportMap behavior
    // After implementation, `use My::Module;` with no args should add
    // My::Module to the import map with an empty set, BUT the completion
    // provider should then query the export table for @EXPORT symbols.
    //
    // Currently, extract_import_map returns early for no args, so the module
    // isn't even in the import map.

    let code = r#"
use My::Module;

process
"#;
    let pkg = parse_and_symbol_at(code, "process");
    // This should resolve via export table, currently it won't
    assert_eq!(
        pkg.as_deref(),
        Some("My::Module"),
        "process should resolve to My::Module via @EXPORT"
    );
}

// -----------------------------------------------------------------------------
// AC4: Export Tag Resolution
// Tests that :tag imports resolve to correct symbols
// -----------------------------------------------------------------------------

/// AC4: Given module My::Module with %EXPORT_TAGS = (ops => [qw(add subtract)])
/// And a file containing use My::Module qw(:ops);
/// Then symbol_at_cursor should resolve add/subtract to My::Module
///
/// CURRENT STATUS: FAILS because export tag resolution isn't implemented
#[test]
fn test_ac4_export_tag_resolution() {
    let code = r#"
use My::Module qw(:ops);
add(1, 2);
subtract(3, 4);
"#;
    let pkg_add = parse_and_symbol_at(code, "add(");
    let pkg_sub = parse_and_symbol_at(code, "subtract(");

    assert_eq!(
        pkg_add.as_deref(),
        Some("My::Module"),
        "add() should resolve to My::Module via :ops tag"
    );
    assert_eq!(
        pkg_sub.as_deref(),
        Some("My::Module"),
        "subtract() should resolve to My::Module via :ops tag"
    );
}

// -----------------------------------------------------------------------------
// AC5: No False Positives for Non-Exporter Files
// Tests that @EXPORT without Exporter inheritance doesn't resolve
// -----------------------------------------------------------------------------

/// AC5: Given a file with @EXPORT but no Exporter inheritance
/// When symbol_at_cursor is called on an exported symbol
/// Then it should NOT resolve (symbol is not actually exported)
///
/// CURRENT STATUS: Currently passes because non-Exporter files aren't
/// in the export table anyway (the export table doesn't exist yet)
#[test]
fn test_ac5_no_false_positives_non_exporter() {
    let code = r#"
package Fake::Exporter;
our @EXPORT = qw(not_exported);

sub not_exported { }

package main;
use strict;
my $x = not_exported();
"#;
    let pkg = parse_and_symbol_at(code, "not_exported()");
    // Should NOT resolve because Fake::Exporter doesn't actually inherit Exporter
    // Currently this might return None or Some("Fake::Exporter") depending on
    // whether the symbol is found locally
    let _ = pkg; // Placeholder - actual implementation needs to verify
}

// -----------------------------------------------------------------------------
// AC6: Symbol Collision Resolution
// Tests that import order determines which module wins in collisions
// -----------------------------------------------------------------------------

/// AC6: Given module A exports helper and module B exports helper
/// And a file contains use A; use B; helper();
/// When symbol_at_cursor is triggered on helper
/// Then it should resolve to B (most recently imported)
///
/// CURRENT STATUS: FAILS because there's no export table to query for collision
#[test]
fn test_ac6_symbol_collision_most_recent_wins() {
    let code = r#"
use A;
use B;
helper();
"#;
    let pkg = parse_and_symbol_at(code, "helper()");
    assert_eq!(
        pkg.as_deref(),
        Some("B"),
        "helper() should resolve to B (most recent import), got: {pkg:?}"
    );
}

/// AC6 variant: Three-way collision
#[test]
fn test_ac6_symbol_collision_three_way() {
    let code = r#"
use A;
use B;
use C;
helper();
"#;
    let pkg = parse_and_symbol_at(code, "helper()");
    assert_eq!(
        pkg.as_deref(),
        Some("C"),
        "helper() should resolve to C (most recent import), got: {pkg:?}"
    );
}

// -----------------------------------------------------------------------------
// Exporter Detection Tests
// Tests that verify the three Exporter inheritance patterns
// -----------------------------------------------------------------------------

/// Test Exporter pattern: use Exporter 'import';
#[test]
fn test_exporter_pattern_use_exporter_import() {
    let code = r#"
package My::Loader;
use Exporter 'import';
our @EXPORT = qw(load_data);

sub load_data { }
"#;
    // After implementation, this module should be detected as an Exporter
    // and its @EXPORT symbols should be resolvable
    let _ = code; // Placeholder - module detection needs implementation
}

/// Test Exporter pattern: use parent 'Exporter';
#[test]
fn test_exporter_pattern_use_parent_exporter() {
    let code = r#"
package My::Module;
use parent 'Exporter';
our @EXPORT = qw(exported_func);

sub exported_func { }
"#;
    let _ = code; // Placeholder
}

/// Test Exporter pattern: our @ISA = qw(Exporter);
#[test]
fn test_exporter_pattern_our_isa_exporter() {
    let code = r#"
package My::Base;
our @ISA = qw(Exporter);
our @EXPORT = qw(base_method);

sub base_method { }
"#;
    let _ = code; // Placeholder
}

// -----------------------------------------------------------------------------
// Edge Cases
// -----------------------------------------------------------------------------

/// Test that local function shadows exported function
#[test]
fn test_local_shadows_exported() {
    let code = r#"
use My::Module;
sub local_func { }  # This shadows My::Module's export
local_func();
"#;
    let pkg = parse_and_symbol_at(code, "local_func()");
    // Should resolve to local definition, not My::Module
    // This test might pass even without export table because local is found first
    let _ = pkg;
}

/// Test that explicit import overrides default export
#[test]
fn test_explicit_import_overrides_default() {
    let code = r#"
use My::Module qw(specific_func);
specific_func();
"#;
    let pkg = parse_and_symbol_at(code, "specific_func()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Module"),
        "specific_func() should resolve to My::Module via explicit import"
    );
}

/// Test empty @EXPORT - no symbols should resolve
#[test]
fn test_empty_export_no_resolution() {
    let code = r#"
use Empty::Module;  # @EXPORT = ()
nonexistent();
"#;
    let pkg = parse_and_symbol_at(code, "nonexistent()");
    // Should not resolve because Empty::Module exports nothing
    assert_eq!(
        pkg.as_deref(),
        None,
        "nonexistent() should NOT resolve (Empty::Module exports nothing)"
    );
}
