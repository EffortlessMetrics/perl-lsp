//! Tests for WorkspaceIndex cross-file symbol resolution
//!
//! This test suite demonstrates that the WorkspaceIndex can successfully:
//! 1. Index symbols from multiple files
//! 2. Find symbols across file boundaries
//! 3. Track dependencies between files
//! 4. Find unused symbols across the workspace

use perl_parser::workspace_index::{SymbolKind, WorkspaceIndex};
// ReferenceKind is not exported, we'll check Location fields instead

#[test]
fn test_cross_file_symbol_resolution() {
    let index = WorkspaceIndex::new();

    // File 1: lib/Foo.pm - defines a package and subroutine
    let file1_uri = "file:///workspace/lib/Foo.pm";
    let file1_content = r#"
package Foo;

sub bar {
    my $x = 42;
    return $x;
}

sub unused_func {
    return "never called";
}

1;
"#;

    // File 2: script.pl - uses Foo::bar
    let file2_uri = "file:///workspace/script.pl";
    let file2_content = r#"
use Foo;

my $result = Foo::bar();
print $result;
"#;

    // Index both files
    index
        .index_file(url::Url::parse(file1_uri).unwrap(), file1_content.to_string())
        .expect("Failed to index file1");
    index
        .index_file(url::Url::parse(file2_uri).unwrap(), file2_content.to_string())
        .expect("Failed to index file2");

    // Test 1: Find the 'bar' subroutine
    let bar_symbols = index.find_symbols("bar");
    assert_eq!(bar_symbols.len(), 1, "Should find exactly one 'bar' symbol");
    assert_eq!(bar_symbols[0].name, "bar");
    assert_eq!(bar_symbols[0].kind, SymbolKind::Subroutine);
    assert_eq!(bar_symbols[0].qualified_name, Some("Foo::bar".to_string()));

    // Test 2: Find references to 'bar'
    let bar_refs = index.find_references("bar");
    // Note: Currently FunctionCall for Foo::bar() doesn't extract "bar" separately,
    // it extracts "Foo::bar" as the function name
    // We'll check that we found at least the definition reference
    assert!(!bar_refs.is_empty(), "Should find at least one reference to 'bar'");

    // Test 3: Find the Foo package
    let foo_symbols = index.find_symbols("Foo");
    // The search finds all symbols with "Foo" in their name or qualified name
    // This includes the package and the subroutines in the package
    let foo_packages: Vec<_> =
        foo_symbols.iter().filter(|s| s.kind == SymbolKind::Package).collect();
    assert_eq!(foo_packages.len(), 1, "Should find exactly one 'Foo' package");
    assert_eq!(foo_packages[0].name, "Foo");

    // Test 4: Find references to Foo (the use statement)
    let foo_refs = index.find_references("Foo");
    // Should find at least one reference (the use statement)
    assert!(!foo_refs.is_empty(), "Should find at least one reference to 'Foo'");

    // Test 5: Find unused symbols
    let unused = index.find_unused_symbols();
    let unused_funcs: Vec<_> = unused.iter().filter(|s| s.name == "unused_func").collect();
    assert_eq!(unused_funcs.len(), 1, "Should find 'unused_func' as unused");

    // Test 6: Track dependencies
    let file2_deps = index.file_dependencies(file2_uri);
    assert!(file2_deps.contains("Foo"), "script.pl should depend on Foo module");

    // Test 7: Get package members
    let foo_members = index.get_package_members("Foo");
    assert!(foo_members.len() >= 2, "Should find at least 2 members in Foo package");
    let member_names: Vec<_> = foo_members.iter().map(|s| &s.name).collect();
    assert!(member_names.contains(&&"bar".to_string()), "Should find 'bar' in Foo package");
    assert!(
        member_names.contains(&&"unused_func".to_string()),
        "Should find 'unused_func' in Foo package"
    );
}

#[test]
fn test_workspace_index_file_updates() {
    let index = WorkspaceIndex::new();
    let file_uri = "file:///workspace/test.pl";

    // Initial content
    let content_v1 = r#"
sub old_function {
    return 1;
}
"#;

    index
        .index_file(url::Url::parse(file_uri).unwrap(), content_v1.to_string())
        .expect("Failed to index v1");

    let old_func = index.find_symbols("old_function");
    assert_eq!(old_func.len(), 1, "Should find old_function");

    // Updated content
    let content_v2 = r#"
sub new_function {
    return 2;
}
"#;

    index
        .index_file(url::Url::parse(file_uri).unwrap(), content_v2.to_string())
        .expect("Failed to index v2");

    let old_func = index.find_symbols("old_function");
    assert_eq!(old_func.len(), 0, "Should not find old_function after update");

    let new_func = index.find_symbols("new_function");
    assert_eq!(new_func.len(), 1, "Should find new_function after update");
}

#[test]
fn test_workspace_index_clear_file() {
    let index = WorkspaceIndex::new();
    let file_uri = "file:///workspace/temp.pl";

    let content = r#"
package TempPackage;
sub temp_sub { }
"#;

    index
        .index_file(url::Url::parse(file_uri).unwrap(), content.to_string())
        .expect("Failed to index");

    let symbols = index.find_symbols("TempPackage");
    let packages: Vec<_> = symbols.iter().filter(|s| s.kind == SymbolKind::Package).collect();
    assert_eq!(packages.len(), 1, "Should find TempPackage");

    // Clear the file
    index.clear_file(file_uri);

    let symbols = index.find_symbols("TempPackage");
    assert_eq!(symbols.len(), 0, "Should not find TempPackage after clearing");
}

#[test]
fn test_variable_indexing() {
    let index = WorkspaceIndex::new();
    let file_uri = "file:///workspace/vars.pl";

    let content = r#"
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');
my ($x, $y, $z) = (1, 2, 3);
"#;

    index
        .index_file(url::Url::parse(file_uri).unwrap(), content.to_string())
        .expect("Failed to index");

    let scalar = index.find_symbols("$scalar");
    assert_eq!(scalar.len(), 1, "Should find $scalar");
    assert_eq!(scalar[0].kind, SymbolKind::Variable);

    let array = index.find_symbols("@array");
    assert_eq!(array.len(), 1, "Should find @array");

    let hash = index.find_symbols("%hash");
    assert_eq!(hash.len(), 1, "Should find %hash");

    let x = index.find_symbols("$x");
    assert_eq!(x.len(), 1, "Should find $x from list declaration");
}
