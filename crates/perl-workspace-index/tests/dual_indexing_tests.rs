//! Tests for dual indexing (qualified + bare names) and real-world patterns.
//!
//! Verifies the PR #122 dual indexing strategy: every symbol should be
//! discoverable under both its fully qualified name (`Package::function`) and
//! its bare name (`function`).
//!
//! Also exercises realistic Perl patterns:
//! - Large modules with many subs (DBI-like)
//! - Exporter-based modules
//! - Moose method modifiers (`around`, `before`, `after`)
//! - Multiple packages in a single file

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

// ---------------------------------------------------------------------------
// Helper: parse a file:// URL
// ---------------------------------------------------------------------------
fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// =========================================================================
// 1. Dual indexing – qualified + bare name lookups
// =========================================================================

#[test]
fn dual_index_file_find_find_qualified_definition() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/File/Find.pm")?;

    let code = "\
package File::Find;
sub find { return 1; }
";
    index.index_file(uri, code.to_string())?;

    // Should be discoverable via the qualified name
    let def = index.find_definition("File::Find::find");
    assert!(def.is_some(), "expected File::Find::find to be found via qualified name");
    Ok(())
}

#[test]
fn dual_index_file_find_find_bare_definition() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/File/Find.pm")?;

    let code = "\
package File::Find;
sub find { return 1; }
";
    index.index_file(uri, code.to_string())?;

    // Should also be discoverable via bare name
    let def = index.find_definition("find");
    assert!(def.is_some(), "expected File::Find::find to be found via bare name 'find'");
    Ok(())
}

#[test]
fn dual_index_file_find_find_search_returns_both() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/File/Find.pm")?;

    let code = "\
package File::Find;
sub find { return 1; }
";
    index.index_file(uri, code.to_string())?;

    // search_symbols should return the symbol when searching either way
    let by_qualified = index.search_symbols("File::Find::find");
    assert!(!by_qualified.is_empty(), "search_symbols should find results for 'File::Find::find'");

    let by_bare = index.search_symbols("find");
    assert!(!by_bare.is_empty(), "search_symbols should find results for bare 'find'");
    Ok(())
}

#[test]
fn dual_index_moose_has_qualified_definition() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Moose.pm")?;

    let code = "\
package Moose;
sub has { return 1; }
";
    index.index_file(uri, code.to_string())?;

    let def = index.find_definition("Moose::has");
    assert!(def.is_some(), "expected Moose::has to be found via qualified name");
    Ok(())
}

#[test]
fn dual_index_moose_has_bare_definition() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Moose.pm")?;

    let code = "\
package Moose;
sub has { return 1; }
";
    index.index_file(uri, code.to_string())?;

    let def = index.find_definition("has");
    assert!(def.is_some(), "expected Moose::has to be found via bare name 'has'");
    Ok(())
}

#[test]
fn dual_index_references_found_from_qualified_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/lib/Utils.pm")?;
    let caller_uri = file_url("/scripts/main.pl")?;

    // Define the function
    index.index_file(def_uri, "package Utils;\nsub process_data { return 1; }".to_string())?;

    // Call it by bare name in another file
    index.index_file(caller_uri, "process_data();".to_string())?;

    // Searching references with qualified name should find the bare call
    let refs = index.find_references("Utils::process_data");
    assert!(
        !refs.is_empty(),
        "find_references('Utils::process_data') should pick up bare 'process_data()' call"
    );
    Ok(())
}

#[test]
fn dual_index_references_found_from_bare_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let def_uri = file_url("/lib/Utils.pm")?;
    let caller_uri = file_url("/scripts/main.pl")?;

    index.index_file(def_uri, "package Utils;\nsub process_data { return 1; }".to_string())?;

    // Call it qualified in another file
    index.index_file(caller_uri, "Utils::process_data();".to_string())?;

    // Searching references with bare name should find the call
    let refs = index.find_references("process_data");
    assert!(!refs.is_empty(), "find_references('process_data') should find calls");
    Ok(())
}

// =========================================================================
// 2. Re-indexing removes old entries
// =========================================================================

#[test]
fn reindex_file_replaces_old_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Evolving.pm")?;

    // Initial version with sub old_func
    index.index_file(uri.clone(), "package Evolving;\nsub old_func { 1 }".to_string())?;

    assert!(
        index.find_definition("old_func").is_some(),
        "old_func should exist after initial index"
    );

    // Re-index with different content (old_func is gone, new_func appears)
    index.index_file(uri, "package Evolving;\nsub new_func { 2 }".to_string())?;

    assert!(index.find_definition("new_func").is_some(), "new_func should exist after re-index");

    // old_func should no longer be in the index
    assert!(
        index.find_definition("old_func").is_none(),
        "old_func should be gone after re-index with different content"
    );
    Ok(())
}

