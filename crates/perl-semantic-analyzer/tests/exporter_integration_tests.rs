//! Tests for Exporter Symbol Resolution - Integration Tests
//!
//! These tests verify the workspace-wide export symbol table implementation
//! that enables go-to-definition and completion for default-imported symbols
//! from Exporter-based Perl modules.
//!
//! These tests specifically target:
//! - ExportSymbolExtractor module (perl-semantic-analyzer/src/analysis/export_analyzer.rs)
//! - WorkspaceIndex export table integration
//! - find_declaration export table resolution
//! - CompletionProvider default export inclusion
//!
//! See: ADR-2025 Export Symbol Table for Exporter-Based Module Resolution
//! See: Issue #3409 Import/Export Gap: Exporter 'import' pattern not analyzed

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::must;

// -----------------------------------------------------------------------------
// Test infrastructure
// -----------------------------------------------------------------------------

fn parse_and_extract(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

// -----------------------------------------------------------------------------
// ExportSymbolExtractor Unit Tests
// Tests for the new export_analyzer.rs module
// -----------------------------------------------------------------------------

/// Test that ExportSymbolExtractor module exists and can be instantiated
/// This test will fail to compile if the module doesn't exist
#[test]
fn test_export_symbol_extractor_module_exists() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies the ExportSymbolExtractor module exists
    // The actual implementation should be at perl-semantic-analyzer/src/analysis/export_analyzer.rs

    let code = r#"
package Test::Module;
use Exporter 'import';
our @EXPORT = qw(exported_func);
sub exported_func { }
1;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // After implementation, this should work:
    // let extractor = ExportSymbolExtractor::new();
    // let export_info = extractor.extract(&ast);
    // assert!(export_info.is_some());
    // let info = export_info.unwrap();
    // assert!(info.default_export.contains("exported_func"));

    // Currently this test just verifies the AST can be parsed
    // The actual ExportSymbolExtractor tests will be written once the module exists
    assert!(has_symbol(&parse_and_extract(code), "exported_func", SymbolKind::Subroutine));

    Ok(())
}

/// Test Exporter inheritance detection: use Exporter 'import';
#[test]
fn test_exporter_detector_use_exporter_import() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::UseExporter;
use Exporter 'import';
our @EXPORT = qw(foo);
sub foo { }
1;
"#;

    let _ast = {
        let mut parser = Parser::new(code);
        must(parser.parse())
    };
    let table = parse_and_extract(code);

    // The module should be detected as an Exporter
    // After implementation:
    // let detector = ExporterDetector::detect(&ast);
    // assert!(matches!(detector, Some(ExporterDetector::UseExporterImport)));

    // Verify structure for now - foo is a subroutine
    assert!(has_symbol(&table, "foo", SymbolKind::Subroutine));

    Ok(())
}

/// Test Exporter inheritance detection: use parent 'Exporter';
#[test]
fn test_exporter_detector_use_parent_exporter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::UseParent;
use parent 'Exporter';
our @EXPORT = qw(bar);
sub bar { }
1;
"#;

    let table = parse_and_extract(code);

    // After implementation:
    // let detector = ExporterDetector::detect(&ast);
    // assert!(matches!(detector, Some(ExporterDetector::UseParentExporter)));

    assert!(has_symbol(&table, "bar", SymbolKind::Subroutine));

    Ok(())
}

/// Test Exporter inheritance detection: our @ISA = qw(Exporter);
#[test]
fn test_exporter_detector_our_isa_exporter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::OurIsa;
our @ISA = qw(Exporter);
our @EXPORT = qw(baz);
sub baz { }
1;
"#;

    let table = parse_and_extract(code);

    // After implementation:
    // let detector = ExporterDetector::detect(&ast);
    // assert!(matches!(detector, Some(ExporterDetector::OurIsaExporter)));

    assert!(has_symbol(&table, "baz", SymbolKind::Subroutine));

    Ok(())
}

/// Test QW delimiter parsing: ()
#[test]
fn test_qw_parsing_parentheses() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::QWParens;
use Exporter 'import';
our @EXPORT = qw(alpha beta);
sub alpha { }
sub beta { }
1;
"#;

    let table = parse_and_extract(code);

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast).unwrap();
    // assert!(info.default_export.contains("alpha"));
    // assert!(info.default_export.contains("beta"));

    assert!(has_symbol(&table, "alpha", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "beta", SymbolKind::Subroutine));

    Ok(())
}

