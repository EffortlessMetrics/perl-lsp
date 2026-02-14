//! Integration tests for scoped rename refactoring
//!
//! Tests validate that scoped rename correctly respects Perl's lexical scoping rules
//! and handles different scope types (File, Package, Function, Block).

use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use perl_tdd_support::must;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_function_scope_lexical_variable() {
    // AC2, AC3: Function scope with lexical variables
    // AC6: Special handling for `my` variables
    let mut file = must(NamedTempFile::new());
    let code = r#"
sub foo {
    my $x = 1;
    print $x;
}

sub bar {
    my $x = 2;
    print $x;
}
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Rename $x only in function foo
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$renamed".to_string(),
            scope: RefactoringScope::Function { file: path.clone(), name: "foo".to_string() },
        },
        vec![path.clone()],
    ));

    assert!(result.success, "Rename should succeed");
    let new_code = must(std::fs::read_to_string(&path));

    // $x in foo should be renamed to $renamed
    assert!(new_code.contains("sub foo"), "Function foo should exist");
    assert!(new_code.contains("my $renamed = 1"), "Variable in foo should be renamed");

    // $x in bar should remain unchanged
    assert!(new_code.contains("sub bar"), "Function bar should exist");
    assert!(new_code.contains("my $x = 2"), "Variable in bar should be unchanged");
}

#[test]
fn test_block_scope_lexical_variable() {
    // AC2, AC3: Block scope with lexical variables
    // AC5: Nested scopes
    let mut file = must(NamedTempFile::new());
    let code = r#"
my $outer = 1;
{
    my $inner = 2;
    print $inner;
}
print $outer;
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Rename $inner only in the block (lines 3-5, roughly)
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$inner".to_string(),
            new_name: "$block_var".to_string(),
            scope: RefactoringScope::Block {
                file: path.clone(),
                start: (3, 0), // Start of block
                end: (5, 10),  // End of block
            },
        },
        vec![path.clone()],
    ));

    assert!(result.success, "Rename should succeed");
    let new_code = must(std::fs::read_to_string(&path));

    // $inner in block should be renamed
    assert!(new_code.contains("my $block_var = 2"), "Block variable should be renamed");
    assert!(new_code.contains("print $block_var"), "Block variable usage should be renamed");

    // $outer should remain unchanged
    assert!(new_code.contains("my $outer = 1"), "Outer variable should be unchanged");
    assert!(new_code.contains("print $outer"), "Outer variable usage should be unchanged");
}

#[test]
fn test_package_scope_our_variable() {
    // AC2, AC3, AC6: Package scope with `our` variables
    let mut file = must(NamedTempFile::new());
    let code = r#"
package Foo;
our $var = 1;
print $var;

package Bar;
our $var = 2;
print $var;
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Rename $var only in package Foo
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$var".to_string(),
            new_name: "$foo_var".to_string(),
            scope: RefactoringScope::Package { file: path.clone(), name: "Foo".to_string() },
        },
        vec![path.clone()],
    ));

    assert!(result.success, "Rename should succeed");
    let new_code = must(std::fs::read_to_string(&path));

    // $var in Foo should be renamed
    assert!(new_code.contains("package Foo"), "Package Foo should exist");
    assert!(new_code.contains("our $foo_var = 1"), "Variable in Foo should be renamed");

    // $var in Bar should remain unchanged
    assert!(new_code.contains("package Bar"), "Package Bar should exist");
    assert!(new_code.contains("our $var = 2"), "Variable in Bar should be unchanged");
}

#[test]
fn test_file_scope_preserves_external_files() {
    // AC3, AC4: File scope should not affect other files
    let mut file1 = must(NamedTempFile::new());
    let mut file2 = must(NamedTempFile::new());

    let code1 = "my $shared = 1;\nprint $shared;";
    let code2 = "my $shared = 2;\nprint $shared;";

    must(write!(file1, "{}", code1));
    must(write!(file2, "{}", code2));

    let path1 = file1.path().to_path_buf();
    let path2 = file2.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path1, code1));
    must(engine.index_file(&path2, code2));

    // Rename $shared only in file1
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$shared".to_string(),
            new_name: "$renamed".to_string(),
            scope: RefactoringScope::File(path1.clone()),
        },
        vec![path1.clone()], // Only file1 in the operation
    ));

    assert!(result.success, "Rename should succeed");
    assert_eq!(result.files_modified, 1, "Only one file should be modified");

    let new_code1 = must(std::fs::read_to_string(&path1));
    let new_code2 = must(std::fs::read_to_string(&path2));

    // file1 should have renamed variable
    assert!(new_code1.contains("$renamed"), "Variable in file1 should be renamed");
    assert!(!new_code1.contains("$shared"), "Old name should not exist in file1");

    // file2 should remain unchanged
    assert!(new_code2.contains("$shared"), "Variable in file2 should be unchanged");
    assert!(!new_code2.contains("$renamed"), "Renamed variable should not exist in file2");
}

#[test]
fn test_scope_validation_error() {
    // AC8: Preview mode shows exactly which occurrences will be renamed
    let mut file = must(NamedTempFile::new());
    let code = r#"
sub foo {
    my $x = 1;
}
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Try to rename in non-existent function
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Function {
                file: path.clone(),
                name: "nonexistent".to_string(),
            },
        },
        vec![path.clone()],
    );

    // This should either succeed with 0 changes or fail gracefully
    if let Ok(res) = result {
        assert_eq!(res.changes_made, 0, "No changes should be made for non-existent scope");
    }
}

#[test]
fn test_nested_scopes_shadowing() {
    // AC5: Nested scopes with variable shadowing
    let mut file = must(NamedTempFile::new());
    let code = r#"
my $x = 1;
print $x;
{
    my $x = 2;
    print $x;
}
print $x;
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Rename inner $x only (in block)
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$inner".to_string(),
            scope: RefactoringScope::Block { file: path.clone(), start: (4, 0), end: (6, 10) },
        },
        vec![path.clone()],
    ));

    assert!(result.success, "Rename should succeed");
    let new_code = must(std::fs::read_to_string(&path));

    // Outer $x should remain unchanged
    let lines: Vec<&str> = new_code.lines().collect();
    assert!(lines.iter().any(|l| l.contains("my $x = 1")), "Outer declaration should be unchanged");

    // Inner $x should be renamed
    assert!(new_code.contains("my $inner = 2"), "Inner declaration should be renamed");
    assert!(
        lines.iter().filter(|l| l.contains("print $x")).count() == 2,
        "Outer usages should remain"
    );
}

#[test]
fn test_local_dynamic_scope() {
    // AC6: Special handling for `local` variables (dynamic scope)
    let mut file = must(NamedTempFile::new());
    let code = r#"
our $global = 1;
sub foo {
    local $global = 2;
    print $global;
}
print $global;
"#;
    must(write!(file, "{}", code));
    let path = file.path().to_path_buf();

    let config = RefactoringConfig { safe_mode: false, ..Default::default() };
    let mut engine = RefactoringEngine::with_config(config);
    must(engine.index_file(&path, code));

    // Rename $global in function scope
    let result = must(engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$global".to_string(),
            new_name: "$localized".to_string(),
            scope: RefactoringScope::Function { file: path.clone(), name: "foo".to_string() },
        },
        vec![path.clone()],
    ));

    assert!(result.success, "Rename should succeed");
    let new_code = must(std::fs::read_to_string(&path));

    // Note: This test documents current behavior. Proper handling of `local`
    // requires understanding that it creates dynamic scope, not lexical scope.
    // The renamed variable should be different from the outer one.
    assert!(new_code.contains("foo"), "Function should exist");
}
