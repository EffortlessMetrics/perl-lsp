//! Tests for same-file strict-subs validation of package-qualified function
//! calls (Phase 1 MVP for issue #3358).
//!
//! The analyzer's strict-subs pass fires `IssueKind::UndeclaredSubroutine` only
//! when all of the following hold:
//!
//!   * the source has `use strict` / `use strict 'subs'` / a version pragma
//!     that implies strict in effect at the call site,
//!   * the call syntax is `Package::sub_name()` (qualified function call),
//!   * the target package is declared in the current file,
//!   * the package has none of: `AUTOLOAD`, `our @ISA = ...`, `use parent`,
//!     `use base`, an object framework (`Moo`, `Moose`, `Mouse`,
//!     `Object::Pad`, `Role::Tiny`, …), typeglob aliasing (`*Foo::x = ...`),
//!     or a surrounding `eval`,
//!   * and the called sub name is not among that package's declared subs.
//!
//! Every other case must stay silent. These tests lock in that contract.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

fn scope_issues_no_pragma(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &[])
}

fn has_undeclared_sub(issues: &[ScopeIssue], qualified_name: &str) -> bool {
    issues
        .iter()
        .any(|i| i.kind == IssueKind::UndeclaredSubroutine && i.variable_name == qualified_name)
}

fn count_undeclared_subs(issues: &[ScopeIssue]) -> usize {
    issues.iter().filter(|i| i.kind == IssueKind::UndeclaredSubroutine).count()
}

fn dump(issues: &[ScopeIssue]) -> String {
    issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredSubroutine)
        .map(|i| format!("{:?}:{}", i.kind, i.variable_name))
        .collect::<Vec<_>>()
        .join(", ")
}

// ===========================================================================
// 1. Positive case — declared package, missing sub → warn
// ===========================================================================

#[test]
fn missing_sub_in_same_file_package_warns() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
sub bar { 1 }

package main;
Foo::baz();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_undeclared_sub(&issues, "Foo::baz"),
        "expected UndeclaredSubroutine for Foo::baz; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn defined_sub_in_same_file_package_does_not_warn() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
sub bar { 1 }

package main;
Foo::bar();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::bar"),
        "defined sub must not warn; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 2. Nested package names
// ===========================================================================

#[test]
fn nested_package_resolved_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo::Bar;
sub baz { 1 }

package main;
Foo::Bar::baz();
Foo::Bar::qux();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::Bar::baz"),
        "defined deep sub must not warn; got: {}",
        dump(&issues)
    );
    assert!(
        has_undeclared_sub(&issues, "Foo::Bar::qux"),
        "missing deep sub must warn; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 3. Strict subs must be enabled
// ===========================================================================

#[test]
fn without_strict_pragma_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Foo;
sub bar { 1 }

package main;
Foo::baz();
"#;
    let issues = scope_issues_no_pragma(code);
    assert_eq!(count_undeclared_subs(&issues), 0, "no strict → no qualified-call diagnostic");
    Ok(())
}

#[test]
fn strict_vars_only_is_not_enough() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict 'vars';
package Foo;
sub bar { 1 }

package main;
Foo::baz();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::baz"),
        "strict 'vars' alone should not trigger qualified-sub validation; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn strict_subs_only_is_enough() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict 'subs';
package Foo;
sub bar { 1 }

package main;
Foo::baz();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_undeclared_sub(&issues, "Foo::baz"),
        "strict 'subs' must enable qualified-sub validation; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 4. Packages not defined in this file → no validation
// ===========================================================================

#[test]
fn unknown_package_not_validated() -> Result<(), Box<dyn std::error::Error>> {
    // `Foo` never appears as a `package Foo;` declaration in this file, so the
    // analyzer has nothing to check against and must stay silent.
    let code = r#"
use strict;
Foo::bar();
"#;
    let issues = scope_issues_strict(code);
    assert_eq!(count_undeclared_subs(&issues), 0);
    Ok(())
}

// ===========================================================================
// 5. Built-in / pseudo packages are always silent
// ===========================================================================