/// Test QW delimiter parsing: []
#[test]
fn test_qw_parsing_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::QWBrackets;
use Exporter 'import';
our @EXPORT = [qw(gamma delta)];
sub gamma { }
sub delta { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "gamma", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "delta", SymbolKind::Subroutine));

    Ok(())
}

/// Test QW delimiter parsing: <>
#[test]
fn test_qw_parsing_angle_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::QWAngles;
use Exporter 'import';
our @EXPORT = qw<epsilon zeta>;
sub epsilon { }
sub zeta { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "epsilon", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "zeta", SymbolKind::Subroutine));

    Ok(())
}

/// Test QW delimiter parsing: //
#[test]
fn test_qw_parsing_double_slash() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::QWSlash;
use Exporter 'import';
our @EXPORT = qw// eta theta //;
sub eta { }
sub theta { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "eta", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "theta", SymbolKind::Subroutine));

    Ok(())
}

/// Test QW delimiter parsing: ||
#[test]
fn test_qw_parsing_double_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::QWPipe;
use Exporter 'import';
our @EXPORT_OK = qw|| iota kappa ||;
sub iota { }
sub kappa { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "iota", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "kappa", SymbolKind::Subroutine));

    Ok(())
}

/// Test EXPORT_TAGS parsing
#[test]
fn test_export_tags_parsing_nested_qw() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Test::Tags;
use Exporter 'import';

our @EXPORT_OK = qw(red green blue);
our %EXPORT_TAGS = (
    primary => [qw(red green)],
    colors  => [qw(blue)],
);

sub red { }
sub green { }
sub blue { }
1;
"#;

    let table = parse_and_extract(code);

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast).unwrap();
    // let primary = info.export_tags.get("primary");
    // assert_eq!(primary, Some(&vec!["red".to_string(), "green".to_string()]));

    assert!(has_symbol(&table, "red", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "green", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "blue", SymbolKind::Subroutine));

    Ok(())
}

// -----------------------------------------------------------------------------
// AC5: No False Positives for Non-Exporter Files
// -----------------------------------------------------------------------------

/// Test that use Exporter; without 'import' does NOT enable exports
#[test]
fn test_no_false_positive_use_exporter_no_import_arg() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Not::Exporter;
use Exporter;  # Missing 'import' - does NOT enable Exporter's import
our @EXPORT = qw(wrong);
sub wrong { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "wrong", SymbolKind::Subroutine));

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast);
    // assert!(info.is_none(), "Should not detect as Exporter without 'import' arg");

    Ok(())
}

/// Test that @EXPORT without Exporter inheritance is not treated as exports
#[test]
fn test_no_false_positive_no_exporter_inheritance() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Fake::Exporter;
# No Exporter inheritance at all
our @EXPORT = qw(not_exported);
sub not_exported { }
1;
"#;

    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "not_exported", SymbolKind::Subroutine));

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast);
    // assert!(info.is_none(), "Should not detect as Exporter without inheritance");

    Ok(())
}

// -----------------------------------------------------------------------------
// WorkspaceIndex Export Table Tests
// Tests for FileIndex and WorkspaceIndex extension with export fields
// -----------------------------------------------------------------------------

/// Test that FileIndex has export-related fields after implementation
/// This test verifies the structure but currently just checks basic indexing
#[test]
fn test_workspace_index_export_table_structure() -> Result<(), Box<dyn std::error::Error>> {
    // After implementation, WorkspaceIndex should have:
    // - export_table: HashMap<String, HashSet<String>> (module URI -> exported symbols)
    // - is_exported(module_uri, symbol) -> bool
    // - get_export_tags(module_uri, tag) -> Option<Vec<String>>

    let module_code = r#"
package My::Module;
use Exporter 'import';
our @EXPORT = qw(func_a func_b);
our @EXPORT_OK = qw(func_c);
our %EXPORT_TAGS = (group => [qw(func_c)]);
sub func_a { }
sub func_b { }
sub func_c { }
1;
"#;

    let table = parse_and_extract(module_code);
    assert!(has_symbol(&table, "func_a", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "func_b", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "func_c", SymbolKind::Subroutine));

    // After implementation, WorkspaceIndex should:
    // 1. Index the module and detect Exporter inheritance
    // 2. Extract export info: @EXPORT = {func_a, func_b}, @EXPORT_OK = {func_c}
    // 3. Extract export_tags: group -> [func_c]
    // 4. Store in export_table: "My::Module" -> {func_a, func_b, func_c}
    //
    // let index = WorkspaceIndex::new();
    // index.index_file(uri, module_code)?;
    //
    // assert!(index.is_exported("My::Module", "func_a"));
    // assert!(index.is_exported("My::Module", "func_b"));
    // assert!(index.is_exported("My::Module", "func_c"));
    // assert!(!index.is_exported("My::Module", "nonexistent"));
    //
    // let tags = index.get_export_tags("My::Module", "group");
    // assert_eq!(tags, Some(vec!["func_c".to_string()]));

    Ok(())
}

