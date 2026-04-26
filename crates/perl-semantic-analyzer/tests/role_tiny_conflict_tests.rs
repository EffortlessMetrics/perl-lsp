//! Tests for Role::Tiny role conflict detection (PL303)
//!
//! Verifies that the role conflict detection correctly identifies method
//! conflicts when a class consumes multiple Role::Tiny roles that provide
//! the same method name.
//!
//! These tests are RED tests — they define what correct behavior looks like
//! and should FAIL before the Role::Tiny support is implemented, and PASS after.

use perl_lsp_rs_core::providers::diagnostics::Diagnostic;
use perl_lsp_rs_core::providers::diagnostics::role_conflicts::check_role_conflicts;
use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::symbol::SymbolExtractor;
use perl_tdd_support::{must, must_some};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn has_pl303(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.code.as_deref() == Some("PL303"))
}

fn extract_diagnostics(code: &str) -> Vec<Diagnostic> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let symbol_table = SymbolExtractor::new_with_source(code).extract(&ast);
    let mut diagnostics = Vec::new();
    check_role_conflicts(&ast, &symbol_table, &mut diagnostics);
    diagnostics
}

// ===========================================================================
// Test 1: Basic two-role conflict
// ===========================================================================
// When two Role::Tiny roles in the same file provide the same method,
// and a class consumes both via `with()`, a PL303 diagnostic should be emitted.
//
// Perl code pattern:
//   package RoleA;
//   use Role::Tiny;
//   sub helper { ... }
//
//   package RoleB;
//   use Role::Tiny;
//   sub helper { ... }
//
//   package MyClass;
//   use Role::Tiny::With;
//   with 'RoleA', 'RoleB';
//
// Expected: PL303 diagnostic on the `with()` call
// ===========================================================================

#[test]
fn test_role_tiny_two_role_conflict_emits_pl303() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package RoleA;
use Role::Tiny;

sub helper {
    return "RoleA";
}

package RoleB;
use Role::Tiny;

sub helper {
    return "RoleB";
}

package MyClass;
use Role::Tiny::With;

with 'RoleA', 'RoleB';

1;
"#;

    let diagnostics = extract_diagnostics(code);

    assert!(
        has_pl303(&diagnostics),
        "T-role-tiny-conflict-1: Expected PL303 diagnostic for conflicting `helper` method \
         from RoleA and RoleB, but got diagnostics: {diagnostics:?}"
    );

    Ok(())
}

// ===========================================================================
// Test 2: Three-way conflict detection
// ===========================================================================
// When three Role::Tiny roles provide the same method and are consumed together,
// PL303 should be emitted.
//
// Perl code pattern:
//   package RoleA, RoleB, RoleC;
//   each provides sub duplicate_method { }
//
//   package MyClass;
//   with 'RoleA', 'RoleB', 'RoleC';
//
// Expected: PL303 diagnostic
// ===========================================================================

#[test]
fn test_role_tiny_three_way_conflict_emits_pl303() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package RoleA;
use Role::Tiny;

sub duplicate_method {
    return "A";
}

package RoleB;
use Role::Tiny;

sub duplicate_method {
    return "B";
}

package RoleC;
use Role::Tiny;

sub duplicate_method {
    return "C";
}

package MyClass;
use Role::Tiny::With;

with 'RoleA', 'RoleB', 'RoleC';

1;
"#;

    let diagnostics = extract_diagnostics(code);

    assert!(
        has_pl303(&diagnostics),
        "T-role-tiny-conflict-2: Expected PL303 diagnostic for conflicting `duplicate_method` \
         from RoleA, RoleB, and RoleC, but got diagnostics: {diagnostics:?}"
    );

    Ok(())
}

// ===========================================================================
// Test 3: Class method suppresses conflict
// ===========================================================================
// When the consuming class defines its own implementation of the conflicting
// method, no diagnostic should be emitted.
//
// Perl code pattern:
//   Same as Test 1, but MyClass also defines `sub helper { }`
//
// Expected: NO diagnostic (class implementation suppresses the conflict)
// ===========================================================================

#[test]
fn test_role_tiny_class_method_suppresses_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package RoleA;
use Role::Tiny;

sub helper {
    return "RoleA";
}

package RoleB;
use Role::Tiny;

sub helper {
    return "RoleB";
}

package MyClass;
use Role::Tiny::With;

with 'RoleA', 'RoleB';

sub helper {
    return "MyClass";
}

1;
"#;

    let diagnostics = extract_diagnostics(code);

    assert!(
        !has_pl303(&diagnostics),
        "T-role-tiny-conflict-3: Expected NO PL303 diagnostic when consuming class \
         defines its own `helper` method (should suppress conflict), but got: {diagnostics:?}"
    );

    Ok(())
}