#[test]
fn reindex_preserves_other_files() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri_a = file_url("/lib/A.pm")?;
    let uri_b = file_url("/lib/B.pm")?;

    index.index_file(uri_a.clone(), "package A;\nsub alpha { 1 }".to_string())?;
    index.index_file(uri_b, "package B;\nsub beta { 1 }".to_string())?;

    // Re-index A with new content
    index.index_file(uri_a, "package A;\nsub alpha_v2 { 2 }".to_string())?;

    // B should be untouched
    assert!(
        index.find_definition("beta").is_some(),
        "beta in B.pm should survive re-indexing of A.pm"
    );
    assert!(
        index.find_definition("alpha_v2").is_some(),
        "alpha_v2 should exist after re-index of A.pm"
    );
    assert!(
        index.find_definition("alpha").is_none(),
        "alpha should be gone after re-index of A.pm"
    );
    Ok(())
}

#[test]
fn reindex_updates_global_symbol_map() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Mapped.pm")?;

    index.index_file(uri.clone(), "package Mapped;\nsub before_rename { 1 }".to_string())?;

    assert!(index.find_definition("Mapped::before_rename").is_some());

    // Re-index with renamed function
    index.index_file(uri, "package Mapped;\nsub after_rename { 1 }".to_string())?;

    assert!(
        index.find_definition("Mapped::after_rename").is_some(),
        "after_rename should be in the global symbol map"
    );
    assert!(
        index.find_definition("Mapped::before_rename").is_none(),
        "before_rename should be removed from the global symbol map"
    );
    Ok(())
}

// =========================================================================
// 3. Real-world pattern: large module with many subs (DBI-like)
// =========================================================================

#[test]
fn large_module_many_subs() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/DBI.pm")?;

    // Simulate a DBI-like module with many subroutines
    let mut code = String::from("package DBI;\n");
    let sub_names: Vec<String> = (0..50).map(|i| format!("method_{}", i)).collect();
    for name in &sub_names {
        code.push_str(&format!("sub {} {{ return 1; }}\n", name));
    }

    index.index_file(uri.clone(), code)?;

    let uri_str = uri.to_string();

    // All subs should be discoverable
    for name in &sub_names {
        let qualified = format!("DBI::{}", name);
        assert!(
            index.find_definition(&qualified).is_some(),
            "expected {} to be found via qualified name",
            qualified,
        );
        assert!(
            index.find_definition(name).is_some(),
            "expected {} to be found via bare name",
            name,
        );
    }

    // Package members should list them all
    let members = index.get_package_members("DBI");
    assert!(members.len() >= 50, "DBI should have at least 50 members, got {}", members.len());

    // File symbols should include all subs plus the package
    let file_syms = index.file_symbols(&uri_str);
    // 50 subs + 1 package declaration
    assert!(
        file_syms.len() >= 51,
        "expected at least 51 file symbols (50 subs + 1 package), got {}",
        file_syms.len()
    );
    Ok(())
}

// =========================================================================
// 4. Real-world pattern: Exporter-based module
// =========================================================================

#[test]
fn exporter_based_module_tracks_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MyExporter.pm")?;

    let code = "\
package MyExporter;
use Exporter;
sub export_func_a { return 1; }
sub export_func_b { return 2; }
sub _private { return 3; }
";
    let uri_str = uri.to_string();
    index.index_file(uri, code.to_string())?;

    // Dependency on Exporter should be tracked
    let deps = index.file_dependencies(&uri_str);
    assert!(deps.contains("Exporter"), "MyExporter should depend on Exporter, deps: {:?}", deps,);

    // All subs should be indexed
    assert!(index.find_definition("MyExporter::export_func_a").is_some());
    assert!(index.find_definition("MyExporter::export_func_b").is_some());
    assert!(index.find_definition("MyExporter::_private").is_some());

    // Bare name lookup
    assert!(index.find_definition("export_func_a").is_some());
    assert!(index.find_definition("_private").is_some());
    Ok(())
}