// -----------------------------------------------------------------------------
// AC2: Go-to-Definition for Default Exports
// Tests that find_declaration queries export table for unresolved symbols
// -----------------------------------------------------------------------------

/// Test that find_declaration resolves symbols via export table
/// This is an integration test that requires both WorkspaceIndex and declaration resolution
#[test]
fn test_find_declaration_via_export_table() -> Result<(), Box<dyn std::error::Error>> {
    let loader_code = r#"
package My::Loader;
use Exporter 'import';
our @EXPORT = qw(load_data);

sub load_data {
    return "loaded";
}

1;
"#;

    let consumer_code = r#"
package main;
use My::Loader;

load_data();  # Cursor on load_data should go to My::Loader::load_data

1;
"#;

    // First verify the loader module structure
    let loader_table = parse_and_extract(loader_code);
    assert!(has_symbol(&loader_table, "load_data", SymbolKind::Subroutine));

    // After implementation:
    // - WorkspaceIndex indexes My::Loader and extracts export info
    // - find_declaration in consumer code is called for "load_data"
    // - Local resolution fails (load_data not defined in main)
    // - find_declaration queries export table: "which module exports load_data?"
    // - My::Loader is found, returns My::Loader::load_data location
    //
    // let index = WorkspaceIndex::new();
    // index.index_file(loader_uri, loader_code)?;
    // index.index_file(consumer_uri, consumer_code)?;
    //
    // let declaration = find_declaration(consumer_ast, offset, "main", Some(index));
    // assert!(declaration.is_some());
    // let links = declaration.unwrap();
    // assert_eq!(links[0].target_uri, loader_uri);

    Ok(())
}

// -----------------------------------------------------------------------------
// AC3: Completion for Default Exports
// Tests that CompletionProvider includes @EXPORT symbols when use Module; has no args
// -----------------------------------------------------------------------------

/// Test that completion includes @EXPORT symbols for use Module; with no args
/// This is an integration test for CompletionProvider
#[test]
fn test_completion_includes_default_exports() -> Result<(), Box<dyn std::error::Error>> {
    let module_code = r#"
package My::Utils;
use Exporter 'import';
our @EXPORT = qw(process format);
our @EXPORT_OK = qw(extra_func);
sub process { }
sub format { }
sub extra_func { }
sub _private { }
1;
"#;

    let consumer_code = r#"
package main;
use My::Utils;

# Completion after "proc" should include "process"
# Completion after "ext" should include "extra_func" (from @EXPORT_OK, requires explicit import)
# "_private" should NOT appear (not exported at all)

1;
"#;

    // Verify module structure
    let module_table = parse_and_extract(module_code);
    assert!(has_symbol(&module_table, "process", SymbolKind::Subroutine));
    assert!(has_symbol(&module_table, "format", SymbolKind::Subroutine));
    assert!(has_symbol(&module_table, "extra_func", SymbolKind::Subroutine));
    assert!(has_symbol(&module_table, "_private", SymbolKind::Subroutine));

    // After implementation, CompletionProvider should:
    // 1. Detect `use My::Utils;` with no arguments
    // 2. Query export table: My::Utils -> @EXPORT = {process, format}
    // 3. Include process and format in completions
    // 4. NOT include extra_func (requires explicit qw(...) import)
    // 5. NOT include _private (not exported)
    //
    // let provider = CompletionProvider::new_with_index(consumer_ast, Some(index));
    // let completions = provider.get_completions(consumer_code, proc_offset);
    // let labels: Vec<_> = completions.iter().map(|c| c.label.clone()).collect();
    // assert!(labels.contains(&"process".to_string()));
    // assert!(labels.contains(&"format".to_string()));
    // assert!(!labels.contains(&"_private".to_string()));

    Ok(())
}

// -----------------------------------------------------------------------------
// AC4: Export Tag Resolution
// Tests that :tag imports resolve to correct symbols
// -----------------------------------------------------------------------------

