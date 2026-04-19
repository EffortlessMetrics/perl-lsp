//! Integration tests for PL200, PL201, and PL300 diagnostic codes
//!
//! # Codes tested
//!
//! | Code  | Name                      | Status       |
//! |-------|---------------------------|--------------|
//! | PL200 | MissingPackageDeclaration | Implemented  |
//! | PL201 | DuplicatePackage          | Implemented  |
//! | PL300 | DuplicateSubroutine       | Implemented  |
//! | PL301 | MissingReturn             | Deferred     |
//! | PL402 | ImplicitReturn            | Deferred     |
//!
//! Tests FAIL before `package_subroutine.rs` is created and wired.
//! Tests PASS after the implementation is complete.
//!
//! See: crates/perl-lsp-diagnostics/src/lints/package_subroutine.rs

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn codes_for(source: &str) -> Vec<String> {
    diagnostics_for(source)
        .into_iter()
        .filter_map(|d| d.code)
        .collect()
}

fn has_code(source: &str, code: &str) -> bool {
    codes_for(source).iter().any(|c| c == code)
}

fn count_code(source: &str, code: &str) -> usize {
    codes_for(source)
        .iter()
        .filter(|c| c.as_str() == code)
        .count()
}

// =========================================================================
// PL200 — MissingPackageDeclaration
// =========================================================================

/// Test 1: PL200 fires when no package declaration is present
#[test]
fn pl200_fires_when_no_package_declaration() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    assert!(
        has_code(source, "PL200"),
        "Expected PL200 (MissingPackageDeclaration) when file has no package declaration. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 2: PL200 is suppressed when a package declaration is present
#[test]
fn pl200_suppressed_when_package_present() {
    let source = "package Foo;\nuse strict;\n";
    assert!(
        !has_code(source, "PL200"),
        "PL200 should NOT fire when file has a package declaration. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 3: PL200 is suppressed for block-form package declaration
#[test]
fn pl200_suppressed_for_block_form_package() {
    let source = "package Foo {\n    use strict;\n    my $x = 1;\n}\n";
    assert!(
        !has_code(source, "PL200"),
        "PL200 should NOT fire when file uses block-form package declaration. \
         Got codes: {:?}",
        codes_for(source)
    );
}

// =========================================================================
// PL201 — DuplicatePackage
// =========================================================================

/// Test 4: PL201 fires on duplicate package declaration (second occurrence)
#[test]
fn pl201_fires_on_duplicate_package() {
    let source = "package Foo;\nmy $x = 1;\npackage Foo;\nmy $y = 2;\n";
    let count = count_code(source, "PL201");
    assert_eq!(
        count,
        1,
        "Expected exactly 1 PL201 diagnostic (second occurrence of package Foo). \
         Got {} PL201 diagnostics. All codes: {:?}",
        count,
        codes_for(source)
    );
}

/// Test 5: PL201 fires twice when a package is declared three times (2nd and 3rd duplicates)
#[test]
fn pl201_fires_twice_on_triple_duplicate_package() {
    let source = "package Foo;\npackage Bar;\npackage Foo;\npackage Foo;\n";
    let count = count_code(source, "PL201");
    assert_eq!(
        count,
        2,
        "Expected exactly 2 PL201 diagnostics (3rd and 4th occurrences of package Foo). \
         Got {} PL201 diagnostics. All codes: {:?}",
        count,
        codes_for(source)
    );
}

/// Test 6: PL201 absent when all package names are different
#[test]
fn pl201_absent_when_packages_are_different() {
    let source = "package Foo;\npackage Bar;\n";
    assert!(
        !has_code(source, "PL201"),
        "PL201 should NOT fire when all package names are unique. \
         Got codes: {:?}",
        codes_for(source)
    );
}

// =========================================================================
// PL300 — DuplicateSubroutine
// =========================================================================

/// Test 7: PL300 fires on duplicate named subroutine
#[test]
fn pl300_fires_on_duplicate_named_sub() {
    let source = "package Foo;\nsub bar { return 1; }\nsub bar { return 2; }\n";
    assert!(
        has_code(source, "PL300"),
        "Expected PL300 (DuplicateSubroutine) when a named sub is defined twice. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 8: PL300 absent for anonymous subroutines (they have no name to deduplicate)
#[test]
fn pl300_absent_for_anonymous_subs() {
    let source = "package Foo;\nmy $f = sub { 1 };\nmy $g = sub { 2 };\n";
    assert!(
        !has_code(source, "PL300"),
        "PL300 should NOT fire for anonymous subroutines. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 9: PL300 fires inside package block form (requires walker Package arm fix)
///
/// This test FAILS before the walker.rs fix is applied (Package block is not traversed),
/// and PASSES after adding the `NodeKind::Package { block, .. }` arm to `walk_node`.
#[test]
fn pl300_fires_inside_package_block_form() {
    let source = "package Foo {\n    sub bar { 1 }\n    sub bar { 2 }\n}\n";
    assert!(
        has_code(source, "PL300"),
        "Expected PL300 (DuplicateSubroutine) for duplicate subs inside package block form. \
         This test requires the walker.rs Package block arm to be present. \
         Got codes: {:?}",
        codes_for(source)
    );
}

/// Test 10: PL300 absent when subroutine names are different
#[test]
fn pl300_absent_when_sub_names_are_different() {
    let source = "package Foo;\nsub bar { 1 }\nsub baz { 2 }\n";
    assert!(
        !has_code(source, "PL300"),
        "PL300 should NOT fire when subroutine names are all unique. \
         Got codes: {:?}",
        codes_for(source)
    );
}

// =========================================================================
// Edge case: multiple packages each defining sub new — must NOT fire PL300
// =========================================================================

/// Test 11: PL300 absent when same sub name appears in different packages (package-statement form)
///
/// Two packages each declaring `sub new` is common Perl OO style.
/// PL300 must NOT fire — the names are in different namespaces.
#[test]
fn pl300_absent_for_same_sub_name_in_different_packages() {
    let source =
        "package Foo;\nsub new { bless {}, shift }\n\npackage Bar;\nsub new { bless {}, shift }\n";
    assert!(
        !has_code(source, "PL300"),
        "PL300 must NOT fire when the same sub name appears in separate packages. \
         Each package has its own namespace. \
         Got codes: {:?}",
        codes_for(source)
    );
}