#[test]
fn exporter_consumer_tracks_import_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let mod_uri = file_url("/lib/MyExporter.pm")?;
    let consumer_uri = file_url("/scripts/consumer.pl")?;

    index.index_file(mod_uri, "package MyExporter;\nsub exported { 1 }".to_string())?;

    let code = "\
use MyExporter;
exported();
";
    let consumer_uri_str = consumer_uri.to_string();
    index.index_file(consumer_uri, code.to_string())?;

    let deps = index.file_dependencies(&consumer_uri_str);
    assert!(deps.contains("MyExporter"), "consumer should depend on MyExporter");

    // find_dependents should discover the consumer
    let dependents = index.find_dependents("MyExporter");
    assert!(
        !dependents.is_empty(),
        "find_dependents should find at least one dependent of MyExporter"
    );
    Ok(())
}

#[test]
fn module_runtime_literal_calls_track_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/scripts/runtime_loader.pl")?;
    let code = "\
use Module::Runtime qw(use_module require_module);
my $m = use_module('My::Loader');
require_module('My::Other');
";
    let uri_str = uri.to_string();
    index.index_file(uri, code.to_string())?;

    let deps = index.file_dependencies(&uri_str);
    assert!(
        deps.contains("My::Loader"),
        "literal use_module should add dependency, deps: {:?}",
        deps
    );
    assert!(
        deps.contains("My::Other"),
        "literal require_module should add dependency, deps: {:?}",
        deps
    );
    Ok(())
}

#[test]
fn module_runtime_dynamic_calls_stay_conservative() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/scripts/runtime_loader_dynamic.pl")?;
    let code = "\
use Module::Runtime qw(require_module);
my $name = 'My::Loader';
require_module($name);
";
    let uri_str = uri.to_string();
    index.index_file(uri, code.to_string())?;

    let deps = index.file_dependencies(&uri_str);
    assert!(
        !deps.contains("My::Loader"),
        "dynamic require_module argument should remain unresolved, deps: {:?}",
        deps
    );
    Ok(())
}

// =========================================================================
// 5. Real-world pattern: Moose method modifiers
// =========================================================================

#[test]
fn moose_method_modifiers_as_function_calls() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MyMooseClass.pm")?;

    // Moose method modifiers are function calls: around('method_name', sub { ... })
    // The parser sees them as function calls to 'around', 'before', 'after'
    let code = "\
package MyMooseClass;
use Moose;

sub process { return 1; }
sub validate { return 1; }
sub cleanup { return 1; }

around('process', sub { my $orig = shift; return $orig->(@_); });
before('validate', sub { return 1; });
after('cleanup', sub { return 1; });
";
    let uri_str = uri.to_string();
    index.index_file(uri, code.to_string())?;

    // The three subs should be indexed
    assert!(index.find_definition("MyMooseClass::process").is_some());
    assert!(index.find_definition("MyMooseClass::validate").is_some());
    assert!(index.find_definition("MyMooseClass::cleanup").is_some());

    // Moose should be a dependency
    let deps = index.file_dependencies(&uri_str);
    assert!(deps.contains("Moose"), "should depend on Moose, deps: {:?}", deps);

    // around/before/after should appear as references (function calls)
    let around_refs = index.find_references("around");
    assert!(!around_refs.is_empty(), "around() calls should be indexed as references");

    let before_refs = index.find_references("before");
    assert!(!before_refs.is_empty(), "before() calls should be indexed as references");

    let after_refs = index.find_references("after");
    assert!(!after_refs.is_empty(), "after() calls should be indexed as references");
    Ok(())
}

#[test]
fn moose_has_attribute_declarations() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Person.pm")?;

    // Moose `has` is a function call
    let code = "\
package Person;
use Moose;

has('name', is => 'ro', isa => 'Str');
has('age', is => 'rw', isa => 'Int');

sub greet { return 'hello'; }
";
    index.index_file(uri, code.to_string())?;

    // The sub should be indexed
    assert!(index.find_definition("Person::greet").is_some());

    // `has` calls should show up as references
    let has_refs = index.find_references("has");
    assert!(!has_refs.is_empty(), "has() calls should produce references");
    Ok(())
}

// =========================================================================
// 6. Real-world pattern: multiple packages in one file
// =========================================================================

#[test]
fn multiple_packages_in_one_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MultiPkg.pm")?;

    let code = "\
package Alpha;
sub alpha_method { return 'a'; }

package Beta;
sub beta_method { return 'b'; }