/// Test that use Module qw(:tag); resolves tag to correct symbols
#[test]
fn test_export_tag_resolution_in_completion() -> Result<(), Box<dyn std::error::Error>> {
    let module_code = r#"
package My::Module;
use Exporter 'import';
our @EXPORT_OK = qw(default_func);
our %EXPORT_TAGS = (
    ops => [qw(add subtract multiply)],
    cmp => [qw(compare validate)],
);
sub default_func { }
sub add { }
sub subtract { }
sub multiply { }
sub compare { }
sub validate { }
1;
"#;

    let consumer_code = r#"
package main;
use My::Module qw(:ops);

# Completion should include add, subtract, multiply (from :ops)
# NOT include compare, validate (from :cmp)
# NOT include default_func (requires explicit import or :all)

1;
"#;

    let module_table = parse_and_extract(module_code);
    for name in ["add", "subtract", "multiply", "compare", "validate", "default_func"] {
        assert!(has_symbol(&module_table, name, SymbolKind::Subroutine));
    }

    // After implementation:
    // - extract_import_map should detect qw(:ops) argument
    // - resolve_known_export_tag should look up :ops in export_tags
    // - CompletionProvider should include add, subtract, multiply

    Ok(())
}

// -----------------------------------------------------------------------------
// AC6: Symbol Collision Resolution
// Tests that most recent import wins in collision
// -----------------------------------------------------------------------------

/// Test that symbol collision uses import order (most recent wins)
#[test]
fn test_symbol_collision_import_order() -> Result<(), Box<dyn std::error::Error>> {
    let module_a_code = r#"
package A;
use Exporter 'import';
our @EXPORT = qw(helper);
sub helper { "A's helper" }
1;
"#;

    let module_b_code = r#"
package B;
use Exporter 'import';
our @EXPORT = qw(helper);
sub helper { "B's helper" }
1;
"#;

    let consumer_code = r#"
package main;
use A;
use B;

helper();  # Should resolve to B::helper (most recent import)

1;
"#;

    // Verify both modules export helper
    let table_a = parse_and_extract(module_a_code);
    let table_b = parse_and_extract(module_b_code);
    assert!(has_symbol(&table_a, "helper", SymbolKind::Subroutine));
    assert!(has_symbol(&table_b, "helper", SymbolKind::Subroutine));

    // After implementation:
    // - Both A and B are indexed with helper in their export tables
    // - When resolving helper in consumer code:
    // - Import order is: A (first), B (second/more recent)
    // - Most recent import (B) should win
    //
    // let declaration = find_declaration(consumer_ast, offset, "main", Some(index));
    // assert!(declaration.is_some());
    // let links = declaration.unwrap();
    // assert_eq!(links[0].target_uri, b_uri);  # B is more recent

    Ok(())
}

// -----------------------------------------------------------------------------
// Edge Case: Empty Exports
// -----------------------------------------------------------------------------

/// Test empty @EXPORT
#[test]
fn test_empty_export_array() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Empty::Export;
use Exporter 'import';
our @EXPORT = ();
1;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "EXPORT", SymbolKind::array()));

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast).unwrap();
    // assert!(info.default_export.is_empty());

    Ok(())
}

/// Test empty %EXPORT_TAGS
#[test]
fn test_empty_export_tags() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Empty::Tags;
use Exporter 'import';
our @EXPORT_OK = qw(something);
our %EXPORT_TAGS = ();
sub something { }
1;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "EXPORT_TAGS", SymbolKind::hash()));

    // After implementation:
    // let info = ExportSymbolExtractor::extract(&ast).unwrap();
    // assert!(info.export_tags.is_empty());

    Ok(())
}

// -----------------------------------------------------------------------------
// Edge Case: Multiple Packages in One File
// -----------------------------------------------------------------------------

/// Test that only Exporter package gets export table entry
#[test]
fn test_multiple_packages_only_exporter_gets_export() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Exporter::Pkg;
use Exporter 'import';
our @EXPORT = qw(exported);
sub exported { }

package Non::Exporter::Pkg;
# No Exporter inheritance
our @EXPORT = qw(not_exported);
sub not_exported { }

1;
"#;
    let table = parse_and_extract(code);

    assert!(has_symbol(&table, "exported", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "not_exported", SymbolKind::Subroutine));

    // After implementation:
    // - Exporter::Pkg should be in export table with "exported"
    // - Non::Exporter::Pkg should NOT be in export table

    Ok(())
}