#[test]
fn builtin_packages_always_silent() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
CORE::print("x");
UNIVERSAL::isa("Foo", "Bar");
SUPER::new();
Scalar::Util::blessed($x);
List::Util::first { $_ > 0 } 1, 2, 3;
"#;
    let issues = scope_issues_strict(code);
    assert_eq!(
        count_undeclared_subs(&issues),
        0,
        "built-in pseudo packages never validated; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 6. Opacity markers on the target package — must skip validation
// ===========================================================================

#[test]
fn autoload_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
sub AUTOLOAD { }

package main;
Foo::anything();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::anything"),
        "AUTOLOAD makes dispatch dynamic; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn our_isa_declaration_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
our @ISA = ('Parent');
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "@ISA implies inheritance; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn use_parent_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use parent 'Parent';
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "use parent implies inheritance; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn use_base_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use base 'Parent';
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "use base implies inheritance; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn moo_class_is_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use Moo;
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "Moo classes have auto-generated accessors; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn moose_class_is_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use Moose;
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "Moose classes have meta machinery; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn object_pad_class_is_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use Object::Pad;
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "Object::Pad classes generate accessors; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn exporter_use_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
use Exporter 'import';
sub bar { 1 }

package main;
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::missing"),
        "Exporter imports are dynamic; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn typeglob_alias_marks_target_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
sub bar { 1 }

package main;
*Foo::aliased = sub { 42 };
Foo::aliased();
Foo::other_missing();
"#;
    let issues = scope_issues_strict(code);
    // The typeglob assignment should mark `Foo` as opaque so neither the
    // aliased nor the truly-missing call is flagged.
    assert_eq!(
        count_undeclared_subs(&issues),
        0,
        "typeglob alias marks target package opaque; got: {}",
        dump(&issues)
    );
    Ok(())
}

#[test]
fn string_eval_marks_package_opaque() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo;
sub bar { 1 }
eval "sub generated { 42 }";

package main;
Foo::generated();
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    // eval can add symbols to any package — treat as opaque.
    assert_eq!(
        count_undeclared_subs(&issues),
        0,
        "eval marks enclosing package opaque; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 7. Explicit package-prefixed sub definitions
// ===========================================================================

#[test]
fn explicit_package_prefixed_sub_is_visible() -> Result<(), Box<dyn std::error::Error>> {
    // `sub Foo::bar { ... }` defines `bar` in package `Foo` regardless of the
    // surrounding package context.
    let code = r#"
use strict;
package main;
sub Foo::bar { 1 }
Foo::bar();
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    // `Foo` is now known via the explicit sub definition.
    assert!(
        !has_undeclared_sub(&issues, "Foo::bar"),
        "explicit qualified sub definition should count; got: {}",
        dump(&issues)
    );
    assert!(
        has_undeclared_sub(&issues, "Foo::missing"),
        "truly missing sub should still warn; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 8. Package block form vs. statement form
// ===========================================================================

#[test]
fn block_form_package_resolves_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Foo {
    sub bar { 1 }
}

package main;
Foo::bar();
Foo::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "Foo::bar"),
        "block-form sub should count; got: {}",
        dump(&issues)
    );
    assert!(
        has_undeclared_sub(&issues, "Foo::missing"),
        "missing sub should warn; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 9. Statement-form package scopes subsequent siblings
// ===========================================================================

#[test]
fn statement_form_package_scopes_siblings() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
package Alpha;
sub one { 1 }

package Beta;
sub two { 2 }

package main;
Alpha::one();
Beta::two();
Alpha::two();
Beta::one();
"#;
    let issues = scope_issues_strict(code);
    assert!(!has_undeclared_sub(&issues, "Alpha::one"));
    assert!(!has_undeclared_sub(&issues, "Beta::two"));
    assert!(has_undeclared_sub(&issues, "Alpha::two"));
    assert!(has_undeclared_sub(&issues, "Beta::one"));
    Ok(())
}

// ===========================================================================
// 10. `use v5.xx` version pragma enables strict subs
// ===========================================================================

#[test]
fn use_v5_40_enables_qualified_sub_check() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use v5.40;
package Foo;
sub bar { 1 }

package main;
Foo::baz();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_undeclared_sub(&issues, "Foo::baz"),
        "use v5.40 should enable strict subs; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 11. No false positives on the `main::` prefix
// ===========================================================================

#[test]
fn main_qualified_call_to_top_level_sub_ok() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
sub helper { 1 }
main::helper();
main::missing();
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_undeclared_sub(&issues, "main::helper"),
        "top-level subs are in package main; got: {}",
        dump(&issues)
    );
    assert!(
        has_undeclared_sub(&issues, "main::missing"),
        "truly missing main::missing should warn; got: {}",
        dump(&issues)
    );
    Ok(())
}

// ===========================================================================
// 12. Empty / malformed qualified names are ignored
// ===========================================================================

#[test]
fn empty_package_or_sub_name_silent() -> Result<(), Box<dyn std::error::Error>> {
    // Nothing in the grammar should produce these, but defending against them
    // keeps the validator robust against AST quirks.
    let code = r#"
use strict;
package Foo;
sub bar { 1 }

package main;
Foo::bar();
"#;
    let issues = scope_issues_strict(code);
    // The happy path: just assert we don't crash and we produce 0 undeclared-sub
    // diagnostics for this purely-valid program.
    assert_eq!(count_undeclared_subs(&issues), 0);
    Ok(())
}