package Gamma;
sub gamma_method { return 'g'; }
";
    let uri_str = uri.to_string();
    index.index_file(uri, code.to_string())?;

    // Each package should be indexed
    let syms = index.file_symbols(&uri_str);
    let pkg_names: Vec<&str> = syms
        .iter()
        .filter(|s| s.kind == perl_symbol::SymbolKind::Package)
        .map(|s| s.name.as_str())
        .collect();
    assert!(pkg_names.contains(&"Alpha"), "Alpha package should be indexed");
    assert!(pkg_names.contains(&"Beta"), "Beta package should be indexed");
    assert!(pkg_names.contains(&"Gamma"), "Gamma package should be indexed");

    // Subs should be qualified under their respective packages
    assert!(
        index.find_definition("Alpha::alpha_method").is_some(),
        "alpha_method should be qualified under Alpha"
    );
    assert!(
        index.find_definition("Beta::beta_method").is_some(),
        "beta_method should be qualified under Beta"
    );
    assert!(
        index.find_definition("Gamma::gamma_method").is_some(),
        "gamma_method should be qualified under Gamma"
    );

    // Bare names should also work
    assert!(index.find_definition("alpha_method").is_some());
    assert!(index.find_definition("beta_method").is_some());
    assert!(index.find_definition("gamma_method").is_some());
    Ok(())
}

#[test]
fn multiple_packages_subs_assigned_to_correct_container() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/TwoPkg.pm")?;

    let code = "\
package First;
sub first_sub { 1 }

package Second;
sub second_sub { 2 }
";
    index.index_file(uri, code.to_string())?;

    // Package members should be separated correctly
    let first_members = index.get_package_members("First");
    let first_names: Vec<&str> = first_members.iter().map(|s| s.name.as_str()).collect();
    assert!(first_names.contains(&"first_sub"), "first_sub should belong to First");
    assert!(!first_names.contains(&"second_sub"), "second_sub should NOT belong to First");

    let second_members = index.get_package_members("Second");
    let second_names: Vec<&str> = second_members.iter().map(|s| s.name.as_str()).collect();
    assert!(second_names.contains(&"second_sub"), "second_sub should belong to Second");
    assert!(!second_names.contains(&"first_sub"), "first_sub should NOT belong to Second");
    Ok(())
}

// =========================================================================
// 7. Edge cases
// =========================================================================

#[test]
fn deeply_qualified_package_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/A/B/C/D.pm")?;

    let code = "\
package A::B::C::D;
sub deep_func { return 1; }
";
    index.index_file(uri, code.to_string())?;

    // Qualified lookup
    assert!(
        index.find_definition("A::B::C::D::deep_func").is_some(),
        "deep_func should be found via fully qualified name"
    );

    // Bare lookup
    assert!(
        index.find_definition("deep_func").is_some(),
        "deep_func should be found via bare name"
    );

    // Search should find it
    let results = index.search_symbols("deep_func");
    assert!(!results.is_empty());
    Ok(())
}

#[test]
fn empty_file_indexes_without_error() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/empty.pl")?;

    // Should not error on empty content
    index.index_file(uri, String::new())?;
    assert_eq!(index.symbol_count(), 0);
    Ok(())
}

#[test]
fn reindex_after_removing_all_subs() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Shrink.pm")?;

    // Start with content
    index.index_file(
        uri.clone(),
        "package Shrink;\nsub big_func { 1 }\nsub another { 2 }".to_string(),
    )?;
    assert!(index.find_definition("big_func").is_some());
    assert!(index.find_definition("another").is_some());

    // Re-index with just the package declaration (all subs removed)
    index.index_file(uri, "package Shrink;\n# all subs removed".to_string())?;
    assert!(
        index.find_definition("big_func").is_none(),
        "big_func should be gone after removing all subs"
    );
    assert!(
        index.find_definition("another").is_none(),
        "another should be gone after removing all subs"
    );

    // The package should still be there
    let results = index.search_symbols("Shrink");
    assert!(!results.is_empty(), "Shrink package should still be indexed");
    Ok(())
}

#[test]
fn cross_file_function_call_references() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let lib_uri = file_url("/lib/Math.pm")?;
    let script_uri = file_url("/scripts/calc.pl")?;

    index.index_file(lib_uri, "package Math;\nsub add { return $_[0] + $_[1]; }".to_string())?;

    // Call both qualified and unqualified
    index.index_file(script_uri, "use Math;\nMath::add(1, 2);\nadd(3, 4);".to_string())?;

    // find_references for the qualified name should find both calls
    let refs = index.find_references("Math::add");
    // Should find at minimum the two calls (qualified + bare)
    assert!(refs.len() >= 2, "expected at least 2 references for Math::add, got {}", refs.len());
    Ok(())
}