// ===========================================================================
// Test 4: No conflict when roles provide different methods
// ===========================================================================
// When two Role::Tiny roles provide different methods, no diagnostic.
//
// Expected: NO diagnostic
// ===========================================================================

#[test]
fn test_role_tiny_no_conflict_different_methods() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package RoleA;
use Role::Tiny;

sub method_a {
    return "A";
}

package RoleB;
use Role::Tiny;

sub method_b {
    return "B";
}

package MyClass;
use Role::Tiny::With;

with 'RoleA', 'RoleB';

1;
"#;

    let diagnostics = extract_diagnostics(code);

    assert!(
        !has_pl303(&diagnostics),
        "T-role-tiny-conflict-4: Expected NO PL303 diagnostic when roles provide \
         different methods (method_a vs method_b), but got: {diagnostics:?}"
    );

    Ok(())
}

// ===========================================================================
// Test 5: Both Role::Tiny import styles work
// ===========================================================================
// Both `use Role::Tiny;` (role definition) and `use Role::Tiny::With;`
// (role consumption) should be recognized as the Role::Tiny framework.
//
// This tests that the framework detection works when the consuming class
// uses `use Role::Tiny::With;` rather than `use Role::Tiny;`.
//
// Expected: PL303 diagnostic
// ===========================================================================

#[test]
fn test_role_tiny_with_import_style_emits_pl303() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package PrintRole;
use Role::Tiny;

sub print_greeting {
    print "Hello\n";
}

package FormatRole;
use Role::Tiny;

sub print_greeting {
    print "Formatted: Hello\n";
}

package MyFormatter;
use Role::Tiny::With;

with 'PrintRole', 'FormatRole';

1;
"#;

    let diagnostics = extract_diagnostics(code);

    assert!(
        has_pl303(&diagnostics),
        "T-role-tiny-conflict-5: Expected PL303 diagnostic when using `use Role::Tiny::With;` \
         style import, but got diagnostics: {diagnostics:?}"
    );

    Ok(())
}

// ===========================================================================
// Test 6: Role::Tiny role marked as SymbolKind::Role in symbol table
// ===========================================================================
// Verify that packages using `use Role::Tiny;` are correctly marked as
// SymbolKind::Role in the symbol table (required for check_role_conflicts
// to include them in role conflict detection).
// ===========================================================================

#[test]
fn test_role_tiny_package_marked_as_role_symbol() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

    fn has_role_symbol(table: &SymbolTable, name: &str) -> bool {
        table
            .symbols
            .get(name)
            .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Role))
    }

    let code = r#"
package My::Role;
use Role::Tiny;

sub do_something { }

1;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let symbol_table = SymbolExtractor::new_with_source(code).extract(&ast);

    assert!(
        has_role_symbol(&symbol_table, "My::Role"),
        "T-role-tiny-symbol-1: Expected package 'My::Role' with `use Role::Tiny;` \
         to be marked as SymbolKind::Role in symbol table, but it was not. \
         Symbol table keys: {:?}",
        symbol_table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

// ===========================================================================
// Test 7: Framework::RoleTiny detected by ClassModelBuilder
// ===========================================================================
// Verify that the Framework enum includes RoleTiny and that the
// ClassModelBuilder correctly detects `use Role::Tiny;` imports.
// ===========================================================================

#[test]
fn test_framework_enum_has_role_tiny() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::class_model::{ClassModel, ClassModelBuilder};

    let code = r#"
package My::Role;
use Role::Tiny;

sub do_something { }

1;
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let models: Vec<ClassModel> = ClassModelBuilder::new().build(&ast);

    // Find the My::Role model
    let role_model = models.iter().find(|m| m.name == "My::Role");

    assert!(
        role_model.is_some(),
        "T-role-tiny-framework-1: Expected ClassModelBuilder to produce a ClassModel \
         for 'My::Role', but no model was found. Models produced: {models:?}"
    );

    let role_model = must_some(role_model);

    // The framework should be detected as RoleTiny (once implemented)
    // Currently this will return Framework::None, so the test will fail
    // until RoleTiny is added to the Framework enum and detect_framework
    // is updated to recognize Role::Tiny
    assert!(
        role_model.framework == perl_semantic_analyzer::class_model::Framework::RoleTiny,
        "T-role-tiny-framework-1: Expected Framework::RoleTiny for `use Role::Tiny;` package, \
         but got: {:?}. This test will pass once Role::Tiny support is implemented.",
        role_model.framework
    );

    Ok(())
}
